// Submodule: generics
//
// a new named function is generated with type vars substituted.

use action_frontend::ast::*;
use action_frontend::hir::HirStmt;
use std::collections::HashMap;

use super::llvm_err;
use super::CodeGen;
use super::TypedValue;

use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::BasicMetadataValueEnum;

impl<'ctx> CodeGen<'ctx> {
    /// Build type_map for monomorphization: walk a parameter type and collect
    /// TypeVar → concrete type bindings from the runtime argument type name.
    pub(super) fn collect_type_args(
        &self,
        param_ty: &Type,
        arg_type_name: &str,
        type_map: &mut HashMap<String, Type>,
    ) {
        match param_ty {
            Type::TypeVar(name) => {
                type_map
                    .entry(name.clone())
                    .or_insert_with(|| Type::Named(arg_type_name.to_string()));
            }
            Type::Generic(base, _params) => {
                // For List[T] etc., recursively collect from base if needed
                self.collect_type_args(base, arg_type_name, type_map);
            }
            _ => {}
        }
    }

    /// Generic call with unified CallArg path (HIR-native; no AST round-trip).
    pub(super) fn compile_generic_call_from_call_args(
        &mut self,
        stmt: &HirStmt,
        name: &str,
        args: &[super::call_arg::CallArg<'_>],
        trailing: Option<super::call_arg::CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let HirStmt::Fun {
            params,
            type_params,
            ..
        } = stmt
        else {
            return Err("Expected Fun statement".to_string());
        };

        let arg_vals: Vec<TypedValue<'ctx>> = args
            .iter()
            .map(|a| self.compile_call_arg(*a))
            .collect::<Result<_, _>>()?;

        let mut type_map: HashMap<String, Type> = HashMap::new();
        for (i, param) in params.iter().enumerate() {
            if i >= args.len() {
                break;
            }
            let param_ty = param.ty.clone().unwrap_or(Type::Named("Int".into()));
            let arg_type_name = self.typed_value_type_name(&arg_vals[i]);
            self.collect_type_args(&param_ty, &arg_type_name, &mut type_map);
        }

        for tp in type_params {
            type_map
                .entry(tp.clone())
                .or_insert_with(|| Type::Named("Int".into()));
        }

        let type_suffix: Vec<String> = type_params
            .iter()
            .map(|tp| {
                let resolved = type_map
                    .get(tp)
                    .ok_or_else(|| format!("type parameter {} not resolved", tp))?;
                Ok(format!("{}", resolved))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mangled_name = format!("{}_{}", name, type_suffix.join("_"));

        self.compile_monomorphized_fn_hir(stmt, &mangled_name, &type_map)?;

        if self.in_fallible_region()
            && self
                .fallibility
                .symbols
                .get(name)
                .is_some_and(|s| s.is_fallible)
        {
            return self.compile_fallible_user_call(&mangled_name, args, trailing);
        }

        let fn_val = self
            .module
            .get_function(&mangled_name)
            .ok_or_else(|| format!("Monomorphized function '{}' not found", mangled_name))?;
        let fn_type = fn_val.get_type();
        let llvm_param_tys = fn_type.get_param_types();

        let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let bv = self.compile_and_load_call_arg(*a)?;
            let casted = self.coerce_arg(bv, llvm_param_tys.get(i))?;
            ca.push(casted.into());
        }
        if let Some(lam) = trailing {
            let bv = self.compile_and_load_call_arg(lam)?;
            let casted = self.coerce_arg(bv, llvm_param_tys.get(args.len()))?;
            ca.push(casted.into());
        }

        let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
        for av in &arg_vals {
            self.rc_free_intermediate(av)?;
        }
        match cc.try_as_basic_value().basic() {
            Some(bv) => self.bv_to_typed(bv),
            None => Ok(TypedValue::Unit),
        }
    }

    /// Generate a monomorphized version of a generic function with concrete types.
    /// No-op when this instantiation was already compiled (or is being compiled).
    pub(super) fn compile_monomorphized_fn_hir(
        &mut self,
        stmt: &HirStmt,
        mangled_name: &str,
        type_map: &HashMap<String, Type>,
    ) -> Result<(), String> {
        if !self
            .mono_cache
            .monomorphized_fns
            .insert(mangled_name.to_string())
        {
            return Ok(());
        }

        let HirStmt::Fun {
            name: orig_name,
            params,
            return_type,
            body,
            fn_or_fallback,
            ..
        } = stmt
        else {
            return Err("Expected Fun statement".to_string());
        };

        // Resolve type variables in params and return type
        let resolved_params: Vec<Param> = params
            .iter()
            .map(|p| Param {
                name: p.name.clone(),
                ty: p.ty.as_ref().map(|t| resolve_type_vars(t, type_map)),
            })
            .collect();

        let resolved_return = return_type
            .as_ref()
            .map(|rt| resolve_type_vars(rt, type_map));

        // Declare the LLVM function
        let param_llvm_tys: Vec<BasicMetadataTypeEnum> = resolved_params
            .iter()
            .map(|p| self.ast_type_to_llvm(p.ty.as_ref()))
            .collect::<Result<_, _>>()?;
        let is_propagating = fn_or_fallback.is_none()
            && resolved_return.is_some()
            && self
                .fallibility
                .symbols
                .get(orig_name)
                .is_some_and(|s| s.is_fallible);
        let fn_type = if is_propagating {
            self.build_fallible_fn_type(resolved_return.as_ref().unwrap(), &param_llvm_tys)?
        } else {
            self.build_fn_type(resolved_return.as_ref(), mangled_name, &param_llvm_tys)?
        };
        self.module.add_function(mangled_name, fn_type, None);
        if is_propagating {
            self.mono_cache
                .fallible_user_fns
                .insert(mangled_name.to_string());
        }
        if let Some(rt) = resolved_return.clone() {
            self.mono_cache
                .fun_return_types
                .insert(mangled_name.to_string(), rt);
        }

        // Compile the body with resolved types
        self.compile_fun_def_hir(
            mangled_name,
            orig_name,
            &resolved_params,
            resolved_return.as_ref(),
            body,
            fn_or_fallback.as_ref(),
        )?;

        Ok(())
    }
}
