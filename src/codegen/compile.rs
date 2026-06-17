// Submodule: compile
//
// The main compilation entry point: handles LLVM module setup, function
// declaration passes, and top-level compilation orchestration.

use super::CodeGen;
use crate::ast::*;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use std::collections::HashMap;

impl<'ctx> CodeGen<'ctx> {
    pub fn compile(&mut self, program: &Program) -> Result<(), String> {
        self.define_runtime()?;
        self.detach_builder()?;

        // Pass 0: Register type definitions and create LLVM types
        for stmt in &program.stmts {
            self.registry.register(stmt)?;
            match stmt {
                Stmt::TypeAlias {
                    name, definition, ..
                } => {
                    if let Type::Struct(fields) = definition {
                        let field_tys: Vec<BasicTypeEnum> = fields
                            .iter()
                            .map(|(_, ty)| self.ast_type_to_basic_type(ty))
                            .collect();
                        let struct_ty = self.context.struct_type(&field_tys, false);
                        self.named_structs.insert(name.clone(), struct_ty);
                    }
                }
                Stmt::Enum { name, .. } => {
                    let i64 = self.i64_ty();
                    let ptr = self.ptr_ty();
                    let enum_ty = self.context.struct_type(&[i64.into(), ptr.into()], false);
                    self.enum_types.insert(name.clone(), enum_ty);
                }
                _ => {}
            }
        }

        // Detect overloaded function names (non-extension, non-module functions)
        let mut name_counts: HashMap<String, usize> = HashMap::new();
        for stmt in &program.stmts {
            if let Stmt::Fun { name, params, .. } = stmt {
                if params.iter().all(|p| p.ty.is_some()) {
                    *name_counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }
        let overloaded_names: std::collections::HashSet<String> = name_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name)
            .collect();

        // Pass 1: Declare all user-defined functions for forward references
        for stmt in &program.stmts {
            if let Stmt::Fun {
                name,
                params,
                return_type,
                body,
                type_params,
                ..
            } = stmt
            {
                // Skip generic functions — they are monomorphized on demand
                if !type_params.is_empty() {
                    self.generic_fun_defs.insert(name.clone(), stmt.clone());
                    continue;
                }
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                    .collect();
                let all_typed = params.iter().all(|p| p.ty.is_some());
                let mangled = if all_typed && overloaded_names.contains(name.as_str()) {
                    Self::mangle_name(name, &param_types)
                } else {
                    name.clone()
                };

                // Record overload info for call dispatch
                if all_typed && overloaded_names.contains(name.as_str()) {
                    self.overloaded_functions
                        .entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push((param_types.clone(), mangled.clone()));
                }

                let param_llvm_tys: Vec<BasicMetadataTypeEnum> = params
                    .iter()
                    .map(|p| self.ast_type_to_llvm(p.ty.as_ref()))
                    .collect();
                let ret_type = if name == "main" {
                    Some(Type::Named("Int".into()))
                } else {
                    return_type.as_ref().cloned().or_else(|| {
                        if all_typed {
                            self.infer_return_type(body)
                        } else {
                            None
                        }
                    })
                };
                let fn_type = self.build_fn_type(ret_type.as_ref(), &mangled, &param_llvm_tys);
                self.module.add_function(&mangled, fn_type, None);
                if name != "main" {
                    if let Some(rt) = ret_type {
                        self.fun_return_types.insert(mangled, rt);
                    }
                }
            }
            if let Stmt::Module {
                name: mod_name,
                body,
                ..
            } = stmt
            {
                let prefix = format!("{}_", mod_name);
                for inner_stmt in body {
                    if let Stmt::Fun {
                        name: fn_name,
                        params,
                        return_type,
                        body: fn_body,
                        type_params,
                        ..
                    } = inner_stmt
                    {
                        if !type_params.is_empty() {
                            continue;
                        }
                        let mangled = format!("{}{}", prefix, fn_name);
                        let param_llvm_tys: Vec<BasicMetadataTypeEnum> = params
                            .iter()
                            .map(|p| self.ast_type_to_llvm(p.ty.as_ref()))
                            .collect();
                        let ret_type = return_type.as_ref().cloned().or_else(|| {
                            if params.iter().all(|p| p.ty.is_some()) {
                                self.infer_return_type(fn_body)
                            } else {
                                None
                            }
                        });
                        let fn_type =
                            self.build_fn_type(ret_type.as_ref(), &mangled, &param_llvm_tys);
                        self.module.add_function(&mangled, fn_type, None);
                        if let Some(rt) = ret_type {
                            self.fun_return_types.insert(mangled, rt);
                        }
                    }
                }
            }
            if let Stmt::Extension {
                type_name, methods, ..
            } = stmt
            {
                for m in methods {
                    if let Stmt::Fun {
                        name,
                        params,
                        return_type,
                        body,
                        ..
                    } = m
                    {
                        let fn_name = format!("{}_{}", type_name, name);
                        self.extension_methods
                            .insert(format!("{}.{}", type_name, name), fn_name.clone());
                        let param_llvm_tys: Vec<BasicMetadataTypeEnum> = params
                            .iter()
                            .map(|p| self.ast_type_to_llvm(p.ty.as_ref()))
                            .collect();
                        let ret_type = return_type.as_ref().cloned().or_else(|| {
                            if params.iter().all(|p| p.ty.is_some()) {
                                self.infer_return_type(body)
                            } else {
                                None
                            }
                        });
                        let fn_type =
                            self.build_fn_type(ret_type.as_ref(), &fn_name, &param_llvm_tys);
                        self.module.add_function(&fn_name, fn_type, None);
                        if let Some(rt) = ret_type {
                            self.fun_return_types.insert(fn_name, rt);
                        }
                    }
                }
            }
        }

        // Pass 2: Compile function bodies and let/val/expr statements
        let mut has_main = false;

        // Check for main function
        for stmt in &program.stmts {
            if let Stmt::Fun { name, .. } = stmt {
                if name == "main" {
                    has_main = true;
                }
            }
        }

        // If no explicit main, create one first so that top-level
        // Let/Val/Expr statements compile into the correct function.
        if !has_main {
            let main_fn = self.i64_ty().fn_type(&[], false);
            let main_func = self.module.add_function("main", main_fn, None);
            let entry = self.context.append_basic_block(main_func, "entry");
            self.builder.position_at_end(entry);

            for stmt in &program.stmts {
                match stmt {
                    Stmt::Fun { type_params, .. } if !type_params.is_empty() => {
                        // Skip generic functions — they are monomorphized at call sites
                    }
                    Stmt::Fun { .. } | Stmt::Extension { .. } => {
                        // Compile function bodies into their own LLVM functions
                        self.compile_stmt(stmt)?;
                    }
                    Stmt::TypeAlias { .. } | Stmt::Enum { .. } => {
                        // Skip pure type-level declarations
                    }
                    _ => {
                        self.compile_stmt(stmt)?;
                    }
                }
            }
            if let Some(fflush_fn) = self.module.get_function("fflush") {
                let _ =
                    self.builder
                        .build_call(fflush_fn, &[self.ptr_ty().const_null().into()], "");
            }
            let _ = self
                .builder
                .build_return(Some(&self.i64_ty().const_int(0, false)));
        } else {
            for stmt in &program.stmts {
                match stmt {
                    Stmt::Fun { type_params, .. } if !type_params.is_empty() => {
                        // Skip generic functions — monomorphized at call sites
                    }
                    _ => {
                        self.compile_stmt(stmt)?;
                    }
                }
            }
        }

        self.finalize_codegen_anchor()?;

        Ok(())
    }
}
