// Submodule: builtins_stdlib_collection — List/Map/Set collection builtin functions
//
// Extracted from builtins_stdlib.rs.
//
// Submodule: builtins_stdlib

use crate::ast::*;
use inkwell::IntPredicate;
use inkwell::values::BasicValue;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn builtin_stdlib_collection(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "head" => {
                if args.len() != 1 {
                    return Err("head expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                        let list_val = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(list_val, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        // The nullable wraps the i64 tag of the fat element struct
                        let nullable_ty =
                            self.get_nullable_type(self.i64_ty().into(), "Nullable<Int>");
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "head_some");
                        let none_bb = self.context.append_basic_block(current_fn, "head_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "head_merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, none_bb, some_bb);
                        // Some: {flag=0, elem_tag} — extract i64 tag from fat elem
                        self.builder.position_at_end(some_bb);
                        let elem =
                            self.call_rt("action_list_get", &[list_val.into(), zero.into()])?;
                        let elem_bv = elem
                            .try_as_basic_value()
                            .basic()
                            .ok_or("get failed")?
                            .into_struct_value();
                        let elem_tag = self
                            .builder
                            .build_extract_value(elem_bv, 0, "elem_tag")
                            .map_err(llvm_err)?;
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_tag, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "head_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "head")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("head: argument must be a list".to_string()),
                }
            }
            "last" => {
                if args.len() != 1 {
                    return Err("last expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                        let list_val = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(list_val, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        let last_idx = self
                            .builder
                            .build_int_sub(len, self.i64_ty().const_int(1, false), "last_idx")
                            .map_err(llvm_err)?;
                        let nullable_ty =
                            self.get_nullable_type(self.i64_ty().into(), "Nullable<Int>");
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "last_some");
                        let none_bb = self.context.append_basic_block(current_fn, "last_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "last_merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, none_bb, some_bb);
                        // Some: {flag=0, elem_tag} — extract i64 tag from fat elem
                        self.builder.position_at_end(some_bb);
                        let elem =
                            self.call_rt("action_list_get", &[list_val.into(), last_idx.into()])?;
                        let elem_bv = elem
                            .try_as_basic_value()
                            .basic()
                            .ok_or("get failed")?
                            .into_struct_value();
                        let elem_tag = self
                            .builder
                            .build_extract_value(elem_bv, 0, "elem_tag")
                            .map_err(llvm_err)?;
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_tag, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "last_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "last")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("last: argument must be a list".to_string()),
                }
            }
            "get" => {
                if args.len() != 2 {
                    return Err("get expects 2 arguments (list, index)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let neg = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *iv, zero, "neg")
                            .map_err(llvm_err)?;
                        let ge_len = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, *iv, len, "ge_len")
                            .map_err(llvm_err)?;
                        let oob = self
                            .builder
                            .build_or(neg, ge_len, "oob")
                            .map_err(llvm_err)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "get_some");
                        let none_bb = self.context.append_basic_block(current_fn, "get_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "get_merge");
                        let _ = self.builder.build_conditional_branch(oob, none_bb, some_bb);
                        // Some: {flag=0, elem} — value inlined, no heap alloc
                        self.builder.position_at_end(some_bb);
                        let elem = self.call_rt("action_list_get", &[lv.into(), (*iv).into()])?;
                        let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                        let nullable_ty =
                            self.get_nullable_type(self.string_type.into(), "Nullable<Str>");
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_bv, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "get_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "get")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("get: first argument must be a list, second an Int".to_string()),
                }
            }
            "remove" => {
                if args.len() != 2 {
                    return Err("remove expects 2 arguments (list, index)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let result =
                            self.call_rt("action_list_remove", &[lv.into(), (*iv).into()])?;
                        let rv = result.try_as_basic_value().basic().ok_or("remove failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "remove_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, rv).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("remove expects (List, Int)".to_string()),
                }
            }
            "reverse" => {
                if args.len() != 1 {
                    return Err("reverse expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_reverse", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("reverse failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "rev")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("reverse: argument must be a list".to_string()),
                }
            }
            "contains" => {
                if args.len() != 2 {
                    return Err("contains expects 2 arguments (list, element)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let elem_val = self.compile_expr(&args[1])?;
                match (&list_val, &elem_val) {
                    (TypedValue::List(lp), _) => {
                        let lv = self.load_list(*lp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let cc = self.call_rt("action_list_contains", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("contains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    (TypedValue::Set(sp), _) => {
                        let lv = self.load_list(*sp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let cc = self.call_rt("action_map_contains", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("contains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("contains: first argument must be a list or set".to_string()),
                }
            }
            "containsKey" => {
                if args.len() != 2 {
                    return Err("containsKey expects 2 arguments (map, key)".to_string());
                }
                let map_val = self.compile_expr(&args[0])?;
                let key_val = self.compile_expr(&args[1])?;
                match &map_val {
                    TypedValue::Map(mp) => {
                        let lv = self.load_list(*mp)?;
                        let key_fat = self.to_fat_struct(&key_val)?;
                        let cc =
                            self.call_rt("action_map_contains", &[lv.into(), key_fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("map_contains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("containsKey: first argument must be a map".to_string()),
                }
            }
            "prepend" => {
                if args.len() != 2 {
                    return Err("prepend expects 2 arguments (element, list)".to_string());
                }
                let elem_val = self.compile_expr(&args[0])?;
                let list_val = self.compile_expr(&args[1])?;
                match list_val {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len_bv = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let new_cap = self
                            .builder
                            .build_int_add(len_bv, self.i64_ty().const_int(4, false), "new_cap")
                            .map_err(llvm_err)?;
                        let new_list = self.call_rt("action_list_create", &[new_cap.into()])?;
                        let new_list_bv = new_list
                            .try_as_basic_value()
                            .basic()
                            .ok_or("create failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "prepend")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, new_list_bv)
                            .map_err(llvm_err)?;
                        // Push element first
                        let fat = self.to_fat_struct(&elem_val)?;
                        let lv2 = self.load_list(alloca)?;
                        let pushed1 =
                            self.call_rt("action_list_push", &[lv2.into(), fat.into()])?;
                        let pb1 = pushed1.try_as_basic_value().basic().ok_or("push1 failed")?;
                        self.builder.build_store(alloca, pb1).map_err(llvm_err)?;
                        // Then push all original elements
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let entry_block = current_fn.get_last_basic_block().unwrap();
                        let loop_bb = self.context.append_basic_block(current_fn, "prepend_loop");
                        let done_bb = self.context.append_basic_block(current_fn, "prepend_done");
                        let _ = self.builder.build_unconditional_branch(loop_bb);
                        self.builder.position_at_end(loop_bb);
                        let i = self
                            .builder
                            .build_phi(self.i64_ty(), "pp_i")
                            .map_err(llvm_err)?;
                        let lv_orig = self.load_list(lp)?;
                        let lv_cur = self.load_list(alloca)?;
                        let elem = self.call_rt(
                            "action_list_get",
                            &[lv_orig.into(), i.as_basic_value().into_int_value().into()],
                        )?;
                        let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                        let pushed =
                            self.call_rt("action_list_push", &[lv_cur.into(), elem_bv.into()])?;
                        let pb = pushed.try_as_basic_value().basic().ok_or("push2 failed")?;
                        self.builder.build_store(alloca, pb).map_err(llvm_err)?;
                        let ni = self
                            .builder
                            .build_int_add(
                                i.as_basic_value().into_int_value(),
                                self.i64_ty().const_int(1, false),
                                "pp_ni",
                            )
                            .map_err(llvm_err)?;
                        let done_cond = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, ni, len_bv, "pp_done")
                            .map_err(llvm_err)?;
                        let loop_block = self.builder.get_insert_block().unwrap();
                        i.add_incoming(&[
                            (&self.i64_ty().const_int(0, false), entry_block),
                            (&ni, loop_block),
                        ]);
                        let _ = self
                            .builder
                            .build_conditional_branch(done_cond, done_bb, loop_bb);
                        self.builder.position_at_end(done_bb);
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("prepend: second argument must be a list".to_string()),
                }
            }
            "take" => {
                if args.len() != 2 {
                    return Err("take expects 2 arguments (list, n)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let n_val = self.compile_expr(&args[1])?;
                match (&list_val, &n_val) {
                    (TypedValue::List(lp), TypedValue::Int(nv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_take", &[lv.into(), (*nv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("take failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "take")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("take: first argument must be a list, second an Int".to_string()),
                }
            }
            "drop" => {
                if args.len() != 2 {
                    return Err("drop expects 2 arguments (list, n)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let n_val = self.compile_expr(&args[1])?;
                match (&list_val, &n_val) {
                    (TypedValue::List(lp), TypedValue::Int(nv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_drop", &[lv.into(), (*nv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("drop failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "drop")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("drop: first argument must be a list, second an Int".to_string()),
                }
            }
            "range" => {
                if args.len() != 2 {
                    return Err("range expects 2 arguments (start, end)".to_string());
                }
                let start = self.compile_expr(&args[0])?;
                let end = self.compile_expr(&args[1])?;
                match (&start, &end) {
                    (TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let cc =
                            self.call_rt("action_list_range", &[(*sv).into(), (*ev).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("range failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "range")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("range: arguments must be Int".to_string()),
                }
            }
            "repeat" => {
                if args.len() != 2 {
                    return Err("repeat expects 2 arguments (value, count)".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                let count = self.compile_expr(&args[1])?;
                match count {
                    TypedValue::Int(cv) => {
                        let cap = self.i64_ty().const_int(4, false);
                        let new_list = self.call_rt("action_list_create", &[cap.into()])?;
                        let new_list_bv = new_list
                            .try_as_basic_value()
                            .basic()
                            .ok_or("create failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "repeat")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, new_list_bv)
                            .map_err(llvm_err)?;
                        let fat = self.to_fat_struct(&val)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let entry_block = current_fn.get_last_basic_block().unwrap();
                        let loop_bb = self.context.append_basic_block(current_fn, "repeat_loop");
                        let done_bb = self.context.append_basic_block(current_fn, "repeat_done");
                        let _ = self.builder.build_unconditional_branch(loop_bb);
                        self.builder.position_at_end(loop_bb);
                        let i = self
                            .builder
                            .build_phi(self.i64_ty(), "rep_i")
                            .map_err(llvm_err)?;
                        let lv = self.load_list(alloca)?;
                        let pushed = self.call_rt("action_list_push", &[lv.into(), fat.into()])?;
                        let pb = pushed.try_as_basic_value().basic().ok_or("push failed")?;
                        self.builder.build_store(alloca, pb).map_err(llvm_err)?;
                        let ni = self
                            .builder
                            .build_int_add(
                                i.as_basic_value().into_int_value(),
                                self.i64_ty().const_int(1, false),
                                "rep_ni",
                            )
                            .map_err(llvm_err)?;
                        let done_cond = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, ni, cv, "rep_done")
                            .map_err(llvm_err)?;
                        let loop_block = self.builder.get_insert_block().unwrap();
                        i.add_incoming(&[
                            (&self.i64_ty().const_int(0, false), entry_block),
                            (&ni, loop_block),
                        ]);
                        let _ = self
                            .builder
                            .build_conditional_branch(done_cond, done_bb, loop_bb);
                        self.builder.position_at_end(done_bb);
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("repeat: second argument must be Int".to_string()),
                }
            }
            "tail" => {
                if args.len() != 1 {
                    return Err("tail expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
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
                        self.build_nullable_list(result, is_empty)
                    }
                    _ => Err("tail: argument must be a list".to_string()),
                }
            }
            "zip" => {
                if args.len() != 2 {
                    return Err("zip expects 2 arguments (list1, list2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::List(lp1), TypedValue::List(lp2)) => {
                        let lv1 = self.load_list(*lp1)?;
                        let lv2 = self.load_list(*lp2)?;
                        let cc = self.call_rt("action_list_zip", &[lv1.into(), lv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("zip failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "zip")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("zip: arguments must be lists".to_string()),
                }
            }
            "insert" => {
                if args.len() != 3 {
                    return Err("insert expects 3 arguments (list, index, elem)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
                let elem_val = self.compile_expr(&args[2])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let result = self.call_rt(
                            "action_list_insert",
                            &[lv.into(), (*iv).into(), fat.into()],
                        )?;
                        let rv = result.try_as_basic_value().basic().ok_or("insert failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "insert_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, rv).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("insert expects (List, Int, Any)".to_string()),
                }
            }
            "init" => {
                if args.len() != 1 {
                    return Err("init expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
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
                        self.build_nullable_list(result, is_empty)
                    }
                    _ => Err("init: argument must be a list".to_string()),
                }
            }
            "setToList" => {
                if args.len() != 1 {
                    return Err("setToList expects 1 argument (set)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Set(p) => Ok(TypedValue::List(p)),
                    _ => Err("setToList: argument must be a set".to_string()),
                }
            }
            "randChoice" => {
                if args.len() != 1 {
                    return Err("randChoice expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let empty = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                len,
                                self.i64_ty().const_int(0, false),
                                "empty",
                            )
                            .map_err(llvm_err)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .unwrap()
                            .get_parent()
                            .unwrap();
                        let has_elem = self.context.append_basic_block(current_fn, "has_elem");
                        let no_elem = self.context.append_basic_block(current_fn, "no_elem");
                        let merge = self.context.append_basic_block(current_fn, "merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, no_elem, has_elem);
                        // No element: return None (tag=0)
                        self.builder.position_at_end(no_elem);
                        let none_fat = self.string_type.get_undef();
                        let none1 = self
                            .builder
                            .build_insert_value(
                                none_fat,
                                self.i64_ty().const_int(0, false),
                                0,
                                "none_tag",
                            )
                            .map_err(llvm_err)?;
                        let none2 = self
                            .builder
                            .build_insert_value(none1, self.ptr_ty().const_zero(), 1, "none_data")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge);
                        let none_block = self.builder.get_insert_block().unwrap();
                        // Has element: pick random index
                        self.builder.position_at_end(has_elem);
                        let idx = self
                            .builder
                            .build_int_sub(len, self.i64_ty().const_int(1, false), "max_idx")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt(
                            "action_rand_int",
                            &[self.i64_ty().const_int(0, false).into(), idx.into()],
                        )?;
                        let ri = cc.try_as_basic_value().unwrap_basic().into_int_value();
                        let data = self
                            .builder
                            .build_extract_value(lv, 0, "data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let ep = unsafe {
                            self.builder
                                .build_gep(self.string_type, data, &[ri], "ep")
                                .map_err(llvm_err)
                        }?;
                        let elem = self
                            .builder
                            .build_load(self.string_type, ep, "elem")
                            .map_err(llvm_err)?
                            .into_struct_value();
                        // Wrap in Some: tag=1, data=ptr to elem copy
                        let malloc = self.module.get_function("malloc").unwrap();
                        let some_ptr = self
                            .builder
                            .build_call(
                                malloc,
                                &[self.i64_ty().const_int(16, false).into()],
                                "some",
                            )
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        self.builder.build_store(some_ptr, elem).map_err(llvm_err)?;
                        let some_fat = self.string_type.get_undef();
                        let some1 = self
                            .builder
                            .build_insert_value(
                                some_fat,
                                self.i64_ty().const_int(1, false),
                                0,
                                "some_tag",
                            )
                            .map_err(llvm_err)?;
                        let some2 = self
                            .builder
                            .build_insert_value(some1, some_ptr, 1, "some_data")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge);
                        let some_block = self.builder.get_insert_block().unwrap();
                        // Merge
                        self.builder.position_at_end(merge);
                        let phi = self
                            .builder
                            .build_phi(self.string_type, "choice")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[
                            (&none2.as_basic_value_enum(), none_block),
                            (&some2.as_basic_value_enum(), some_block),
                        ]);
                        // Return as fat struct (Tag=EnumKind(3), data=ptr to fat value)
                        let opt_alloca = self
                            .builder
                            .build_alloca(self.string_type, "opt")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(opt_alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::List(opt_alloca)) // Reuse List type for the result
                    }
                    _ => Err("randChoice: argument must be a list".to_string()),
                }
            }
            "withIndex" => {
                if args.len() != 1 {
                    return Err("withIndex expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_with_index", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("withIndex failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "wi")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("withIndex: argument must be a list".to_string()),
                }
            }
            "unique" => {
                if args.len() != 1 {
                    return Err("unique expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_unique", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("unique failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "unique")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("unique: argument must be a list".to_string()),
                }
            }
            "slice" => {
                if args.len() != 3 {
                    return Err("slice expects 3 arguments (collection, start, end)".to_string());
                }
                let coll_v = self.compile_expr(&args[0])?;
                let start_v = self.compile_expr(&args[1])?;
                let end_v = self.compile_expr(&args[2])?;
                match (&coll_v, &start_v, &end_v) {
                    // slice(List<T>, Int, Int) -> List<T>  with [start, end) semantics
                    (TypedValue::List(lp), TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt(
                            "action_list_slice",
                            &[lv.into(), (*sv).into(), (*ev).into()],
                        )?;
                        let result = cc.try_as_basic_value().basic().ok_or("slice failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "slice")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    // slice(String, Int, Int) -> String  with [start, end) semantics
                    (TypedValue::Str(sp), TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let str_val = self.load_string(*sp)?;
                        let len = self
                            .builder
                            .build_int_sub(*ev, *sv, "slice_len")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt(
                            "action_string_substring",
                            &[str_val.into(), (*sv).into(), len.into()],
                        )?;
                        let result = cc.try_as_basic_value().basic().ok_or("slice failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "slice_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err(
                        "slice: first argument must be a list or string, second and third Int"
                            .to_string(),
                    ),
                }
            }
            "flatten" => {
                if args.len() != 1 {
                    return Err("flatten expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_flatten", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("flatten failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "flatten")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("flatten: argument must be a list".to_string()),
                }
            }
            "splitAt" => {
                if args.len() != 2 {
                    return Err("splitAt expects 2 arguments (list, index)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let idx_v = self.compile_expr(&args[1])?;
                match (&list_v, &idx_v) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc =
                            self.call_rt("action_list_split_at", &[lv.into(), (*iv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("splitAt failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "splitAt")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("splitAt: first argument must be a list, second Int".to_string()),
                }
            }
            "chunks" => {
                if args.len() != 2 {
                    return Err("chunks expects 2 arguments (list, size)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let size_v = self.compile_expr(&args[1])?;
                match (&list_v, &size_v) {
                    (TypedValue::List(lp), TypedValue::Int(sv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_chunks", &[lv.into(), (*sv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("chunks failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "chunks")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("chunks: first argument must be a list, second Int".to_string()),
                }
            }
            "windows" => {
                if args.len() != 2 {
                    return Err("windows expects 2 arguments (list, size)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let size_v = self.compile_expr(&args[1])?;
                match (&list_v, &size_v) {
                    (TypedValue::List(lp), TypedValue::Int(sv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_windows", &[lv.into(), (*sv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("windows failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "windows")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("windows: first argument must be a list, second Int".to_string()),
                }
            }
            "mapKeys" => {
                if args.len() != 1 {
                    return Err("mapKeys expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_keys", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapKeys failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "keys")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapKeys: argument must be a map".to_string()),
                }
            }
            "mapValues" => {
                if args.len() != 1 {
                    return Err("mapValues expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_values", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapValues failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "values")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapValues: argument must be a map".to_string()),
                }
            }
            "mapEntries" => {
                if args.len() != 1 {
                    return Err("mapEntries expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_entries", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapEntries failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "entries")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapEntries: argument must be a map".to_string()),
                }
            }
            "mapUnion" => {
                if args.len() != 2 {
                    return Err("map.union expects 2 arguments (map1, map2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Map(mp1), TypedValue::Map(mp2)) => {
                        let mv1 = self.load_list(*mp1)?;
                        let mv2 = self.load_list(*mp2)?;
                        let cc = self.call_rt("action_map_union", &[mv1.into(), mv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("map.union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "mapUnion")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Map(alloca))
                    }
                    _ => Err("map.union: arguments must be maps".to_string()),
                }
            }
            "setUnion" => {
                if args.len() != 2 {
                    return Err("set.union expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc = self.call_rt("action_set_union", &[sv1.into(), sv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("set.union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "union")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.union: arguments must be sets".to_string()),
                }
            }
            "setIntersection" => {
                if args.len() != 2 {
                    return Err("set.intersection expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc =
                            self.call_rt("action_set_intersection", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.intersection failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "intersection")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.intersection: arguments must be sets".to_string()),
                }
            }
            "setDifference" => {
                if args.len() != 2 {
                    return Err("set.difference expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc =
                            self.call_rt("action_set_difference", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.difference failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "difference")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.difference: arguments must be sets".to_string()),
                }
            }
            "setIsSubset" => {
                if args.len() != 2 {
                    return Err("set.isSubset expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc = self.call_rt("action_set_is_subset", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.isSubset failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("set.isSubset: arguments must be sets".to_string()),
                }
            }
            "randShuffle" => {
                if args.len() != 1 {
                    return Err("randShuffle expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_rand_shuffle", &[lv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("randShuffle failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "shuffled")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("randShuffle: argument must be a list".to_string()),
                }
            }
            "sorted" => {
                if args.len() != 1 {
                    return Err("sorted expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_sorted", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("sorted failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "sorted")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("sorted: argument must be a list".to_string()),
                }
            }
            "sum" => {
                if args.len() != 1 {
                    return Err("sum expects 1 argument (list)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("sum: argument must be a list".to_string()),
                };
                let list = self.load_list(list_ptr)?;
                let len = self.list_len_val(list)?;
                let data = self.list_data_ptr(list)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                let sum_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "sum")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(sum_a, self.i64_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                let i_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "i")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(i_a, self.i64_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                let hdr = self.context.append_basic_block(current, "sum_hdr");
                let bdy = self.context.append_basic_block(current, "sum_bdy");
                let ext = self.context.append_basic_block(current, "sum_ext");
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(hdr);
                let iv = self
                    .builder
                    .build_load(self.i64_ty(), i_a, "iv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, iv, len, "cond")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(cond, bdy, ext);
                self.builder.position_at_end(bdy);
                let ep = unsafe {
                    self.builder
                        .build_gep(self.string_type, data, &[iv], "ep")
                        .map_err(llvm_err)
                }?;
                let ev = self
                    .builder
                    .build_load(self.string_type, ep, "ev")
                    .map_err(llvm_err)?;
                let etag = self
                    .builder
                    .build_extract_value(ev.into_struct_value(), 0, "etag")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur = self
                    .builder
                    .build_load(self.i64_ty(), sum_a, "cur")
                    .map_err(llvm_err)?
                    .into_int_value();
                let new_sum = self
                    .builder
                    .build_int_add(cur, etag, "new_sum")
                    .map_err(llvm_err)?;
                self.builder.build_store(sum_a, new_sum).map_err(llvm_err)?;
                let ni = self
                    .builder
                    .build_int_add(iv, self.i64_ty().const_int(1, false), "ni")
                    .map_err(llvm_err)?;
                self.builder.build_store(i_a, ni).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(ext);
                let result = self
                    .builder
                    .build_load(self.i64_ty(), sum_a, "result")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Int(result.into_int_value()))
            }
            "product" => {
                if args.len() != 1 {
                    return Err("product expects 1 argument (list)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("product: argument must be a list".to_string()),
                };
                let list = self.load_list(list_ptr)?;
                let len = self.list_len_val(list)?;
                let data = self.list_data_ptr(list)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                let prod_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "prod")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(prod_a, self.i64_ty().const_int(1, false))
                    .map_err(llvm_err)?;
                let i_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "i")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(i_a, self.i64_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                let hdr = self.context.append_basic_block(current, "prod_hdr");
                let bdy = self.context.append_basic_block(current, "prod_bdy");
                let ext = self.context.append_basic_block(current, "prod_ext");
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(hdr);
                let iv = self
                    .builder
                    .build_load(self.i64_ty(), i_a, "iv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, iv, len, "cond")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(cond, bdy, ext);
                self.builder.position_at_end(bdy);
                let ep = unsafe {
                    self.builder
                        .build_gep(self.string_type, data, &[iv], "ep")
                        .map_err(llvm_err)
                }?;
                let ev = self
                    .builder
                    .build_load(self.string_type, ep, "ev")
                    .map_err(llvm_err)?;
                let etag = self
                    .builder
                    .build_extract_value(ev.into_struct_value(), 0, "etag")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur = self
                    .builder
                    .build_load(self.i64_ty(), prod_a, "cur")
                    .map_err(llvm_err)?
                    .into_int_value();
                let new_prod = self
                    .builder
                    .build_int_mul(cur, etag, "new_prod")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(prod_a, new_prod)
                    .map_err(llvm_err)?;
                let ni = self
                    .builder
                    .build_int_add(iv, self.i64_ty().const_int(1, false), "ni")
                    .map_err(llvm_err)?;
                self.builder.build_store(i_a, ni).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(ext);
                let result = self
                    .builder
                    .build_load(self.i64_ty(), prod_a, "result")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Int(result.into_int_value()))
            }
            "digits" => {
                // digits(n) -> List<Int>: decimal digits of abs(n), MSD first. 0 -> [0].
                if args.len() != 1 {
                    return Err("digits expects 1 argument (int)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let n = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("digits: argument must be an int".to_string()),
                };
                let ten = self.i64_ty().const_int(10, false);
                let zero = self.i64_ty().const_int(0, false);
                let one = self.i64_ty().const_int(1, false);
                // abs_n = n < 0 ? -n : n
                let neg = self.builder.build_int_neg(n, "neg").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, n, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_n = self
                    .builder
                    .build_select(is_neg, neg, n, "abs_n")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_zero = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, n, zero, "is0")
                    .map_err(llvm_err)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                // Count digits via repeated division
                let dc_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "dc")
                    .map_err(llvm_err)?;
                self.builder.build_store(dc_a, zero).map_err(llvm_err)?;
                let tmp_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "tmp")
                    .map_err(llvm_err)?;
                self.builder.build_store(tmp_a, abs_n).map_err(llvm_err)?;
                let cnt_hdr = self.context.append_basic_block(current, "dc_hdr");
                let cnt_bdy = self.context.append_basic_block(current, "dc_bdy");
                let cnt_ext = self.context.append_basic_block(current, "dc_ext");
                let _ = self.builder.build_unconditional_branch(cnt_hdr);
                self.builder.position_at_end(cnt_hdr);
                let tv = self
                    .builder
                    .build_load(self.i64_ty(), tmp_a, "tv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let gt0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGT, tv, zero, "gt0")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(gt0, cnt_bdy, cnt_ext);
                self.builder.position_at_end(cnt_bdy);
                let dv = self
                    .builder
                    .build_load(self.i64_ty(), dc_a, "dv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let nd = self
                    .builder
                    .build_int_add(dv, one, "nd")
                    .map_err(llvm_err)?;
                self.builder.build_store(dc_a, nd).map_err(llvm_err)?;
                let nt = self
                    .builder
                    .build_int_signed_div(tv, ten, "nt")
                    .map_err(llvm_err)?;
                self.builder.build_store(tmp_a, nt).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(cnt_hdr);
                self.builder.position_at_end(cnt_ext);
                let ndigits = self
                    .builder
                    .build_load(self.i64_ty(), dc_a, "nd")
                    .map_err(llvm_err)?
                    .into_int_value();
                // 0 -> 1 digit
                let final_dc = self
                    .builder
                    .build_select(is_zero, one, ndigits, "fdc")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Create result list with capacity = final_dc
                let cc = self.call_rt("action_list_create", &[final_dc.into()])?;
                let res_bv = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("list_create failed")?;
                let res_a = self
                    .builder
                    .build_alloca(self.list_type, "digits_res")
                    .map_err(llvm_err)?;
                self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
                // Compute 10^(ndigits-1) iteratively
                let pow_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "pow10")
                    .map_err(llvm_err)?;
                self.builder.build_store(pow_a, one).map_err(llvm_err)?;
                let pi_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "pi")
                    .map_err(llvm_err)?;
                self.builder.build_store(pi_a, one).map_err(llvm_err)?;
                let pow_hdr = self.context.append_basic_block(current, "pow_hdr");
                let pow_bdy = self.context.append_basic_block(current, "pow_bdy");
                let pow_ext = self.context.append_basic_block(current, "pow_ext");
                let _ = self.builder.build_unconditional_branch(pow_hdr);
                self.builder.position_at_end(pow_hdr);
                let piv = self
                    .builder
                    .build_load(self.i64_ty(), pi_a, "piv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let plt = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, piv, final_dc, "plt")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(plt, pow_bdy, pow_ext);
                self.builder.position_at_end(pow_bdy);
                let pv = self
                    .builder
                    .build_load(self.i64_ty(), pow_a, "pv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let npv = self
                    .builder
                    .build_int_mul(pv, ten, "npv")
                    .map_err(llvm_err)?;
                self.builder.build_store(pow_a, npv).map_err(llvm_err)?;
                let npi = self
                    .builder
                    .build_int_add(piv, one, "npi")
                    .map_err(llvm_err)?;
                self.builder.build_store(pi_a, npi).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(pow_hdr);
                self.builder.position_at_end(pow_ext);
                let pow10 = self
                    .builder
                    .build_load(self.i64_ty(), pow_a, "pow10")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Extract digits MSD-first: for i in 0..ndigits { d = (abs_n / pow10) % 10; push; pow10 /= 10 }
                self.builder.build_store(tmp_a, abs_n).map_err(llvm_err)?;
                let di_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "di")
                    .map_err(llvm_err)?;
                self.builder.build_store(di_a, zero).map_err(llvm_err)?;
                let p10_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "p10")
                    .map_err(llvm_err)?;
                self.builder.build_store(p10_a, pow10).map_err(llvm_err)?;
                let fill_hdr = self.context.append_basic_block(current, "fill_hdr");
                let fill_bdy = self.context.append_basic_block(current, "fill_bdy");
                let fill_ext = self.context.append_basic_block(current, "fill_ext");
                let _ = self.builder.build_unconditional_branch(fill_hdr);
                self.builder.position_at_end(fill_hdr);
                let div = self
                    .builder
                    .build_load(self.i64_ty(), di_a, "div")
                    .map_err(llvm_err)?
                    .into_int_value();
                let flt = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, div, final_dc, "flt")
                    .map_err(llvm_err)?;
                let _ = self
                    .builder
                    .build_conditional_branch(flt, fill_bdy, fill_ext);
                self.builder.position_at_end(fill_bdy);
                let cur_pow = self
                    .builder
                    .build_load(self.i64_ty(), p10_a, "cur_pow")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur_n = self
                    .builder
                    .build_load(self.i64_ty(), tmp_a, "cur_n")
                    .map_err(llvm_err)?
                    .into_int_value();
                let q = self
                    .builder
                    .build_int_signed_div(cur_n, cur_pow, "q")
                    .map_err(llvm_err)?;
                let digit = self
                    .builder
                    .build_int_signed_rem(q, ten, "digit")
                    .map_err(llvm_err)?;
                // Build fat struct {digit, null} and push
                let undef = self.string_type.get_undef();
                let d1 = self
                    .builder
                    .build_insert_value(undef, digit, 0, "d1")
                    .map_err(llvm_err)?;
                let d2 = self
                    .builder
                    .build_insert_value(d1, self.ptr_ty().const_zero(), 1, "d2")
                    .map_err(llvm_err)?;
                let rl = self
                    .builder
                    .build_load(self.list_type, res_a, "rl")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let rp = self.call_rt(
                    "action_list_push",
                    &[rl.into(), d2.as_basic_value_enum().into()],
                )?;
                self.builder
                    .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
                    .map_err(llvm_err)?;
                // Advance: i++, pow10 /= 10
                let ndi = self
                    .builder
                    .build_int_add(div, one, "ndi")
                    .map_err(llvm_err)?;
                self.builder.build_store(di_a, ndi).map_err(llvm_err)?;
                let np10 = self
                    .builder
                    .build_int_signed_div(cur_pow, ten, "np10")
                    .map_err(llvm_err)?;
                self.builder.build_store(p10_a, np10).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(fill_hdr);
                self.builder.position_at_end(fill_ext);
                Ok(TypedValue::List(res_a))
            }
            _ => Err(format!("Unknown collection builtin: {}", name)),
        }
    }
}
