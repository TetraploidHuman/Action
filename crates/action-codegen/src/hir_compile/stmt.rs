use crate::{llvm_err, CodeGen, TypedValue};
use action_frontend::ast::*;
use action_frontend::hir::*;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_hir_let(&mut self, stmt: &HirStmt) -> Result<(), String> {
        let HirStmt::Let {
            name,
            type_ann,
            value,
            mutable,
            ..
        } = stmt
        else {
            return Err("compile_hir_let expects Let".to_string());
        };

        let raw_val = self.compile_hir_expr(value)?;
        let (ty, kind) = if let Some(ann) = type_ann {
            (
                self.ast_type_to_basic_type(ann),
                self.param_val_kind(Some(ann)),
            )
        } else {
            (raw_val.get_type_for_alloca(self), raw_val.val_kind())
        };
        let val = if let Some(Type::Nullable(inner)) = type_ann {
            if let TypedValue::Nullable(_null_ptr, null_bt) = raw_val {
                let declared_bt = self.ast_type_to_basic_type(
                    type_ann
                        .as_ref()
                        .ok_or_else(|| "Missing type annotation".to_string())?,
                );
                if null_bt == declared_bt {
                    raw_val
                } else {
                    let inner_bt = self.ast_type_to_basic_type(inner);
                    let name_hint = format!("Nullable<{}>", inner);
                    let nty = self.get_nullable_type(inner_bt, &name_hint);
                    let alloca = self
                        .builder
                        .build_alloca(nty, "null_retype")
                        .map_err(llvm_err)?;
                    let undef = nty.get_undef();
                    let with_flag = self
                        .builder
                        .build_insert_value(
                            undef,
                            self.null_flag_ty().const_int(1, false),
                            0,
                            "null_rf",
                        )
                        .map_err(llvm_err)?;
                    self.builder
                        .build_store(alloca, with_flag)
                        .map_err(llvm_err)?;
                    TypedValue::Nullable(alloca, nty.into())
                }
            } else {
                let inner_bt = self.ast_type_to_basic_type(inner);
                let name_hint = format!("Nullable<{}>", inner);
                let nty = self.get_nullable_type(inner_bt, &name_hint);
                self.wrap_in_nullable(&raw_val, nty)?
            }
        } else {
            raw_val
        };
        let alloca = self.builder.build_alloca(ty, name).map_err(llvm_err)?;
        self.store_typed_value(&val, alloca, ty)?;
        self.rc_inc_typed_value(&val)?;
        if self.block_did_rc_inc {
            self.rc_dec_typed_value(&val)?;
        }
        let fn_type = match &val {
            TypedValue::Fn(_, ft) => Some(*ft),
            TypedValue::Closure { .. } => None,
            _ => None,
        };
        let ast_type = type_ann.clone().or_else(|| {
            if matches!(kind, crate::ValKind::Enum) {
                let inferred = &value.ty;
                if matches!(inferred, Type::Named(_) | Type::Generic(_, _)) {
                    Some(inferred.clone())
                } else {
                    None
                }
            } else {
                None
            }
        });
        if *mutable {
            self.scope
                .set_mutable(name.clone(), alloca, ty, kind, fn_type);
        } else if let Some(at) = ast_type {
            self.scope
                .set_with_ast_type(name.clone(), alloca, ty, kind, fn_type, at);
        } else {
            self.scope
                .set_with_fn_type(name.clone(), alloca, ty, kind, fn_type);
        }
        if let TypedValue::Enum(_, _, inner_type, rc_managed) = &val {
            self.scope.set_enum_inner_type(name, *inner_type);
            self.scope.set_enum_data_rc_managed(name, *rc_managed);
        }
        if let TypedValue::Closure {
            fn_ptr,
            actual_fn_type,
            closure_ptr: _,
            closure_ty,
            alloca: _,
        } = &val
        {
            self.scope
                .set_closure_info(name, *closure_ty, *fn_ptr, *actual_fn_type);
        }
        Ok(())
    }

    pub(crate) fn compile_hir_external(&mut self, stmt: &HirStmt) -> Result<(), String> {
        let HirStmt::External {
            name,
            params,
            return_type,
            ..
        } = stmt
        else {
            return Err("compile_hir_external expects External".to_string());
        };
        let param_types: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = params
            .iter()
            .map(|p| {
                let bt = self.ast_type_to_basic_type(
                    p.ty.as_ref().unwrap_or(&Type::Named("Int".to_string())),
                );
                bt.into()
            })
            .collect();
        let fn_type = match return_type {
            Some(rt) => {
                let ret_bt = self.ast_type_to_basic_type(rt);
                ret_bt.fn_type(&param_types, false)
            }
            None => self.void_ty().fn_type(&param_types, false),
        };
        self.module.add_function(name, fn_type, None);
        Ok(())
    }

    pub(crate) fn compile_hir_external_type(&mut self, stmt: &HirStmt) -> Result<(), String> {
        let HirStmt::ExternalType { name, .. } = stmt else {
            return Err("compile_hir_external_type expects ExternalType".to_string());
        };
        let opaque_ty = self.context.opaque_struct_type(name);
        self.type_layout
            .named_structs
            .insert(name.clone(), opaque_ty);
        Ok(())
    }

    pub(crate) fn compile_destructure_hir(
        &mut self,
        mutable: bool,
        names: &[String],
        renames: &[(String, String)],
        rest: &Option<String>,
        is_list: bool,
        is_struct: bool,
        value: &HirExpr,
    ) -> Result<(), String> {
        let val = self.compile_hir_expr(value)?;
        if is_list {
            let list_ptr = match val {
                TypedValue::List(ptr) => ptr,
                _ => return Err("List destructuring requires a list value".to_string()),
            };
            let list_val = self.load_list(list_ptr)?;
            let data = self
                .builder
                .build_extract_value(list_val, 0, "data")
                .map_err(llvm_err)?
                .into_pointer_value();
            let len = self
                .builder
                .build_extract_value(list_val, 1, "len")
                .map_err(llvm_err)?
                .into_int_value();
            let data_str = self
                .builder
                .build_pointer_cast(data, self.ptr_ty(), "data_str")
                .map_err(llvm_err)?;
            for (i, name) in names.iter().enumerate() {
                let idx = self.i64_ty().const_int(i as u64, false);
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.string_type, data_str, &[idx], "delem_ptr")
                }
                .map_err(llvm_err)?;
                let loaded = self
                    .builder
                    .build_load(self.string_type, elem_ptr, "delem")
                    .map_err(llvm_err)?;
                let ss = loaded.into_struct_value();
                let tag = self
                    .builder
                    .build_extract_value(ss, 0, "tag")
                    .map_err(llvm_err)?
                    .into_int_value();
                let tag_ty = tag.get_type();
                let alloca = self.builder.build_alloca(tag_ty, name).map_err(llvm_err)?;
                self.builder.build_store(alloca, tag).map_err(llvm_err)?;
                if mutable {
                    self.scope.set_mutable(
                        name.clone(),
                        alloca,
                        tag_ty.into(),
                        crate::ValKind::Int,
                        None,
                    );
                } else {
                    self.scope
                        .set(name.clone(), alloca, tag_ty.into(), crate::ValKind::Int);
                }
            }
            if let Some(rest_name) = rest {
                let start_idx = names.len() as u64;
                let cap = self.i64_ty().const_int(4, false);
                let new_list_cc = self.call_rt("action_list_create", &[cap.into()])?;
                let new_list_bv = new_list_cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("rest list create fail")?;
                let rest_alloca = self
                    .builder
                    .build_alloca(self.list_type, rest_name)
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(rest_alloca, new_list_bv)
                    .map_err(llvm_err)?;
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let i64 = self.i64_ty();
                let i_a = self.builder.build_alloca(i64, "ri").map_err(llvm_err)?;
                self.builder
                    .build_store(i_a, i64.const_int(start_idx, false))
                    .map_err(llvm_err)?;
                let rest_hdr = self.context.append_basic_block(current_fn, "rest_hdr");
                let rest_bdy = self.context.append_basic_block(current_fn, "rest_bdy");
                let rest_ext = self.context.append_basic_block(current_fn, "rest_ext");
                let _ = self.builder.build_unconditional_branch(rest_hdr);
                self.builder.position_at_end(rest_hdr);
                let cur = self
                    .builder
                    .build_load(i64, i_a, "rc")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur, len, "rc")
                    .map_err(llvm_err)?;
                let _ = self
                    .builder
                    .build_conditional_branch(cond, rest_bdy, rest_ext);
                self.builder.position_at_end(rest_bdy);
                let get_cache = self.alloc_list_get_cache()?;
                let elem = self.list_get_cached_fat(list_ptr, cur, get_cache)?;
                let elem_bv = elem.into_struct_value();
                let rest_loaded = self.load_list(rest_alloca)?;
                let _ = self.call_rt("action_list_push", &[rest_loaded.into(), elem_bv.into()])?;
                let nxt = self
                    .builder
                    .build_int_add(cur, i64.const_int(1, false), "rn")
                    .map_err(llvm_err)?;
                self.builder.build_store(i_a, nxt).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(rest_hdr);
                self.builder.position_at_end(rest_ext);
                let _ = rest_name;
            }
        } else if is_struct {
            match val {
                TypedValue::Struct(alloca, struct_ty) => {
                    let bt: BasicTypeEnum = struct_ty.into();
                    let loaded = self
                        .builder
                        .build_load(bt, alloca, "destr_struct")
                        .map_err(llvm_err)?
                        .into_struct_value();
                    let field_names: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
                    let field_indices: Vec<usize> = if let Some((key, _)) = self
                        .type_layout
                        .anon_structs
                        .iter()
                        .find(|(k, _)| k.as_slice() == field_names)
                    {
                        (0..key.len()).collect()
                    } else {
                        (0..names.len()).collect()
                    };
                    for (i, name) in names.iter().enumerate() {
                        let field_idx = field_indices[i] as u32;
                        let field = self
                            .builder
                            .build_extract_value(loaded, field_idx, &format!("f{}", i))
                            .map_err(llvm_err)?;
                        let field_ty = field.get_type();
                        let local_name = renames
                            .iter()
                            .find(|(fld, _)| fld == name)
                            .map(|(_, local)| local.clone())
                            .unwrap_or_else(|| name.clone());
                        let field_alloca = self
                            .builder
                            .build_alloca(field_ty, &local_name)
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(field_alloca, field)
                            .map_err(llvm_err)?;
                        let kind = self.bv_kind(&field);
                        if mutable {
                            self.scope
                                .set_mutable(local_name, field_alloca, field_ty, kind, None);
                        } else {
                            self.scope.set(local_name, field_alloca, field_ty, kind);
                        }
                    }
                }
                _ => return Err("Struct destructuring requires a struct value".to_string()),
            }
        } else {
            match val {
                TypedValue::Struct(alloca, struct_ty) => {
                    let bt: BasicTypeEnum = struct_ty.into();
                    let loaded = self
                        .builder
                        .build_load(bt, alloca, "destr_tuple")
                        .map_err(llvm_err)?
                        .into_struct_value();
                    for (i, name) in names.iter().enumerate() {
                        let field = self
                            .builder
                            .build_extract_value(loaded, i as u32, &format!("f{}", i))
                            .map_err(llvm_err)?;
                        let field_ty = field.get_type();
                        let field_alloca = self
                            .builder
                            .build_alloca(field_ty, name)
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(field_alloca, field)
                            .map_err(llvm_err)?;
                        let kind = self.bv_kind(&field);
                        if mutable {
                            self.scope.set_mutable(
                                name.clone(),
                                field_alloca,
                                field_ty,
                                kind,
                                None,
                            );
                        } else {
                            self.scope.set(name.clone(), field_alloca, field_ty, kind);
                        }
                    }
                }
                _ => return Err("Destructuring requires a tuple value".to_string()),
            }
        }
        Ok(())
    }

    pub(crate) fn compile_hir_const(&mut self, name: &str, value: &HirExpr) -> Result<(), String> {
        match &value.kind {
            HirExprKind::Literal(lit) => {
                let (global_ptr, ty, kind): (
                    inkwell::values::PointerValue<'ctx>,
                    BasicTypeEnum<'ctx>,
                    crate::ValKind,
                ) = match lit {
                    Literal::Int(n) => {
                        let g = self.add_module_global(self.i64_ty(), name)?;
                        g.set_initializer(&self.i64_ty().const_int(*n as u64, true));
                        (
                            g.as_pointer_value(),
                            self.i64_ty().into(),
                            crate::ValKind::Int,
                        )
                    }
                    Literal::Float(n) => {
                        let g = self.add_module_global(self.f64_ty(), name)?;
                        g.set_initializer(&self.f64_ty().const_float(*n));
                        (
                            g.as_pointer_value(),
                            self.f64_ty().into(),
                            crate::ValKind::Float,
                        )
                    }
                    Literal::Bool(b) => {
                        let g = self.add_module_global(self.bool_ty(), name)?;
                        g.set_initializer(&self.bool_ty().const_int(if *b { 1 } else { 0 }, false));
                        (
                            g.as_pointer_value(),
                            self.bool_ty().into(),
                            crate::ValKind::Bool,
                        )
                    }
                    Literal::Char(c) => {
                        let g = self.add_module_global(self.i64_ty(), name)?;
                        g.set_initializer(&self.i64_ty().const_int(*c as u64, false));
                        (
                            g.as_pointer_value(),
                            self.i64_ty().into(),
                            crate::ValKind::Int,
                        )
                    }
                    Literal::Unit => {
                        let g = self.add_module_global(self.i64_ty(), name)?;
                        g.set_initializer(&self.i64_ty().const_int(0, false));
                        (
                            g.as_pointer_value(),
                            self.i64_ty().into(),
                            crate::ValKind::Unit,
                        )
                    }
                    Literal::String(s) => {
                        let content_bytes: Vec<u8> = s.bytes().chain(std::iter::once(0)).collect();
                        let arr_ty = self
                            .context
                            .i8_type()
                            .array_type(content_bytes.len() as u32);
                        let str_data_g =
                            self.add_module_global(arr_ty, &format!("__const_str_data_{}", name))?;
                        let arr_val = self.context.const_string(&content_bytes, false);
                        str_data_g.set_initializer(&arr_val);
                        let len_val = self.i64_ty().const_int(s.len() as u64, false);
                        let i8_ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                        let data_ptr = str_data_g.as_pointer_value();
                        let data_ptr_i8 = data_ptr.const_cast(i8_ptr_ty);
                        let fat_struct = self
                            .context
                            .const_struct(&[len_val.into(), data_ptr_i8.into()], false);
                        let g = self.add_module_global(self.string_type, name)?;
                        g.set_initializer(&fat_struct);
                        (
                            g.as_pointer_value(),
                            self.string_type.into(),
                            crate::ValKind::Str,
                        )
                    }
                };
                self.type_layout
                    .consts
                    .insert(name.to_string(), (global_ptr, ty, kind));
            }
            _ => {
                let val = self.compile_hir_expr(value)?;
                if let Some(bv) = val.to_bv() {
                    let ty = bv.get_type();
                    let g = self.add_module_global(ty, name)?;
                    g.set_initializer(&bv);
                    self.type_layout
                        .consts
                        .insert(name.to_string(), (g.as_pointer_value(), ty, val.val_kind()));
                } else {
                    return Err(format!("Non-basic-value const '{}' is not supported", name));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn rename_module_hir_stmt(&self, stmt: &HirStmt, prefix: &str) -> HirStmt {
        match stmt {
            HirStmt::Fun {
                name,
                params,
                return_type,
                body,
                type_params,
                is_single_expr,
                is_test,
                span,
            } => HirStmt::Fun {
                name: format!("{}{}", prefix, name),
                params: params.clone(),
                return_type: return_type.clone(),
                body: body.clone(),
                type_params: type_params.clone(),
                is_single_expr: *is_single_expr,
                is_test: *is_test,
                span: *span,
            },
            HirStmt::Const {
                name,
                type_ann,
                value,
                span,
            } => HirStmt::Const {
                name: format!("{}{}", prefix, name),
                type_ann: type_ann.clone(),
                value: value.clone(),
                span: *span,
            },
            other => other.clone(),
        }
    }
}
