// Submodule: builtins_iter

use action_frontend::ast::*;
use inkwell::values::{IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn builtin_map(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // map(fn, list) or map(list) { lambda }
        let (fn_ptr, list_val) = if let Some(lam) = trailing {
            // map(list) { lambda }
            if args.len() != 1 {
                return Err("map with trailing lambda expects 1 argument (list)".to_string());
            }
            let lv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv)
        } else if args.len() == 2 {
            let fv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            (fv, lv)
        } else {
            return Err("map expects 2 arguments (fn, list)".to_string());
        };

        if let Some(result) = self.try_builtin_map_direct(fn_ptr, list_val)? {
            return Ok(result);
        }

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("map: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("map: second argument must be a list".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let input_list = list_struct;

        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "map_result")
            .map_err(llvm_err)?;

        let map_cc = self.call_rt("action_list_map_walk", &[input_list.into(), fn_ptr.into()])?;
        let result_bv = map_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_walk failed")?;
        self.builder
            .build_store(result_alloca, result_bv)
            .map_err(llvm_err)?;

        Ok(TypedValue::List(result_alloca))
    }

    /// Check if an expression is a `map(...)` call and return (map_fn_expr, inner_list_expr).
    /// Handles both `map(fn, list)` and `map(list) { fn }` syntax.
    fn extract_map_call_args(expr: &Expr) -> Option<(&Expr, &Expr)> {
        match expr {
            Expr::Call {
                func,
                args,
                trailing_lambda,
            } => {
                // func must be Ident("map")
                if let Expr::Ident(name) = func.as_ref() {
                    if name != "map" {
                        return None;
                    }
                } else {
                    return None;
                }
                match trailing_lambda {
                    Some(lam) => {
                        // map(list) { fn } → (fn, list)
                        if args.len() == 1 {
                            Some((lam.as_ref(), &args[0]))
                        } else {
                            None
                        }
                    }
                    None => {
                        // map(fn, list) → (fn, list)
                        if args.len() == 2 {
                            Some((&args[0], &args[1]))
                        } else {
                            None
                        }
                    }
                }
            }
            _ => None,
        }
    }

    pub(super) fn builtin_filter(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // Fused map+filter optimization: if the list argument is `map(...)`,
        // fuse map and filter into a single tree walk instead of creating an
        // intermediate list.
        let list_expr: &Expr = if let Some(_lam) = trailing {
            if args.len() != 1 {
                return Err("filter with trailing lambda expects 1 argument (list)".to_string());
            }
            &args[0]
        } else if args.len() == 2 {
            &args[1]
        } else {
            return Err("filter expects 2 arguments (fn, list)".to_string());
        };

        // Check if list_expr is a map call — extract map fn and inner list
        if let Some((map_fn_expr, inner_list_expr)) = Self::extract_map_call_args(list_expr) {
            // Compile filter fn, map fn, and inner list
            let filter_fn_val = if let Some(lam) = trailing {
                self.compile_expr(lam)?
            } else {
                self.compile_expr(&args[0])?
            };
            let map_fn_val = self.compile_expr(map_fn_expr)?;
            let inner_list_val = self.compile_expr(inner_list_expr)?;

            // NOTE: Do NOT call try_builtin_filter_direct here — it would
            // filter on the inner list values directly, skipping the map step.
            // The fused path must always use the fused runtime function.

            let filter_fn_ptr = match filter_fn_val {
                TypedValue::Fn(p, _) => p,
                TypedValue::Closure { fn_ptr, .. } => fn_ptr,
                _ => return Err("filter: first argument must be a function".to_string()),
            };
            let map_fn_ptr = match map_fn_val {
                TypedValue::Fn(p, _) => p,
                TypedValue::Closure { fn_ptr, .. } => fn_ptr,
                _ => return Err("fused map+filter: map function required".to_string()),
            };
            let inner_list_ptr = match inner_list_val {
                TypedValue::List(p) => p,
                _ => return Err("fused map+filter: list required".to_string()),
            };

            let inner_list_struct = self.load_list(inner_list_ptr)?;
            let result_alloca = self
                .builder
                .build_alloca(self.list_type, "mf_result")
                .map_err(llvm_err)?;

            let mf_cc = self.call_rt(
                "action_list_map_filter_walk",
                &[
                    inner_list_struct.into(),
                    map_fn_ptr.into(),
                    filter_fn_ptr.into(),
                ],
            )?;
            let result_bv = mf_cc
                .try_as_basic_value()
                .basic()
                .ok_or("map_filter_walk failed")?;
            self.builder
                .build_store(result_alloca, result_bv)
                .map_err(llvm_err)?;

            return Ok(TypedValue::List(result_alloca));
        }

        // Standard filter path
        let (fn_ptr, list_val) = if let Some(lam) = trailing {
            let lv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv)
        } else {
            let fv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            (fv, lv)
        };

        if let Some(result) = self.try_builtin_filter_direct(fn_ptr.clone(), list_val.clone())? {
            return Ok(result);
        }

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("filter: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("filter: second argument must be a list".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let input_list = list_struct;

        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "filter_result")
            .map_err(llvm_err)?;

        let filter_cc = self.call_rt(
            "action_list_filter_walk",
            &[input_list.into(), fn_ptr.into()],
        )?;
        let result_bv = filter_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter_walk failed")?;
        self.builder
            .build_store(result_alloca, result_bv)
            .map_err(llvm_err)?;

        Ok(TypedValue::List(result_alloca))
    }

    pub(super) fn builtin_fold(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // fold(fn, init, list) or fold(init, list) { lambda }
        let (fn_ptr, init_val, list_val) = if let Some(lam) = trailing {
            if args.len() != 2 {
                return Err(
                    "fold with trailing lambda expects 2 arguments (init, list)".to_string()
                );
            }
            let iv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            let fv = self.compile_expr(lam)?;
            (fv, iv, lv)
        } else if args.len() == 3 {
            let fv = self.compile_expr(&args[0])?;
            let iv = self.compile_expr(&args[1])?;
            let lv = self.compile_expr(&args[2])?;
            (fv, iv, lv)
        } else {
            return Err("fold expects 3 arguments (fn, init, list)".to_string());
        };

        if let Some(result) = self.try_builtin_fold_direct(fn_ptr, init_val, list_val)? {
            return Ok(result);
        }

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fold: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("fold: third argument must be a list".to_string()),
        };
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Err("fold: init must be an integer".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let input_list = list_struct;

        let fold_cc = self.call_rt(
            "action_list_fold_walk",
            &[input_list.into(), fn_ptr.into(), init_i64.into()],
        )?;
        let final_acc = fold_cc
            .try_as_basic_value()
            .basic()
            .ok_or("fold_walk failed")?
            .into_int_value();
        Ok(TypedValue::Int(final_acc))
    }

    /// flatMap(fn, list) = flatten(map(fn, list))
    pub(super) fn builtin_flat_map_list(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let mapped = self.builtin_map(args, trailing)?;
        match mapped {
            TypedValue::List(lp) => {
                let lv = self.load_list(lp)?;
                let cc = self.call_rt("action_list_flatten", &[lv.into()])?;
                let result = cc.try_as_basic_value().basic().ok_or("flatten failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "flatMap")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                Ok(TypedValue::List(alloca))
            }
            _ => Err("flatMap: map result must be a list".to_string()),
        }
    }

    pub(super) fn builtin_callback_list(
        &mut self,
        name: &str,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "any" => self.builtin_any(args, trailing),
            "all" => self.builtin_all(args, trailing),
            "find" => self.builtin_find(args, trailing),
            "findIndex" => self.builtin_find_index(args, trailing),
            "reduce" => self.builtin_reduce(args, trailing),
            "foldRight" => self.builtin_fold_right(args, trailing),
            "takeWhile" => self.builtin_take_while(args, trailing),
            "dropWhile" => self.builtin_drop_while(args, trailing),
            "sortedBy" => self.builtin_sorted_by(args, trailing),
            "partition" => self.builtin_partition(args, trailing),
            "count" => self.builtin_count(args, trailing),
            _ => Err(format!("Unknown callback list builtin: {}", name)),
        }
    }

    /// any(list, fn) or any(list) { lambda } -> Bool
    pub(super) fn builtin_any(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_val, list_val) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err("any with trailing lambda expects 1 argument (list)".to_string());
            }
            let lv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv)
        } else if args.len() == 2 {
            let fv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            (fv, lv)
        } else {
            return Err("any expects 2 arguments (fn, list)".to_string());
        };

        if let Some(result) = self.try_builtin_any_direct(fn_val, list_val)? {
            return Ok(result);
        }

        let fn_ptr = match fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("any: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("any: last argument must be a list".to_string()),
        };
        let input_list = self.load_list(list_ptr)?;

        let any_cc = self.call_rt("action_list_any_walk", &[input_list.into(), fn_ptr.into()])?;
        let res = any_cc
            .try_as_basic_value()
            .basic()
            .ok_or("any_walk failed")?
            .into_int_value();
        Ok(TypedValue::Bool(res))
    }

    /// all(list, fn) or all(list) { lambda } -> Bool
    pub(super) fn builtin_all(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_val, list_val) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err("all with trailing lambda expects 1 argument (list)".to_string());
            }
            let lv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv)
        } else if args.len() == 2 {
            let fv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            (fv, lv)
        } else {
            return Err("all expects 2 arguments (fn, list)".to_string());
        };

        if let Some(result) = self.try_builtin_all_direct(fn_val, list_val)? {
            return Ok(result);
        }

        let fn_ptr = match fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("all: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("all: last argument must be a list".to_string()),
        };
        let input_list = self.load_list(list_ptr)?;

        let all_cc = self.call_rt("action_list_all_walk", &[input_list.into(), fn_ptr.into()])?;
        let res = all_cc
            .try_as_basic_value()
            .basic()
            .ok_or("all_walk failed")?
            .into_int_value();
        Ok(TypedValue::Bool(res))
    }

    /// find(list, fn) or find(list) { lambda } -> Option<T>
    pub(super) fn builtin_find(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "find")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        // Allocate fat struct slot for found element
        let found_a = self
            .builder
            .build_alloca(self.string_type, "found")
            .map_err(llvm_err)?;
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "found_f")
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_flag_a, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "find_hdr");
        let bdy = self.context.append_basic_block(current_fn, "find_bdy");
        let found_bb = self.context.append_basic_block(current_fn, "find_found");
        let ext = self.context.append_basic_block(current_fn, "find_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "find_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_true, found_bb, hdr);
        self.builder.position_at_end(found_bb);
        self.builder
            .build_store(found_a, elem_val)
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_flag_a, self.bool_ty().const_int(1, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ext);
        self.builder.position_at_end(ext);
        // Build nullable String: set flag 0 + found value, or flag 1 (null).
        // InnerType defaults to Int — list elements are fat structs whose type
        // is only known at runtime. Fixing this requires adding element type info
        // to List/Map/Set TypedValue variants.
        self.build_nullable_str(found_a, found_flag_a)
    }

    /// findIndex(list, fn) or findIndex(list) { lambda } -> Option<Int>
    pub(super) fn builtin_find_index(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "findIndex")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let result_a = self.builder.build_alloca(i64, "fi_idx").map_err(llvm_err)?;
        self.builder
            .build_store(result_a, i64.const_int((-1i64) as u64, true))
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "fi_hdr");
        let bdy = self.context.append_basic_block(current_fn, "fi_bdy");
        let ext = self.context.append_basic_block(current_fn, "fi_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "fi_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        self.builder.build_store(result_a, iv).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let fi_hdr2 = self.context.append_basic_block(current_fn, "fi_chk");
        let _ = self.builder.build_conditional_branch(is_true, ext, fi_hdr2);
        self.builder.position_at_end(fi_hdr2);
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let found_idx = self
            .builder
            .build_load(i64, result_a, "found_idx")
            .map_err(llvm_err)?
            .into_int_value();
        let is_found = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                found_idx,
                i64.const_int(0, false),
                "is_found",
            )
            .map_err(llvm_err)?;
        // Build nullable Int: set flag 0 + found_idx, or flag 1 (null)
        self.build_nullable_int(found_idx, is_found)
    }

    /// reduce(list, fn) or reduce(list) { lambda } -> Option<T>
    pub(super) fn builtin_reduce(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "reduce")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let is_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, input_len, zero, "is_empty")
            .map_err(llvm_err)?;
        // Accumulator: fat {i64,ptr}
        let acc_a = self
            .builder
            .build_alloca(self.string_type, "reduce_acc")
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(i_a, one).map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        // Init: load first element into acc
        let init_bb = self.context.append_basic_block(current_fn, "reduce_init");
        let loop_hdr = self.context.append_basic_block(current_fn, "reduce_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "reduce_bdy");
        let loop_ext = self.context.append_basic_block(current_fn, "reduce_ext");
        let empty_bb = self.context.append_basic_block(current_fn, "reduce_empty");
        let merge_bb = self.context.append_basic_block(current_fn, "reduce_merge");
        let _ = self
            .builder
            .build_conditional_branch(is_empty, empty_bb, init_bb);
        // Init: load first element
        self.builder.position_at_end(init_bb);
        let input_list0 = self.load_list(list_ptr)?;
        let first = self.call_rt("action_list_get", &[input_list0.into(), zero.into()])?;
        let first_val = first
            .try_as_basic_value()
            .basic()
            .ok_or("list_get failed")?;
        self.builder
            .build_store(acc_a, first_val)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);
        // Loop
        self.builder.position_at_end(loop_hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);
        self.builder.position_at_end(loop_bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let acc_fat = self
            .builder
            .build_load(self.string_type, acc_a, "acc")
            .map_err(llvm_err)?;
        let acc_tag = self
            .builder
            .build_extract_value(acc_fat.into_struct_value(), 0, "acc_tag")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(
                fn_type,
                fn_ptr,
                &[acc_tag.into(), elem_tag.into()],
                "reduce_call",
            )
            .map_err(llvm_err)?;
        let new_acc = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        self.builder.build_store(acc_a, new_acc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);
        self.builder.position_at_end(loop_ext);
        let final_acc = self
            .builder
            .build_load(self.string_type, acc_a, "final_acc")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Empty: build None
        self.builder.position_at_end(empty_bb);
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge: build Option from fat struct or None
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.string_type, "reduce_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[
            (&final_acc, loop_ext),
            (&self.string_type.get_undef(), empty_bb),
        ]);
        let phi_val = phi.as_basic_value();
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "red_found")
            .map_err(llvm_err)?;
        let phi_flag = self
            .builder
            .build_phi(self.bool_ty(), "red_flag")
            .map_err(llvm_err)?;
        phi_flag.add_incoming(&[
            (&self.bool_ty().const_int(1, false), loop_ext),
            (&self.bool_ty().const_zero(), empty_bb),
        ]);
        self.builder
            .build_store(found_flag_a, phi_flag.as_basic_value())
            .map_err(llvm_err)?;
        let acc_alloca = self
            .builder
            .build_alloca(self.string_type, "red_acc_s")
            .map_err(llvm_err)?;
        self.builder
            .build_store(acc_alloca, phi_val)
            .map_err(llvm_err)?;
        // InnerType defaults to Int — accumulator is a fat struct whose type
        // is only known at runtime. See comment at builtin_find for details.
        self.build_nullable_str(acc_alloca, found_flag_a)
    }

    /// foldRight(list, init, fn) or foldRight(list, init) { lambda } -> T
    pub(super) fn builtin_fold_right(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr, init_val) = self.extract_fold_right_args(args, trailing)?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let acc_a = self.builder.build_alloca(i64, "fr_acc").map_err(llvm_err)?;
        self.builder
            .build_store(acc_a, init_val)
            .map_err(llvm_err)?;
        // Iterate backwards: i = len-1 down to 0
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        let start_i = self
            .builder
            .build_int_sub(input_len, one, "start_i")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, start_i).map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "fr_hdr");
        let bdy = self.context.append_basic_block(current_fn, "fr_bdy");
        let ext = self.context.append_basic_block(current_fn, "fr_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, iv, zero, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let acc = self
            .builder
            .build_load(i64, acc_a, "acc")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into(), acc.into()], "fr_call")
            .map_err(llvm_err)?;
        let new_acc_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let new_acc = if new_acc_bv.is_struct_value() {
            self.builder
                .build_extract_value(new_acc_bv.into_struct_value(), 0, "fr_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            new_acc_bv.into_int_value()
        };
        self.builder.build_store(acc_a, new_acc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_sub(iv, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let final_acc = self
            .builder
            .build_load(i64, acc_a, "final_acc")
            .map_err(llvm_err)?
            .into_int_value();
        Ok(TypedValue::Int(final_acc))
    }

    /// takeWhile(list, fn) or takeWhile(list) { lambda } -> List<T>
    pub(super) fn builtin_take_while(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "takeWhile")?;
        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        // Create result list
        let cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let res_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "tw_res")
            .map_err(llvm_err)?;
        self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "tw_hdr");
        let bdy = self.context.append_basic_block(current_fn, "tw_bdy");
        let ext = self.context.append_basic_block(current_fn, "tw_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "tw_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let push_bb = self.context.append_basic_block(current_fn, "tw_push");
        let _ = self.builder.build_conditional_branch(is_true, push_bb, ext);
        self.builder.position_at_end(push_bb);
        let rl = self
            .builder
            .build_load(self.list_type, res_a, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        self.builder
            .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        Ok(TypedValue::List(res_a))
    }

    /// dropWhile(list, fn) or dropWhile(list) { lambda } -> List<T>
    pub(super) fn builtin_drop_while(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "dropWhile")?;
        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let res_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "dw_res")
            .map_err(llvm_err)?;
        self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
        let dropping_a = self
            .builder
            .build_alloca(self.bool_ty(), "dropping")
            .map_err(llvm_err)?;
        self.builder
            .build_store(dropping_a, self.bool_ty().const_int(1, false))
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "dw_hdr");
        let bdy = self.context.append_basic_block(current_fn, "dw_bdy");
        let ext = self.context.append_basic_block(current_fn, "dw_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let dropping = self
            .builder
            .build_load(self.bool_ty(), dropping_a, "dropping")
            .map_err(llvm_err)?
            .into_int_value();
        let is_dropping = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                dropping,
                self.bool_ty().const_zero(),
                "is_dropping",
            )
            .map_err(llvm_err)?;
        // Only call predicate if still dropping
        let call_bb = self.context.append_basic_block(current_fn, "dw_call");
        let push_bb = self.context.append_basic_block(current_fn, "dw_push");
        let inc_bb = self.context.append_basic_block(current_fn, "dw_inc");
        let _ = self
            .builder
            .build_conditional_branch(is_dropping, call_bb, push_bb);
        // Call predicate
        self.builder.position_at_end(call_bb);
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "dw_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        // If true, still dropping, skip element (go to inc). If false, stop dropping, push element.
        let _ = self
            .builder
            .build_conditional_branch(is_true, inc_bb, push_bb);
        // Push element
        self.builder.position_at_end(push_bb);
        self.builder
            .build_store(dropping_a, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        let rl = self
            .builder
            .build_load(self.list_type, res_a, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        self.builder
            .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(inc_bb);
        // Increment
        self.builder.position_at_end(inc_bb);
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        Ok(TypedValue::List(res_a))
    }

    /// sortedBy(list, fn) or sortedBy(list) { lambda } -> List<T>
    pub(super) fn builtin_sorted_by(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "sortedBy")?;
        let list_struct = self.load_list(list_ptr)?;
        let cc = self.call_rt(
            "action_list_sorted_by",
            &[list_struct.into(), fn_ptr.into()],
        )?;
        let result = cc.try_as_basic_value().basic().ok_or("sortedBy failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "sb_res")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, result).map_err(llvm_err)?;
        Ok(TypedValue::List(alloca))
    }

    /// partition(list, fn) or partition(list) { lambda } -> (List<T>, List<T>)
    pub(super) fn builtin_partition(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "partition")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        // Create two result lists
        let cap = i64.const_int(4, false);
        let left_cc = self.call_rt("action_list_create", &[cap.into()])?;
        let left_bv = left_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create left")?;
        let left_a = self
            .builder
            .build_alloca(self.list_type, "part_left")
            .map_err(llvm_err)?;
        self.builder
            .build_store(left_a, left_bv)
            .map_err(llvm_err)?;
        let right_cc = self.call_rt("action_list_create", &[cap.into()])?;
        let right_bv = right_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create right")?;
        let right_a = self
            .builder
            .build_alloca(self.list_type, "part_right")
            .map_err(llvm_err)?;
        self.builder
            .build_store(right_a, right_bv)
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "part_hdr");
        let bdy = self.context.append_basic_block(current_fn, "part_bdy");
        let ext = self.context.append_basic_block(current_fn, "part_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "part_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let left_bb = self.context.append_basic_block(current_fn, "part_left");
        let right_bb = self.context.append_basic_block(current_fn, "part_right");
        let part_merge = self.context.append_basic_block(current_fn, "part_merge2");
        let _ = self
            .builder
            .build_conditional_branch(is_true, left_bb, right_bb);
        // Push to left
        self.builder.position_at_end(left_bb);
        let ll = self.load_list(left_a)?;
        let lp = self.call_rt("action_list_push", &[ll.into(), elem_val.into()])?;
        let lp_bv = lp.try_as_basic_value().basic().ok_or("push left")?;
        self.builder.build_store(left_a, lp_bv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(part_merge);
        // Push to right
        self.builder.position_at_end(right_bb);
        let rl = self.load_list(right_a)?;
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        let rp_bv = rp.try_as_basic_value().basic().ok_or("push right")?;
        self.builder.build_store(right_a, rp_bv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(part_merge);
        self.builder.position_at_end(part_merge);
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        // Build tuple struct: {list_type, list_type}
        let lv = self
            .builder
            .build_load(self.list_type, left_a, "lv")
            .map_err(llvm_err)?;
        let rv = self
            .builder
            .build_load(self.list_type, right_a, "rv")
            .map_err(llvm_err)?;
        let tuple_ty = self
            .context
            .struct_type(&[self.list_type.into(), self.list_type.into()], false);
        let undef = tuple_ty.get_undef();
        let t1 = self
            .builder
            .build_insert_value(undef, lv, 0, "t_l")
            .map_err(llvm_err)?;
        let t2 = self
            .builder
            .build_insert_value(t1, rv, 1, "t_r")
            .map_err(llvm_err)?;
        let alloca = self
            .builder
            .build_alloca(tuple_ty, "part_tuple")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, t2).map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, tuple_ty))
    }

    /// count(list, fn) or count(list) { lambda } -> Int
    pub(super) fn builtin_count(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "count")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let cnt_a = self.builder.build_alloca(i64, "cnt").map_err(llvm_err)?;
        self.builder
            .build_store(cnt_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "cnt_hdr");
        let bdy = self.context.append_basic_block(current_fn, "cnt_bdy");
        let ext = self.context.append_basic_block(current_fn, "cnt_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "cnt_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let one_or_zero = self
            .builder
            .build_int_z_extend(is_true, i64, "one_or_zero")
            .map_err(llvm_err)?;
        let cur = self
            .builder
            .build_load(i64, cnt_a, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let inc = self
            .builder
            .build_int_add(cur, one_or_zero, "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(cnt_a, inc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let result = self
            .builder
            .build_load(i64, cnt_a, "result")
            .map_err(llvm_err)?;
        Ok(TypedValue::Int(result.into_int_value()))
    }

    /// Helper: extract (fn_ptr, list_ptr) from args for callback-based list functions
    pub(super) fn extract_callback_args(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
        expected_args: usize,
        name: &str,
    ) -> Result<(PointerValue<'ctx>, PointerValue<'ctx>), String> {
        let (fn_expr, list_expr) = if let Some(lam) = trailing {
            if args.len() != expected_args {
                return Err(format!(
                    "{} with trailing lambda expects {} argument(s) (list)",
                    name, expected_args
                ));
            }
            let lv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv)
        } else if args.len() == expected_args + 1 {
            let fv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[expected_args])?;
            (fv, lv)
        } else {
            return Err(format!(
                "{} expects {} argument(s) (fn, list)",
                name,
                expected_args + 1
            ));
        };
        let fn_ptr = match fn_expr {
            TypedValue::Fn(p, _) => p,
            _ => return Err(format!("{}: first argument must be a function", name)),
        };
        let list_ptr = match list_expr {
            TypedValue::List(p) => p,
            _ => return Err(format!("{}: last argument must be a list", name)),
        };
        Ok((fn_ptr, list_ptr))
    }

    /// Helper: extract (fn_ptr, list_ptr, init_i64) for foldRight
    pub(super) fn extract_fold_right_args(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<(PointerValue<'ctx>, PointerValue<'ctx>, IntValue<'ctx>), String> {
        let (fn_expr, list_expr, init_expr) = if let Some(lam) = trailing {
            if args.len() != 2 {
                return Err(
                    "foldRight with trailing lambda expects 2 arguments (init, list)".to_string(),
                );
            }
            let iv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv, iv)
        } else if args.len() == 3 {
            let fv = self.compile_expr(&args[0])?;
            let iv = self.compile_expr(&args[1])?;
            let lv = self.compile_expr(&args[2])?;
            (fv, lv, iv)
        } else {
            return Err("foldRight expects 3 arguments (fn, init, list)".to_string());
        };
        let fn_ptr = match fn_expr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("foldRight: first argument must be a function".to_string()),
        };
        let list_ptr = match list_expr {
            TypedValue::List(p) => p,
            _ => return Err("foldRight: last argument must be a list".to_string()),
        };
        let init_val = match init_expr {
            TypedValue::Int(v) => v,
            _ => return Err("foldRight: init must be an integer".to_string()),
        };
        Ok((fn_ptr, list_ptr, init_val))
    }

    /// Callback-based map functions: mapFilter, mapMapValues, mapFold
    pub(super) fn builtin_callback_map(
        &mut self,
        name: &str,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "mapFilter" => self.builtin_map_filter(args, trailing),
            "mapMapValues" => self.builtin_map_map_values(args, trailing),
            "mapFold" => self.builtin_map_fold(args, trailing),
            _ => Err(format!("Unknown callback map builtin: {}", name)),
        }
    }
}
