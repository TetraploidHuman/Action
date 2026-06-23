//! Expression codegen (R4-3).

use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, StructType};

use super::{collect_free_vars_hir, llvm_err, CodeGen, Scope, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_lambda_hir(
        &mut self,
        params: &[String],
        body: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_lambda_impl(
            params,
            |params, bound, free| collect_free_vars_hir(body, params, bound, free),
            |this| this.compile_hir_expr(body),
        )
    }

    pub(crate) fn compile_lambda_impl(
        &mut self,
        params: &[String],
        collect_free: impl FnOnce(&[String], &mut Vec<String>, &mut Vec<String>),
        compile_body: impl FnOnce(&mut Self) -> Result<TypedValue<'ctx>, String>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.lambda_count += 1;
        let lambda_name = format!(".lambda_{}", self.lambda_count);

        // ---- Free variable analysis ----
        let mut free_vars: Vec<String> = vec![];
        let mut bound: Vec<String> = vec![];
        collect_free(params, &mut bound, &mut free_vars);
        // Only variables that exist in the parent scope can be captured.
        // Builtins (enum constructors like Some/None/Ok/Err, pi, e, etc.)
        // are resolved at compile time and must not be treated as captures.
        free_vars.retain(|name| self.scope.get(name).is_some());

        // ---- Build captures struct type if there are free vars ----
        let has_captures = !free_vars.is_empty();
        let captures_struct_ty: Option<StructType<'ctx>> = if has_captures {
            let field_tys: Vec<BasicTypeEnum> = free_vars
                .iter()
                .map(|name| {
                    self.scope
                        .get(name)
                        .map(|sv| sv.ty)
                        .unwrap_or_else(|| self.i64_ty().into())
                })
                .collect();
            let anon_ty = self
                .context
                .struct_type(&field_tys.iter().map(|&t| t).collect::<Vec<_>>(), false);
            // Cache this anonymous struct type with key = (free_vars, lambda_name)
            // so it can be referenced later if needed
            Some(anon_ty)
        } else {
            None
        };

        // ---- Build LLVM function type ----
        let i64 = self.i64_ty();
        let ptr_ty: BasicMetadataTypeEnum = self.ptr_ty().into();
        let mut param_tys: Vec<BasicMetadataTypeEnum> = Vec::new();
        // If capturing, first param is the captures struct pointer
        if has_captures {
            param_tys.push(ptr_ty);
        }
        for _ in params.iter() {
            param_tys.push(BasicMetadataTypeEnum::from(i64));
        }
        let fn_type = self.build_fn_type(None, &lambda_name, &param_tys);

        let function = self.module.add_function(&lambda_name, fn_type, None);
        let fn_ptr = function.as_global_value().as_pointer_value();
        let entry = self.context.append_basic_block(function, "entry");

        let saved_pos = self.builder.get_insert_block();
        self.builder.position_at_end(entry);

        // ---- Scope setup ----
        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::new();
        self.ht_result_scratch = None;
        let ht_scratch = self
            .builder
            .build_alloca(self.list_type, "ht_result_scratch")
            .map_err(llvm_err)?;
        self.ht_result_scratch = Some(ht_scratch);

        // Load captured values from the captures struct into local allocas
        if has_captures {
            let cst =
                captures_struct_ty.ok_or_else(|| "Closure capture type not set".to_string())?;
            if let Some(captures_ptr) = function.get_nth_param(0) {
                let ptr_val = captures_ptr.into_pointer_value();
                for (i, name) in free_vars.iter().enumerate() {
                    let gep = self
                        .builder
                        .build_struct_gep(cst, ptr_val, i as u32, &format!("cap_gep_{}", name))
                        .map_err(llvm_err)?;
                    // Use saved_scope for type info (self.scope is the new empty lambda scope)
                    let cap_ty = saved_scope
                        .get(name)
                        .map(|sv| sv.ty)
                        .unwrap_or_else(|| self.i64_ty().into());
                    let cap_kind = saved_scope
                        .get(name)
                        .map(|sv| sv.kind)
                        .unwrap_or(ValKind::Int);
                    let loaded = self
                        .builder
                        .build_load(cap_ty, gep, &format!("cap_load_{}", name))
                        .map_err(llvm_err)?;
                    let alloca = self
                        .builder
                        .build_alloca(loaded.get_type(), name)
                        .map_err(llvm_err)?;
                    self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
                    self.scope
                        .set(name.clone(), alloca, loaded.get_type(), cap_kind);
                }
            }
        }

        // Lambda parameters
        let param_offset: u32 = if has_captures { 1 } else { 0 };
        for (i, param) in params.iter().enumerate() {
            if let Some(pv) = function.get_nth_param(i as u32 + param_offset) {
                let alloca = self
                    .builder
                    .build_alloca(pv.get_type(), param)
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, pv).map_err(llvm_err)?;
                self.scope
                    .set(param.clone(), alloca, pv.get_type(), ValKind::Int);
            }
        }

        // ---- Compile body ----
        let result = compile_body(self)?;

        // ---- Build return ----
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| "No insert block")?;
        if current_block.get_terminator().is_none() {
            self.build_lambda_return(&function, &result)?;
        }

        // ---- Restore scope ----
        self.scope = saved_scope;
        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }

        // ---- Allocate captures struct at definition site ----
        if has_captures {
            let cst =
                captures_struct_ty.ok_or_else(|| "Closure capture type not set".to_string())?;
            let size_val = cst.size_of().ok_or("Failed to get captures struct size")?;
            let closure_ptr = self.malloc_rc(size_val)?;

            // Populate captures struct with captured values
            let undef = cst.get_undef();
            let mut cap_struct = undef;
            for (i, name) in free_vars.iter().enumerate() {
                let val = self.scope.get(name).ok_or_else(|| {
                    format!("Captured variable '{}' not found in parent scope", name)
                })?;
                let loaded = self
                    .builder
                    .build_load(val.ty, val.ptr, &format!("cap_val_{}", name))
                    .map_err(llvm_err)?;
                cap_struct = self
                    .builder
                    .build_insert_value(cap_struct, loaded, i as u32, &format!("cap_ins_{}", name))
                    .map_err(llvm_err)?
                    .into_struct_value();
            }
            self.builder
                .build_store(closure_ptr, cap_struct)
                .map_err(llvm_err)?;

            // RC increment for captured heap values (the closure now holds refs)
            for name in &free_vars {
                let var = self.scope.get(name).ok_or_else(|| {
                    format!("Captured variable '{}' not found in parent scope", name)
                })?;
                match var.kind {
                    ValKind::Str => {
                        let sv = self.load_string(var.ptr)?;
                        let dp = self
                            .builder
                            .build_extract_value(sv, 1, "crc_d")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_inc(dp)?;
                    }
                    ValKind::List | ValKind::Map | ValKind::Set => {
                        let lv = self.load_list(var.ptr)?;
                        let dp = self
                            .builder
                            .build_extract_value(lv, 0, "crc_d")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_inc(dp)?;
                    }
                    ValKind::Enum if var.enum_data_rc_managed => {
                        let loaded = self
                            .builder
                            .build_load(var.ty, var.ptr, "crc_enum")
                            .map_err(llvm_err)?;
                        let dp = self
                            .builder
                            .build_extract_value(loaded.into_struct_value(), 1, "crc_ed")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_inc(dp)?;
                    }
                    ValKind::Fn if var.is_closure => {
                        let cap_ptr = self
                            .builder
                            .build_load(self.ptr_ty(), var.ptr, "crc_cap")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_inc(cap_ptr)?;
                    }
                    _ => {}
                }
            }

            Ok(TypedValue::Closure {
                fn_ptr,
                actual_fn_type: fn_type,
                closure_ptr,
                closure_ty: cst,
                alloca: None,
            })
        } else {
            Ok(TypedValue::Fn(fn_ptr, fn_type))
        }
    }

    /// Build the return instruction for a lambda function.
    pub(crate) fn build_lambda_return(
        &mut self,
        function: &inkwell::values::FunctionValue<'ctx>,
        result: &TypedValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let llvm_void: bool = function.get_type().get_return_type().is_none();

        if llvm_void {
            let _ = self.builder.build_return(None);
            return Ok(());
        }
        match result {
            TypedValue::Enum(ptr, ty, ..) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "ret_enum")
                    .map_err(llvm_err)?;
                let sv = loaded.into_struct_value();
                let tag = self
                    .builder
                    .build_extract_value(sv, 0, "etag")
                    .map_err(llvm_err)?;
                let data = self
                    .builder
                    .build_extract_value(sv, 1, "edata")
                    .map_err(llvm_err)?;
                let undef_fat = self.fat_return_type.get_undef();
                let f1 = self
                    .builder
                    .build_insert_value(undef_fat, tag, 0, "ftag")
                    .map_err(llvm_err)?;
                let f2 = self
                    .builder
                    .build_insert_value(f1, data, 1, "fdata")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&f2));
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "ret_struct")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&loaded));
            }
            TypedValue::Bool(v) => {
                let extended = self
                    .builder
                    .build_int_z_extend(*v, i64, "ext")
                    .map_err(llvm_err)?;
                if function
                    .get_type()
                    .get_return_type()
                    .map_or(false, |rt| rt.is_struct_type())
                {
                    let struct_ty = function
                        .get_type()
                        .get_return_type()
                        .unwrap()
                        .into_struct_type();
                    let alloca = self
                        .builder
                        .build_alloca(struct_ty, "ret_pack")
                        .map_err(llvm_err)?;
                    let zero = struct_ty.const_zero();
                    self.builder.build_store(alloca, zero).map_err(llvm_err)?;
                    let gep0 = self
                        .builder
                        .build_struct_gep(struct_ty, alloca, 0, "ret_pack0")
                        .map_err(llvm_err)?;
                    self.builder.build_store(gep0, extended).map_err(llvm_err)?;
                    let loaded = self
                        .builder
                        .build_load(struct_ty, alloca, "ret_packed")
                        .map_err(llvm_err)?;
                    let _ = self.builder.build_return(Some(&loaded));
                } else {
                    let _ = self.builder.build_return(Some(&extended));
                }
            }
            _ => {
                if let Some(bv) = result.to_bv() {
                    let need_pack = function
                        .get_type()
                        .get_return_type()
                        .map_or(false, |rt| rt.is_struct_type());
                    if need_pack {
                        let struct_ty = function
                            .get_type()
                            .get_return_type()
                            .unwrap()
                            .into_struct_type();
                        let alloca = self
                            .builder
                            .build_alloca(struct_ty, "ret_pack")
                            .map_err(llvm_err)?;
                        let zero = struct_ty.const_zero();
                        self.builder.build_store(alloca, zero).map_err(llvm_err)?;
                        let gep0 = self
                            .builder
                            .build_struct_gep(struct_ty, alloca, 0, "ret_pack0")
                            .map_err(llvm_err)?;
                        self.builder.build_store(gep0, bv).map_err(llvm_err)?;
                        let loaded = self
                            .builder
                            .build_load(struct_ty, alloca, "ret_packed")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_return(Some(&loaded));
                    } else {
                        let _ = self.builder.build_return(Some(&bv));
                    }
                } else {
                    if let Some(ret_ty) = function.get_type().get_return_type() {
                        if ret_ty.is_struct_type() {
                            let zero = ret_ty.into_struct_type().const_zero();
                            let _ = self.builder.build_return(Some(&zero));
                        } else {
                            let _ = self.builder.build_return(None);
                        }
                    } else {
                        let _ = self.builder.build_return(None);
                    }
                }
            }
        }
        Ok(())
    }
}
