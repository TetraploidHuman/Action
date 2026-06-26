//! Fallible expression codegen (R7 `or { }` vertical slice).

use action_frontend::ast::{Literal, Type};
use action_frontend::builtin;
use action_frontend::hir::{HirExpr, HirExprKind};
use inkwell::basic_block::BasicBlock;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, FloatValue, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};
use crate::call_arg::CallArg;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn is_fallible_user_fn(&self, llvm_name: &str) -> bool {
        self.mono_cache.fallible_user_fns.contains(llvm_name)
    }

    pub(crate) fn fallible_ret_struct_type(
        &mut self,
        ret_ast: &Type,
    ) -> Result<inkwell::types::StructType<'ctx>, String> {
        let payload = self.ast_type_to_basic_type(ret_ast);
        Ok(self
            .context
            .struct_type(&[payload, self.bool_ty().into()], false))
    }

    pub(crate) fn typed_to_fallible_payload_bv(
        &mut self,
        val: &TypedValue<'ctx>,
        ret_ast: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match val {
            TypedValue::Int(v) => Ok((*v).into()),
            TypedValue::Float(v) => Ok((*v).into()),
            TypedValue::Bool(v) => Ok((*v).into()),
            TypedValue::Ptr(v) => Ok((*v).into()),
            TypedValue::Str(ptr) => Ok(self.load_string(*ptr)?.into()),
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "fall_payload_struct")
                    .map_err(llvm_err)?;
                Ok(loaded)
            }
            _ => Err(format!(
                "fallible return: unsupported payload type for {}",
                ret_ast
            )),
        }
    }

    pub(crate) fn zero_fallible_payload_bv(
        &mut self,
        ret_ast: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match ret_ast {
            Type::Named(n) => match n.as_str() {
                "Int" => Ok(self.i64_ty().const_zero().into()),
                "Float" | "Double" => Ok(self.f64_ty().const_zero().into()),
                "Bool" => Ok(self.bool_ty().const_zero().into()),
                "String" | "Str" => Ok(self.string_type.const_zero().into()),
                name => {
                    if let Some(st) = self.type_layout.named_structs.get(name) {
                        Ok(st.const_zero().into())
                    } else {
                        Ok(self.i64_ty().const_zero().into())
                    }
                }
            },
            Type::Ptr(_) | Type::CString | Type::FileHandle => {
                Ok(self.ptr_ty().const_null().into())
            }
            _ => Ok(self.i64_ty().const_zero().into()),
        }
    }

    pub(crate) fn build_fallible_ret_pair(
        &mut self,
        payload: BasicValueEnum<'ctx>,
        ok: IntValue<'ctx>,
        ret_ast: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let st = self.fallible_ret_struct_type(ret_ast)?;
        let ok_i1 = self.ok_i1(ok)?;
        let undef = st.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, payload, 0, "fall_payload")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, ok_i1, 1, "fall_ok")
            .map_err(llvm_err)?;
        Ok(r2.into_struct_value().into())
    }

    pub(crate) fn build_fallible_ok_return(
        &mut self,
        val: &TypedValue<'ctx>,
        ret_ast: &Type,
    ) -> Result<(), String> {
        let payload = self.typed_to_fallible_payload_bv(val, ret_ast)?;
        let one = self.bool_ty().const_int(1, false);
        let pair = self.build_fallible_ret_pair(payload, one, ret_ast)?;
        let _ = self.builder.build_return(Some(&pair));
        Ok(())
    }

    pub(crate) fn build_fallible_fail_return(&mut self, ret_ast: &Type) -> Result<(), String> {
        let payload = self.zero_fallible_payload_bv(ret_ast)?;
        let zero = self.bool_ty().const_zero();
        let pair = self.build_fallible_ret_pair(payload, zero, ret_ast)?;
        let _ = self.builder.build_return(Some(&pair));
        Ok(())
    }

    fn fallible_pair_to_typed(
        &mut self,
        pair: inkwell::values::StructValue<'ctx>,
        ret_ast: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        let ok = self
            .builder
            .build_extract_value(pair, 1, "fall_ok")
            .map_err(llvm_err)?
            .into_int_value();
        let ok_i1 = self.ok_i1(ok)?;
        match ret_ast {
            Type::Named(n) if n == "Int" => {
                let val = self
                    .builder
                    .build_extract_value(pair, 0, "fall_val")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::FallibleInt { val, ok: ok_i1 })
            }
            Type::Named(n) if n == "String" || n == "Str" => {
                let sv = self
                    .builder
                    .build_extract_value(pair, 0, "fall_str")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let alloca = self
                    .builder
                    .build_alloca(self.string_type, "fall_str_tmp")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, sv).map_err(llvm_err)?;
                Ok(TypedValue::FallibleStr {
                    val: alloca,
                    ok: ok_i1,
                })
            }
            Type::Ptr(_) | Type::CString | Type::FileHandle => {
                let val = self
                    .builder
                    .build_extract_value(pair, 0, "fall_ptr")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                Ok(TypedValue::FalliblePtr { val, ok: ok_i1 })
            }
            Type::Named(name) => {
                if let Some(st) = self.type_layout.named_structs.get(name) {
                    let loaded = self
                        .builder
                        .build_extract_value(pair, 0, "fall_struct")
                        .map_err(llvm_err)?
                        .into_struct_value();
                    let alloca = self
                        .builder
                        .build_alloca(*st, "fall_struct_tmp")
                        .map_err(llvm_err)?;
                    self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
                    Ok(TypedValue::FallibleStruct {
                        val: alloca,
                        ty: *st,
                        ok: ok_i1,
                    })
                } else {
                    Err(format!(
                        "fallible call: unknown struct return type {}",
                        name
                    ))
                }
            }
            _ => Err(format!(
                "fallible call: unsupported return type {}",
                ret_ast
            )),
        }
    }

    pub(crate) fn compile_fallible_user_call(
        &mut self,
        llvm_name: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self
            .module
            .get_function(llvm_name)
            .ok_or_else(|| format!("Function '{}' not found", llvm_name))?;
        let fn_type = fn_val.get_type();
        let param_tys = fn_type.get_param_types();
        let mut ca = Vec::new();
        let mut direct_arg_vals = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let av = self.compile_call_arg(*a)?;
            let bv = self.typed_value_to_bv(&av);
            let casted = self.coerce_arg(bv, param_tys.get(i))?;
            ca.push(casted.into());
            direct_arg_vals.push(av);
        }
        if let Some(lam) = trailing {
            let bv = self.compile_and_load_call_arg(lam)?;
            let casted = self.coerce_arg(bv, param_tys.get(args.len()))?;
            ca.push(casted.into());
        }
        let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
        for av in &direct_arg_vals {
            self.rc_free_intermediate(av)?;
        }
        let ret_ast = self
            .mono_cache
            .fun_return_types
            .get(llvm_name)
            .ok_or_else(|| format!("missing return type for fallible fn {}", llvm_name))?
            .clone();
        let pair = cc
            .try_as_basic_value()
            .basic()
            .ok_or("fallible fn call failed")?
            .into_struct_value();
        let tv = self.fallible_pair_to_typed(pair, &ret_ast)?;
        if let TypedValue::FallibleInt { ok, .. }
        | TypedValue::FallibleStr { ok, .. }
        | TypedValue::FalliblePtr { ok, .. }
        | TypedValue::FallibleStruct { ok, .. } = &tv
        {
            self.branch_to_fail_if(*ok)?;
        }
        Ok(tv)
    }

    pub(crate) fn compile_http_request_fallible(
        &mut self,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 4 {
            return Err("httpRequest expects 4 arguments".to_string());
        }
        let v = self.builtin_http_request_call_args(args[0], args[1], args[2], args[3])?;
        let TypedValue::Struct(ptr, ty) = v else {
            return Err("httpRequest: expected HttpResponse struct".to_string());
        };
        let bt: BasicTypeEnum = ty.into();
        let loaded = self
            .builder
            .build_load(bt, ptr, "http_resp")
            .map_err(llvm_err)?
            .into_struct_value();
        let status = self
            .builder
            .build_extract_value(loaded, 0, "http_status")
            .map_err(llvm_err)?
            .into_int_value();
        let zero = self.i64_ty().const_int(0, false);
        let ok = self
            .builder
            .build_int_compare(IntPredicate::NE, status, zero, "http_ok")
            .map_err(llvm_err)?;
        let ok_i1 = self.ok_i1(ok)?;
        self.branch_to_fail_if(ok_i1)?;
        Ok(TypedValue::FallibleStruct {
            val: ptr,
            ty,
            ok: ok_i1,
        })
    }

    pub(crate) fn in_fallible_region(&self) -> bool {
        self.or_block_depth > 0 || !self.fallible_fail_stack.is_empty()
    }

    pub(crate) fn current_fail_bb(&self) -> Option<BasicBlock<'ctx>> {
        self.fallible_fail_stack.last().copied()
    }

    pub(crate) fn push_fallible_fail_bb(&mut self, bb: BasicBlock<'ctx>) {
        self.fallible_fail_stack.push(bb);
    }

    pub(crate) fn pop_fallible_fail_bb(&mut self) {
        self.fallible_fail_stack.pop();
    }

    pub(crate) fn ok_i1(&mut self, ok: IntValue<'ctx>) -> Result<IntValue<'ctx>, String> {
        if ok.get_type().get_bit_width() == 1 {
            Ok(ok)
        } else {
            self.builder
                .build_int_compare(
                    IntPredicate::NE,
                    ok,
                    self.i64_ty().const_int(0, false),
                    "fok",
                )
                .map_err(llvm_err)
        }
    }

    pub(crate) fn branch_to_fail_if(&mut self, ok: IntValue<'ctx>) -> Result<(), String> {
        let Some(fail_bb) = self.current_fail_bb() else {
            return Ok(());
        };
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("fallible: no function")?;
        let ok_bb = self.context.append_basic_block(current_fn, "fall_ok");
        let ok_i1 = self.ok_i1(ok)?;
        let failed = self
            .builder
            .build_int_compare(IntPredicate::EQ, ok_i1, self.bool_ty().const_zero(), "fall")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(failed, fail_bb, ok_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(ok_bb);
        Ok(())
    }

    pub(crate) fn build_fallible_int_from_ok(
        &mut self,
        val: IntValue<'ctx>,
        is_ok: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let ok_i1 = self.ok_i1(is_ok)?;
        self.branch_to_fail_if(ok_i1)?;
        Ok(TypedValue::FallibleInt { val, ok: ok_i1 })
    }

    /// Build a fallible payload from a list element fat struct + found flag, using AST element type.
    pub(crate) fn build_fallible_from_fat_found_flag(
        &mut self,
        fat_alloca: PointerValue<'ctx>,
        found_flag_a: PointerValue<'ctx>,
        elem_ty: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        let is_found = self
            .builder
            .build_load(self.bool_ty(), found_flag_a, "ff")
            .map_err(llvm_err)?
            .into_int_value();
        let ok_i1 = self.ok_i1(is_found)?;
        self.branch_to_fail_if(ok_i1)?;
        match elem_ty {
            Type::Named(n) if n == "String" || n == "Str" => Ok(TypedValue::FallibleStr {
                val: fat_alloca,
                ok: ok_i1,
            }),
            Type::Named(n) if matches!(n.as_str(), "Int" | "Bool" | "Char" | "Float") => {
                let fat = self
                    .builder
                    .build_load(self.string_type, fat_alloca, "fat_elem")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let tag = self
                    .builder
                    .build_extract_value(fat, 0, "fat_tag")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::FallibleInt { val: tag, ok: ok_i1 })
            }
            Type::Ptr(_) | Type::CString | Type::FileHandle => {
                let fat = self
                    .builder
                    .build_load(self.string_type, fat_alloca, "fat_elem")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let ptr = self
                    .builder
                    .build_extract_value(fat, 1, "fat_data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                Ok(TypedValue::FalliblePtr { val: ptr, ok: ok_i1 })
            }
            Type::Named(name) => {
                if let Some(st) = self.type_layout.named_structs.get(name) {
                    let fat = self
                        .builder
                        .build_load(self.string_type, fat_alloca, "fat_elem")
                        .map_err(llvm_err)?
                        .into_struct_value();
                    let st_alloca = self
                        .builder
                        .build_alloca(*st, "find_struct")
                        .map_err(llvm_err)?;
                    self.builder
                        .build_store(st_alloca, fat)
                        .map_err(llvm_err)?;
                    Ok(TypedValue::FallibleStruct {
                        val: st_alloca,
                        ty: *st,
                        ok: ok_i1,
                    })
                } else {
                    Err(format!("find: unknown element struct type {}", name))
                }
            }
            _ => {
                let fat = self
                    .builder
                    .build_load(self.string_type, fat_alloca, "fat_elem")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let tag = self
                    .builder
                    .build_extract_value(fat, 0, "fat_tag")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::FallibleInt { val: tag, ok: ok_i1 })
            }
        }
    }

    pub(crate) fn build_fallible_list_from_not_empty(
        &mut self,
        list_alloca: PointerValue<'ctx>,
        is_empty: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let not_empty = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_empty,
                self.bool_ty().const_zero(),
                "not_empty",
            )
            .map_err(llvm_err)?;
        let ok_i1 = self.ok_i1(not_empty)?;
        self.branch_to_fail_if(ok_i1)?;
        Ok(TypedValue::FallibleStruct {
            val: list_alloca,
            ty: self.list_type,
            ok: ok_i1,
        })
    }

    pub(crate) fn build_fallible_float_from_ok(
        &mut self,
        val: FloatValue<'ctx>,
        is_ok: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let ok_i1 = self.ok_i1(is_ok)?;
        self.branch_to_fail_if(ok_i1)?;
        Ok(TypedValue::FallibleFloat { val, ok: ok_i1 })
    }

    pub(crate) fn compile_string_index_of_fallible(
        &mut self,
        needle_arg: CallArg<'_>,
        haystack_arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v1 = self.compile_call_arg(needle_arg)?;
        let v2 = self.compile_call_arg(haystack_arg)?;
        match (&v1, &v2) {
            (elem, TypedValue::List(lp)) => {
                let lv = self.load_list(*lp)?;
                let fat = self.to_fat_struct(elem)?;
                let cc = self.call_rt("action_list_index_of", &[lv.into(), fat.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("indexOf failed")?
                    .into_int_value();
                let zero = self.i64_ty().const_zero();
                let found = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, result, zero, "found")
                    .map_err(llvm_err)?;
                self.build_fallible_int_from_ok(result, found)
            }
            (TypedValue::Str(sp1), TypedValue::Str(sp2)) => {
                let sv1 = self.load_string(*sp1)?;
                let sv2 = self.load_string(*sp2)?;
                let cc =
                    self.call_rt("action_string_index_of", &[sv2.into(), sv1.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("indexOf failed")?
                    .into_int_value();
                let neg_one = self.i64_ty().const_int((-1i64) as u64, true);
                let found = self
                    .builder
                    .build_int_compare(IntPredicate::NE, result, neg_one, "found")
                    .map_err(llvm_err)?;
                self.build_fallible_int_from_ok(result, found)
            }
            _ => Err(
                "indexOf: first arg must be (element, list) or (substring, string)".to_string(),
            ),
        }
    }

    pub(crate) fn compile_tail_fallible_call(
        &mut self,
        list_arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_call_arg(list_arg)?;
        match v {
            TypedValue::List(lp) => {
                let lv = self.load_list(lp)?;
                let len = self
                    .builder
                    .build_extract_value(lv, 1, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_empty = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        len,
                        self.i64_ty().const_int(0, false),
                        "empty",
                    )
                    .map_err(llvm_err)?;
                let cc = self.call_rt("action_list_tail", &[lv.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("tail failed")?
                    .into_struct_value();
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "tail_result")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                self.build_fallible_list_from_not_empty(alloca, is_empty)
            }
            _ => Err("tail: argument must be a list".to_string()),
        }
    }

    pub(crate) fn compile_init_fallible_call(
        &mut self,
        list_arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_call_arg(list_arg)?;
        match v {
            TypedValue::List(lp) => {
                let lv = self.load_list(lp)?;
                let len = self
                    .builder
                    .build_extract_value(lv, 1, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_empty = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        len,
                        self.i64_ty().const_int(0, false),
                        "empty",
                    )
                    .map_err(llvm_err)?;
                let cc = self.call_rt("action_list_init", &[lv.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("init failed")?
                    .into_struct_value();
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "init_result")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                self.build_fallible_list_from_not_empty(alloca, is_empty)
            }
            _ => Err("init: argument must be a list".to_string()),
        }
    }

    pub(crate) fn compile_to_char_fallible_call(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_call_arg(arg)?;
        match v {
            TypedValue::Int(iv) => {
                let max_cp = self.i64_ty().const_int(0x10FFFF, false);
                let in_range = self
                    .builder
                    .build_int_compare(IntPredicate::ULE, iv, max_cp, "valid_cp")
                    .map_err(llvm_err)?;
                self.build_fallible_int_from_ok(iv, in_range)
            }
            _ => Err("toChar: argument must be an Int".to_string()),
        }
    }

    pub(crate) fn compile_to_float_fallible_call(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let always_true = self.bool_ty().const_int(1, false);
        let v = self.compile_call_arg(arg)?;
        match v {
            TypedValue::Float(fv) => self.build_fallible_float_from_ok(fv, always_true),
            TypedValue::Int(iv) => {
                let f = self
                    .builder
                    .build_signed_int_to_float(iv, self.f64_ty(), "itof")
                    .map_err(llvm_err)?;
                self.build_fallible_float_from_ok(f, always_true)
            }
            TypedValue::Str(sp) => {
                let sv = self.load_string(sp)?;
                let len = self
                    .builder
                    .build_extract_value(sv, 0, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let has_chars = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGT,
                        len,
                        self.i64_ty().const_int(0, false),
                        "has_chars",
                    )
                    .map_err(llvm_err)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(sv, 1, "dptr")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let first_char = self
                    .builder
                    .build_load(self.context.i8_type(), data_ptr, "first_char")
                    .map_err(llvm_err)?
                    .into_int_value();
                let i8 = self.context.i8_type();
                let is_digit = self
                    .builder
                    .build_int_compare(
                        IntPredicate::UGE,
                        first_char,
                        i8.const_int(b'0' as u64, false),
                        "isd",
                    )
                    .map_err(llvm_err)?;
                let le9 = self
                    .builder
                    .build_int_compare(
                        IntPredicate::ULE,
                        first_char,
                        i8.const_int(b'9' as u64, false),
                        "le9",
                    )
                    .map_err(llvm_err)?;
                let is_d = self.builder.build_and(is_digit, le9, "is_digit").map_err(llvm_err)?;
                let is_minus = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        first_char,
                        i8.const_int(b'-' as u64, false),
                        "is_minus",
                    )
                    .map_err(llvm_err)?;
                let is_plus = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        first_char,
                        i8.const_int(b'+' as u64, false),
                        "is_plus",
                    )
                    .map_err(llvm_err)?;
                let is_dot = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        first_char,
                        i8.const_int(b'.' as u64, false),
                        "is_dot",
                    )
                    .map_err(llvm_err)?;
                let is_sign = self.builder.build_or(is_minus, is_plus, "is_sign").map_err(llvm_err)?;
                let is_num_start = self.builder.build_or(is_d, is_sign, "is_num1").map_err(llvm_err)?;
                let is_valid = self.builder.build_or(is_num_start, is_dot, "is_valid").map_err(llvm_err)?;
                let ok = self.builder.build_and(has_chars, is_valid, "ok").map_err(llvm_err)?;
                let strtod_fn = self.module.get_function("strtod").unwrap();
                let null_ptr = self.ptr_ty().const_zero();
                let result = self
                    .builder
                    .build_call(strtod_fn, &[data_ptr.into(), null_ptr.into()], "fval")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("strtod failed")?
                    .into_float_value();
                self.build_fallible_float_from_ok(result, ok)
            }
            _ => Err("toFloat: cannot convert to Float".to_string()),
        }
    }

    pub(crate) fn list_element_ast_type(&self, list_arg: CallArg<'_>) -> Type {
        let CallArg::Hir(e) = list_arg;
        if let Some(elem) = self.element_type_from_list_ast(&e.ty) {
            return elem;
        }
        if let Some(elem) = self.element_type_from_list_ast(&self.infer_hir_expr_type(e)) {
            return elem;
        }
        Type::Named("Int".into())
    }

    fn element_type_from_list_ast(&self, ty: &Type) -> Option<Type> {
        match ty {
            Type::Generic(base, args) if !args.is_empty() => match base.as_ref() {
                Type::Named(n) if n == "List" || n == "LazyList" => Some(args[0].clone()),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn compile_head_fallible_on_list_ptr(
        &mut self,
        lp: PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let list_val = self.load_list(lp)?;
        let len = self
            .builder
            .build_extract_value(list_val, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let zero = self.i64_ty().const_int(0, false);
        let not_empty = self
            .builder
            .build_int_compare(IntPredicate::NE, len, zero, "not_empty")
            .map_err(llvm_err)?;
        self.branch_to_fail_if(not_empty)?;
        let elem = self.call_rt("action_list_get", &[list_val.into(), zero.into()])?;
        let tag = self
            .builder
            .build_extract_value(
                elem.try_as_basic_value()
                    .basic()
                    .ok_or("get failed")?
                    .into_struct_value(),
                0,
                "tag",
            )
            .map_err(llvm_err)?
            .into_int_value();
        Ok(TypedValue::FallibleInt {
            val: tag,
            ok: not_empty,
        })
    }

    pub(crate) fn compile_find_fallible_call(
        &mut self,
        list_arg: CallArg<'_>,
        fn_arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let elem_ty = self.list_element_ast_type(list_arg);
        let list_val = self.compile_call_arg(list_arg)?;
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("find: argument must be a list".to_string()),
        };
        let fn_val = self.compile_call_arg(fn_arg)?;
        self.find_on_list_ptr(list_ptr, fn_val, &elem_ty)
    }

    pub(crate) fn compile_find_index_fallible_call(
        &mut self,
        list_arg: CallArg<'_>,
        fn_arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let list_val = self.compile_call_arg(list_arg)?;
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("findIndex: argument must be a list".to_string()),
        };
        let fn_val = self.compile_call_arg(fn_arg)?;
        self.find_index_on_list_ptr(list_ptr, fn_val)
    }

    pub(crate) fn compile_list_index_of_fallible(
        &mut self,
        list_arg: CallArg<'_>,
        elem_arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let list_val = self.compile_call_arg(list_arg)?;
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("indexOf: argument must be a list".to_string()),
        };
        let elem_val = self.compile_call_arg(elem_arg)?;
        let fat = self.to_fat_struct(&elem_val)?;
        let lv = self.load_list(list_ptr)?;
        let cc = self.call_rt("action_list_index_of", &[lv.into(), fat.into()])?;
        let result = cc
            .try_as_basic_value()
            .basic()
            .ok_or("indexOf failed")?
            .into_int_value();
        let zero = self.i64_ty().const_zero();
        let found = self
            .builder
            .build_int_compare(IntPredicate::SGE, result, zero, "found")
            .map_err(llvm_err)?;
        self.build_fallible_int_from_ok(result, found)
    }

    pub(crate) fn compile_to_int_fallible_call(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let one = self.bool_ty().const_int(1, false);
        let v = self.compile_call_arg(arg)?;
        match v {
            TypedValue::Int(iv) => Ok(TypedValue::FallibleInt { val: iv, ok: one }),
            TypedValue::Float(fv) => {
                let i = self
                    .builder
                    .build_float_to_signed_int(fv, self.i64_ty(), "ftoi")
                    .map_err(llvm_err)?;
                Ok(TypedValue::FallibleInt { val: i, ok: one })
            }
            TypedValue::Str(sp) => {
                let sv = self.load_string(sp)?;
                let cc = self.call_rt("action_parse_int", &[sv.into()])?;
                let st = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("parseInt failed")?
                    .into_struct_value();
                let val = self
                    .builder
                    .build_extract_value(st, 0, "val")
                    .map_err(llvm_err)?
                    .into_int_value();
                let ok = self
                    .builder
                    .build_extract_value(st, 1, "ok")
                    .map_err(llvm_err)?
                    .into_int_value();
                let ok_i1 = self.ok_i1(ok)?;
                self.branch_to_fail_if(ok_i1)?;
                Ok(TypedValue::FallibleInt { val, ok: ok_i1 })
            }
            _ => Err("toInt: cannot convert to Int".to_string()),
        }
    }

    pub(crate) fn compile_head_fallible(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_call_arg(arg)?;
        match v {
            TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                self.compile_head_fallible_on_list_ptr(lp)
            }
            _ => Err("head: argument must be a list".to_string()),
        }
    }

    pub(crate) fn compile_last_fallible_on_list_ptr(
        &mut self,
        lp: PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let list_val = self.load_list(lp)?;
        let len = self
            .builder
            .build_extract_value(list_val, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let zero = self.i64_ty().const_zero();
        let not_empty = self
            .builder
            .build_int_compare(IntPredicate::SGT, len, zero, "not_empty")
            .map_err(llvm_err)?;
        self.branch_to_fail_if(not_empty)?;
        let last_idx = self
            .builder
            .build_int_sub(len, self.i64_ty().const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        let elem = self.call_rt(
            "action_list_get",
            &[list_val.into(), last_idx.into()],
        )?;
        let tag = self
            .builder
            .build_extract_value(
                elem.try_as_basic_value()
                    .basic()
                    .ok_or("last get failed")?
                    .into_struct_value(),
                0,
                "tag",
            )
            .map_err(llvm_err)?
            .into_int_value();
        Ok(TypedValue::FallibleInt {
            val: tag,
            ok: not_empty,
        })
    }

    pub(crate) fn compile_list_get_fallible_on_ptr(
        &mut self,
        lp: PointerValue<'ctx>,
        idx_iv: inkwell::values::IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let list_val = self.load_list(lp)?;
        let len = self
            .builder
            .build_extract_value(list_val, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let zero = self.i64_ty().const_zero();
        let neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, idx_iv, zero, "neg")
            .map_err(llvm_err)?;
        let ge_len = self
            .builder
            .build_int_compare(IntPredicate::SGE, idx_iv, len, "ge_len")
            .map_err(llvm_err)?;
        let oob = self.builder.build_or(neg, ge_len, "oob").map_err(llvm_err)?;
        let in_range = self
            .builder
            .build_int_compare(IntPredicate::EQ, oob, self.bool_ty().const_zero(), "in_range")
            .map_err(llvm_err)?;
        let elem_bv = if let Some(cache) = self.loop_control.list_loop_get_cache {
            self.list_get_cached_fat(lp, idx_iv, cache)?.into_struct_value()
        } else {
            let elem = self.call_rt("action_list_get", &[list_val.into(), idx_iv.into()])?;
            elem.try_as_basic_value()
                .basic()
                .ok_or("get failed")?
                .into_struct_value()
        };
        let fat_alloca = self
            .builder
            .build_alloca(self.string_type, "get_fat")
            .map_err(llvm_err)?;
        self.builder.build_store(fat_alloca, elem_bv).map_err(llvm_err)?;
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "get_ok")
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_flag_a, in_range)
            .map_err(llvm_err)?;
        self.build_fallible_from_fat_found_flag(
            fat_alloca,
            found_flag_a,
            &Type::Named("Int".into()),
        )
    }

    pub(crate) fn compile_last_fallible(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_call_arg(arg)?;
        match v {
            TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                self.compile_last_fallible_on_list_ptr(lp)
            }
            _ => Err("last: argument must be a list".to_string()),
        }
    }

    pub(crate) fn compile_list_index_fallible(
        &mut self,
        list_arg: CallArg<'_>,
        idx_iv: inkwell::values::IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_call_arg(list_arg)?;
        match v {
            TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                let list_val = self.load_list(lp)?;
                let len = self
                    .builder
                    .build_extract_value(list_val, 1, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let in_range = self
                    .builder
                    .build_int_compare(IntPredicate::ULT, idx_iv, len, "in_range")
                    .map_err(llvm_err)?;
                self.branch_to_fail_if(in_range)?;
                let elem = self.call_rt("action_list_get", &[list_val.into(), idx_iv.into()])?;
                let tag = self
                    .builder
                    .build_extract_value(
                        elem.try_as_basic_value()
                            .basic()
                            .ok_or("get failed")?
                            .into_struct_value(),
                        0,
                        "tag",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::FallibleInt {
                    val: tag,
                    ok: in_range,
                })
            }
            _ => Err("list index: argument must be a list".to_string()),
        }
    }

    pub(crate) fn compile_list_index_literal_fallible(
        &mut self,
        list_arg: CallArg<'_>,
        idx: i64,
    ) -> Result<TypedValue<'ctx>, String> {
        let idx_iv = self.i64_ty().const_int(idx as u64, true);
        self.compile_list_index_fallible(list_arg, idx_iv)
    }

    pub(crate) fn compile_map_index_fallible(
        &mut self,
        map_arg: CallArg<'_>,
        key_arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_call_arg(map_arg)?;
        let key_val = self.compile_call_arg(key_arg)?;
        let TypedValue::Map(map_ptr) = v else {
            return Err("map index: argument must be a map".to_string());
        };
        let key_fat = self.to_fat_struct(&key_val)?;
        let map_loaded = self.load_list(map_ptr)?;
        let cc = self.call_rt("action_map_contains", &[map_loaded.into(), key_fat.into()])?;
        let contains = cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_contains failed")?
            .into_int_value();
        self.branch_to_fail_if(contains)?;
        let map_loaded2 = self.load_list(map_ptr)?;
        let key_fat2 = self.to_fat_struct(&key_val)?;
        let gc = self.call_rt("action_map_get", &[map_loaded2.into(), key_fat2.into()])?;
        let val_fat = gc
            .try_as_basic_value()
            .basic()
            .ok_or("map_get failed")?
            .into_struct_value();
        let actual_val = self
            .builder
            .build_extract_value(val_fat, 0, "map_val")
            .map_err(llvm_err)?
            .into_int_value();
        Ok(TypedValue::FallibleInt {
            val: actual_val,
            ok: contains,
        })
    }

    pub(crate) fn compile_set_index_fallible(
        &mut self,
        set_arg: CallArg<'_>,
        elem_arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_call_arg(set_arg)?;
        let elem_val = self.compile_call_arg(elem_arg)?;
        let TypedValue::Set(set_ptr) = v else {
            return Err("set index: argument must be a set".to_string());
        };
        let elem_fat = self.to_fat_struct(&elem_val)?;
        let set_loaded = self.load_list(set_ptr)?;
        let cc = self.call_rt("action_map_contains", &[set_loaded.into(), elem_fat.into()])?;
        let contains = cc
            .try_as_basic_value()
            .basic()
            .ok_or("set_contains failed")?
            .into_int_value();
        self.branch_to_fail_if(contains)?;
        let elem_fat2 = self.to_fat_struct(&elem_val)?;
        let actual_val = self
            .builder
            .build_extract_value(elem_fat2.into_struct_value(), 0, "set_val")
            .map_err(llvm_err)?
            .into_int_value();
        Ok(TypedValue::FallibleInt {
            val: actual_val,
            ok: contains,
        })
    }

    pub(crate) fn try_compile_fallible_lhs_for_or(
        &mut self,
        expr: &HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        self.try_compile_fallible_expr(expr)
    }

    pub(crate) fn try_compile_fallible_call_in_region(
        &mut self,
        expr: &HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        if !self.in_fallible_region() {
            return Ok(None);
        }
        self.try_compile_fallible_expr(expr)
    }

    fn try_compile_fallible_expr(
        &mut self,
        expr: &HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match &expr.kind {
            HirExprKind::Call {
                func,
                args,
                trailing_lambda,
                ..
            } => {
                let trailing_ca = trailing_lambda.as_ref().map(|l| CallArg::hir(l.as_ref()));
                if let HirExprKind::FieldAccess(obj, method) = &func.kind {
                    match method.as_str() {
                        "get" if args.len() == 1 => {
                            let idx_val = self.compile_call_arg(CallArg::hir(&args[0]))?;
                            if let TypedValue::Int(iv) = idx_val {
                                return self
                                    .compile_list_index_fallible(CallArg::hir(obj), iv)
                                    .map(Some);
                            }
                        }
                        "last" if args.is_empty() => {
                            return self.compile_last_fallible(CallArg::hir(obj)).map(Some);
                        }
                        "tail" if args.is_empty() => {
                            return self
                                .compile_tail_fallible_call(CallArg::hir(obj))
                                .map(Some);
                        }
                        "init" if args.is_empty() => {
                            return self
                                .compile_init_fallible_call(CallArg::hir(obj))
                                .map(Some);
                        }
                        "head" if args.is_empty() => {
                            return self.compile_head_fallible(CallArg::hir(obj)).map(Some);
                        }
                        "indexOf" if args.len() == 1 => {
                            return self
                                .compile_list_index_of_fallible(
                                    CallArg::hir(obj),
                                    CallArg::hir(&args[0]),
                                )
                                .map(Some);
                        }
                        "find" if trailing_lambda.is_some() || args.len() == 1 => {
                            let fn_arg = trailing_ca.unwrap_or(CallArg::hir(&args[0]));
                            return self
                                .compile_find_fallible_call(CallArg::hir(obj), fn_arg)
                                .map(Some);
                        }
                        "findIndex" if trailing_lambda.is_some() || args.len() == 1 => {
                            let fn_arg = trailing_ca.unwrap_or(CallArg::hir(&args[0]));
                            return self
                                .compile_find_index_fallible_call(CallArg::hir(obj), fn_arg)
                                .map(Some);
                        }
                        _ => {}
                    }
                }
                if let HirExprKind::Ident(name) = &func.kind {
                    if self
                        .fallibility
                        .symbols
                        .get(name)
                        .is_some_and(|s| s.is_fallible)
                    {
                        let call_args: Vec<CallArg<'_>> = args.iter().map(CallArg::hir).collect();
                        if let Some(stmt) = self.mono_cache.generic_fun_defs.get(name).cloned() {
                            return self
                                .compile_generic_call_from_call_args(&stmt, name, &call_args, None)
                                .map(Some);
                        }
                        if builtin::lookup(name).is_none() {
                            let llvm_name = self.resolve_user_fn_llvm_name(name, &call_args)?;
                            return self
                                .compile_fallible_user_call(&llvm_name, &call_args, None)
                                .map(Some);
                        }
                    }
                    return match name.as_str() {
                        "toInt" if args.len() == 1 => self
                            .compile_to_int_fallible_call(CallArg::hir(&args[0]))
                            .map(Some),
                        "parseInt" if args.len() == 1 => self
                            .compile_to_int_fallible_call(CallArg::hir(&args[0]))
                            .map(Some),
                        "toFloat" if args.len() == 1 => self
                            .compile_to_float_fallible_call(CallArg::hir(&args[0]))
                            .map(Some),
                        "toChar" if args.len() == 1 => self
                            .compile_to_char_fallible_call(CallArg::hir(&args[0]))
                            .map(Some),
                        "tail" if args.len() == 1 => self
                            .compile_tail_fallible_call(CallArg::hir(&args[0]))
                            .map(Some),
                        "init" if args.len() == 1 => self
                            .compile_init_fallible_call(CallArg::hir(&args[0]))
                            .map(Some),
                        "head" if args.len() == 1 => {
                            self.compile_head_fallible(CallArg::hir(&args[0])).map(Some)
                        }
                        "last" if args.len() == 1 => {
                            self.compile_last_fallible(CallArg::hir(&args[0])).map(Some)
                        }
                        "get" if args.len() == 2 => {
                            let idx_val = self.compile_call_arg(CallArg::hir(&args[1]))?;
                            if let TypedValue::Int(iv) = idx_val {
                                return self
                                    .compile_list_index_fallible(CallArg::hir(&args[0]), iv)
                                    .map(Some);
                            }
                            Ok(None)
                        }
                        "indexOf" if args.len() == 2 => self
                            .compile_string_index_of_fallible(
                                CallArg::hir(&args[0]),
                                CallArg::hir(&args[1]),
                            )
                            .map(Some),
                        "find" if trailing_lambda.is_some() && args.len() == 1 => self
                            .compile_find_fallible_call(
                                CallArg::hir(&args[0]),
                                trailing_ca.expect("find trailing lambda"),
                            )
                            .map(Some),
                        "find" if args.len() == 2 => self
                            .compile_find_fallible_call(
                                CallArg::hir(&args[1]),
                                CallArg::hir(&args[0]),
                            )
                            .map(Some),
                        "findIndex" if trailing_lambda.is_some() && args.len() == 1 => self
                            .compile_find_index_fallible_call(
                                CallArg::hir(&args[0]),
                                trailing_ca.expect("findIndex trailing lambda"),
                            )
                            .map(Some),
                        "findIndex" if args.len() == 2 => self
                            .compile_find_index_fallible_call(
                                CallArg::hir(&args[1]),
                                CallArg::hir(&args[0]),
                            )
                            .map(Some),
                        "readLine" if args.is_empty() => {
                            self.compile_read_line_fallible().map(Some)
                        }
                        "__jsonParse" if args.len() == 1 => self
                            .compile_json_parse_fallible(CallArg::hir(&args[0]))
                            .map(Some),
                        "__jsonGet" if args.len() == 2 => self
                            .compile_json_get_fallible(
                                CallArg::hir(&args[0]),
                                CallArg::hir(&args[1]),
                            )
                            .map(Some),
                        "__jsonGetIdx" if args.len() == 2 => self
                            .compile_json_get_idx_fallible(
                                CallArg::hir(&args[0]),
                                CallArg::hir(&args[1]),
                            )
                            .map(Some),
                        "httpRequest" if args.len() == 4 => {
                            let call_args: Vec<CallArg<'_>> =
                                args.iter().map(CallArg::hir).collect();
                            self.compile_http_request_fallible(&call_args).map(Some)
                        }
                        _ => Ok(None),
                    };
                }
            }
            HirExprKind::FieldAccess(obj, method) if method == "head" => {
                return self.compile_head_fallible(CallArg::hir(obj)).map(Some);
            }
            HirExprKind::Index(obj, idx) => {
                if let HirExprKind::Literal(Literal::Int(n)) = &idx.kind {
                    if let Some(tv) =
                        self.compile_call_arg(CallArg::hir(obj))
                            .ok()
                            .and_then(|v| match v {
                                TypedValue::Map(_) => self
                                    .compile_map_index_fallible(
                                        CallArg::hir(obj),
                                        CallArg::hir(idx),
                                    )
                                    .ok(),
                                TypedValue::Set(_) => self
                                    .compile_set_index_fallible(
                                        CallArg::hir(obj),
                                        CallArg::hir(idx),
                                    )
                                    .ok(),
                                _ => self
                                    .compile_list_index_literal_fallible(CallArg::hir(obj), *n)
                                    .ok(),
                            })
                    {
                        return Ok(Some(tv));
                    }
                }
                let obj_val = self.compile_call_arg(CallArg::hir(obj))?;
                let idx_val = self.compile_call_arg(CallArg::hir(idx))?;
                return match obj_val {
                    TypedValue::Map(_) => self
                        .compile_map_index_fallible(CallArg::hir(obj), CallArg::hir(idx))
                        .map(Some),
                    TypedValue::Set(_) => self
                        .compile_set_index_fallible(CallArg::hir(obj), CallArg::hir(idx))
                        .map(Some),
                    TypedValue::List(_) | TypedValue::LazyList(_) => {
                        if let TypedValue::Int(idx_iv) = idx_val {
                            self.compile_list_index_fallible(CallArg::hir(obj), idx_iv)
                                .map(Some)
                        } else {
                            Ok(None)
                        }
                    }
                    _ => Ok(None),
                };
            }
            _ => {}
        }
        Ok(None)
    }

    pub(crate) fn compile_or_block_fallible_from_ok(
        &mut self,
        fallback: &HirExpr,
        lhs: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("or-block: no function")?;

        let (val, ok) = match lhs {
            TypedValue::FallibleInt { val, ok } => (val, ok),
            TypedValue::FallibleFloat { val, ok } => {
                return self.compile_or_block_fallible_float(fallback, val, ok);
            }
            TypedValue::FallibleStr { val, ok } => {
                return self.compile_or_block_fallible_str(fallback, val, ok);
            }
            TypedValue::FalliblePtr { val, ok } => {
                return self.compile_or_block_fallible_ptr(fallback, val, ok);
            }
            TypedValue::FallibleStruct { val, ty, ok } => {
                return self.compile_or_block_fallible_struct(fallback, val, ty, ok);
            }
            other => return Ok(other),
        };

        let ok_i1 = self.ok_i1(ok)?;
        let failed = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ok_i1,
                self.bool_ty().const_zero(),
                "or_fail",
            )
            .map_err(llvm_err)?;

        let fail_bb = self.context.append_basic_block(current_fn, "orblk_fail");
        let ok_bb = self.context.append_basic_block(current_fn, "orblk_ok");
        let merge_bb = self.context.append_basic_block(current_fn, "orblk_merge");

        self.builder
            .build_conditional_branch(failed, fail_bb, ok_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ok_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(fail_bb);
        let fb = self.compile_hir_expr(fallback)?;
        let fb_int = match fb {
            TypedValue::Int(i) => i,
            _ => return Err("or-block fallback must be Int".into()),
        };
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.i64_ty(), "or_int")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&val, ok_bb), (&fb_int, fail_bb)]);
        Ok(TypedValue::Int(phi.as_basic_value().into_int_value()))
    }

    fn compile_or_block_fallible_float(
        &mut self,
        fallback: &HirExpr,
        val: FloatValue<'ctx>,
        ok: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("or-block: no function")?;
        let ok_i1 = self.ok_i1(ok)?;
        let failed = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ok_i1,
                self.bool_ty().const_zero(),
                "or_fail",
            )
            .map_err(llvm_err)?;
        let fail_bb = self.context.append_basic_block(current_fn, "orblk_fail");
        let ok_bb = self.context.append_basic_block(current_fn, "orblk_ok");
        let merge_bb = self.context.append_basic_block(current_fn, "orblk_merge");
        self.builder
            .build_conditional_branch(failed, fail_bb, ok_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(ok_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(fail_bb);
        let fb = self.compile_hir_expr(fallback)?;
        let fb_float = match fb {
            TypedValue::Float(f) => f,
            _ => return Err("or-block fallback must be Float".into()),
        };
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.f64_ty(), "or_float")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&val, ok_bb), (&fb_float, fail_bb)]);
        Ok(TypedValue::Float(phi.as_basic_value().into_float_value()))
    }

    fn compile_or_block_fallible_struct(
        &mut self,
        fallback: &HirExpr,
        val: PointerValue<'ctx>,
        ty: inkwell::types::StructType<'ctx>,
        ok: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("or-block: no function")?;
        let ok_i1 = self.ok_i1(ok)?;
        let failed = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ok_i1,
                self.bool_ty().const_zero(),
                "or_fail",
            )
            .map_err(llvm_err)?;
        let fail_bb = self.context.append_basic_block(current_fn, "orblk_fail");
        let ok_bb = self.context.append_basic_block(current_fn, "orblk_ok");
        let merge_bb = self.context.append_basic_block(current_fn, "orblk_merge");
        self.builder
            .build_conditional_branch(failed, fail_bb, ok_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(ok_bb);
        let bt: BasicTypeEnum = ty.into();
        let ok_loaded = self
            .builder
            .build_load(bt, val, "or_ok_struct")
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(fail_bb);
        let fb = self.compile_hir_expr(fallback)?;
        let fb_ptr = match fb {
            TypedValue::Struct(p, fb_ty) if fb_ty == ty => p,
            TypedValue::List(p) if ty == self.list_type => p,
            _ => return Err("or-block fallback must match struct type".into()),
        };
        let fb_loaded = self
            .builder
            .build_load(bt, fb_ptr, "or_fb_struct")
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(merge_bb);
        let phi = self.builder.build_phi(bt, "or_struct").map_err(llvm_err)?;
        phi.add_incoming(&[(&ok_loaded, ok_bb), (&fb_loaded, fail_bb)]);
        let merged_alloca = self
            .builder
            .build_alloca(ty, "or_struct_out")
            .map_err(llvm_err)?;
        self.builder
            .build_store(merged_alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        if ty == self.list_type {
            Ok(TypedValue::List(merged_alloca))
        } else {
            Ok(TypedValue::Struct(merged_alloca, ty))
        }
    }

    pub(super) fn compile_or_block_hir(
        &mut self,
        nullable: &HirExpr,
        fallback: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.or_block_depth += 1;
        let result = if let Some(v) = self.try_compile_fallible_lhs_for_or(nullable)? {
            self.compile_or_block_fallible_from_ok(fallback, v)
        } else {
            self.compile_hir_expr(nullable)
        };
        self.or_block_depth -= 1;
        result
    }

    fn compile_or_block_fallible_ptr(
        &mut self,
        fallback: &HirExpr,
        val: PointerValue<'ctx>,
        ok: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("or-block: no function")?;
        let ok_i1 = self.ok_i1(ok)?;
        let failed = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ok_i1,
                self.bool_ty().const_zero(),
                "or_fail",
            )
            .map_err(llvm_err)?;
        let fail_bb = self.context.append_basic_block(current_fn, "orblk_fail");
        let ok_bb = self.context.append_basic_block(current_fn, "orblk_ok");
        let merge_bb = self.context.append_basic_block(current_fn, "orblk_merge");
        self.builder
            .build_conditional_branch(failed, fail_bb, ok_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(ok_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(fail_bb);
        let fb = self.compile_hir_expr(fallback)?;
        let fb_ptr = match fb {
            TypedValue::Ptr(p) => p,
            _ => return Err("or-block fallback must be Ptr".into()),
        };
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.ptr_ty(), "or_ptr")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&val, ok_bb), (&fb_ptr, fail_bb)]);
        Ok(TypedValue::Ptr(phi.as_basic_value().into_pointer_value()))
    }

    fn compile_or_block_fallible_str(
        &mut self,
        fallback: &HirExpr,
        val: PointerValue<'ctx>,
        ok: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("or-block: no function")?;
        let ok_i1 = self.ok_i1(ok)?;
        let failed = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ok_i1,
                self.bool_ty().const_zero(),
                "or_fail",
            )
            .map_err(llvm_err)?;
        let fail_bb = self.context.append_basic_block(current_fn, "orblk_fail");
        let ok_bb = self.context.append_basic_block(current_fn, "orblk_ok");
        let merge_bb = self.context.append_basic_block(current_fn, "orblk_merge");
        self.builder
            .build_conditional_branch(failed, fail_bb, ok_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(ok_bb);
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(fail_bb);
        let fb = self.compile_hir_expr(fallback)?;
        let fb_ptr = match fb {
            TypedValue::Str(p) => p,
            _ => return Err("or-block fallback must be String".into()),
        };
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.ptr_ty(), "or_str")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&val, ok_bb), (&fb_ptr, fail_bb)]);
        Ok(TypedValue::Str(phi.as_basic_value().into_pointer_value()))
    }

    fn compile_json_ptr_fallible(
        &mut self,
        rt: &str,
        args: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
    ) -> Result<TypedValue<'ctx>, String> {
        let ptr = self.json_call_ptr(rt, args)?;
        let TypedValue::Ptr(p) = ptr else {
            return Err("json call expected Ptr".to_string());
        };
        let null = self.ptr_ty().const_null();
        let ok = self
            .builder
            .build_int_compare(IntPredicate::NE, p, null, "json_ok")
            .map_err(llvm_err)?;
        let ok_i1 = self.ok_i1(ok)?;
        self.branch_to_fail_if(ok_i1)?;
        Ok(TypedValue::FalliblePtr { val: p, ok: ok_i1 })
    }

    pub(crate) fn compile_json_parse_fallible(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let cstr = self.json_cstring_arg(arg)?;
        self.compile_json_ptr_fallible("action_json_parse", &[cstr.into()])
    }

    pub(crate) fn compile_json_get_fallible(
        &mut self,
        node: CallArg<'_>,
        key: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let np = self.json_ptr_arg(node)?;
        let kp = self.json_cstring_arg(key)?;
        self.compile_json_ptr_fallible("action_json_get", &[np.into(), kp.into()])
    }

    pub(crate) fn compile_json_get_idx_fallible(
        &mut self,
        node: CallArg<'_>,
        idx: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let np = self.json_ptr_arg(node)?;
        let iv = self.compile_call_arg(idx)?;
        let TypedValue::Int(idx_iv) = iv else {
            return Err("__jsonGetIdx: index must be Int".to_string());
        };
        self.compile_json_ptr_fallible("action_json_get_idx", &[np.into(), idx_iv.into()])
    }

    pub(crate) fn compile_read_line_fallible(&mut self) -> Result<TypedValue<'ctx>, String> {
        if self.module.get_function("action_read_line").is_none() {
            self.emit_read_line_runtime()?;
        }
        let cc = self.call_rt("action_read_line", &[])?;
        let result_struct = cc
            .try_as_basic_value()
            .basic()
            .ok_or("readLine failed")?
            .into_struct_value();
        let str_len = self
            .builder
            .build_extract_value(result_struct, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let str_ptr = self
            .builder
            .build_extract_value(result_struct, 1, "sptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ok = self
            .builder
            .build_extract_value(result_struct, 2, "ok")
            .map_err(llvm_err)?
            .into_int_value();
        let line_undef = self.string_type.get_undef();
        let line1 = self
            .builder
            .build_insert_value(line_undef, str_len, 0, "l_len")
            .map_err(llvm_err)?;
        let line_val = self
            .builder
            .build_insert_value(line1, str_ptr, 1, "l_ptr")
            .map_err(llvm_err)?;
        let fat_alloca = self
            .builder
            .build_alloca(self.string_type, "line")
            .map_err(llvm_err)?;
        self.builder
            .build_store(fat_alloca, line_val)
            .map_err(llvm_err)?;
        let ok_i1 = self.ok_i1(ok)?;
        self.branch_to_fail_if(ok_i1)?;
        Ok(TypedValue::FallibleStr {
            val: fat_alloca,
            ok: ok_i1,
        })
    }

    pub(crate) fn unwrap_fallible_value(
        &self,
        v: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match v {
            TypedValue::FallibleInt { val, .. } => Ok(TypedValue::Int(val)),
            TypedValue::FallibleFloat { val, .. } => Ok(TypedValue::Float(val)),
            TypedValue::FallibleStr { val, .. } => Ok(TypedValue::Str(val)),
            TypedValue::FalliblePtr { val, .. } => Ok(TypedValue::Ptr(val)),
            TypedValue::FallibleStruct { val, ty, .. } => Ok(TypedValue::Struct(val, ty)),
            other => Ok(other),
        }
    }

    pub(crate) fn compile_fn_or_fallback_return(
        &mut self,
        fallback: &HirExpr,
    ) -> Result<(), String> {
        let fb = self.compile_hir_expr(fallback)?;
        let fb = self.unwrap_fallible_value(fb)?;
        self.emit_scope_cleanup()?;
        self.build_return_for_value(&fb)
    }
}
