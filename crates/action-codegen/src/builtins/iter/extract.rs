//! Iterator builtins: map, filter, fold, find (R4-1).

use inkwell::values::{IntValue, PointerValue};
use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// Helper: extract (fn_ptr, list_ptr) from args for callback-based list functions
    pub(crate) fn extract_callback_args(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
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
            let lv = self.compile_call_arg(args[0])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, lv)
        } else if args.len() == expected_args + 1 {
            let a0 = self.compile_call_arg(args[0])?;
            let a1 = self.compile_call_arg(args[1])?;
            let (fn_expr, list_expr) = match (&a0, &a1) {
                (TypedValue::List(_), _) => (a1, a0),
                (_, TypedValue::List(_)) => (a0, a1),
                _ => {
                    return Err(format!("{name} expects one list and one function argument"));
                }
            };
            let fn_ptr = match fn_expr {
                TypedValue::Fn(p, _) => p,
                TypedValue::Closure { fn_ptr, .. } => fn_ptr,
                _ => return Err(format!("{name}: callback must be a function")),
            };
            let list_ptr = match list_expr {
                TypedValue::List(p) => p,
                _ => return Err(format!("{name}: last argument must be a list")),
            };
            return Ok((fn_ptr, list_ptr));
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
    pub(crate) fn extract_fold_right_args(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<(PointerValue<'ctx>, PointerValue<'ctx>, IntValue<'ctx>), String> {
        let (fn_expr, list_expr, init_expr) = if let Some(lam) = trailing {
            if args.len() != 2 {
                return Err(
                    "foldRight with trailing lambda expects 2 arguments (init, list)".to_string(),
                );
            }
            let iv = self.compile_call_arg(args[0])?;
            let lv = self.compile_call_arg(args[1])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, lv, iv)
        } else if args.len() == 3 {
            let fv = self.compile_call_arg(args[0])?;
            let iv = self.compile_call_arg(args[1])?;
            let lv = self.compile_call_arg(args[2])?;
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

    pub(crate) fn is_identity_lambda_call_arg(lam: &CallArg<'_>) -> bool {
        let CallArg::Hir(expr) = lam;
        Self::is_identity_lambda_hir(expr)
    }

    pub(crate) fn is_identity_lambda_hir(expr: &action_frontend::hir::HirExpr) -> bool {
        use action_frontend::hir::HirExprKind;
        let HirExprKind::Lambda {
            params,
            body,
            implicit_it,
            ..
        } = &expr.kind
        else {
            return false;
        };
        if *implicit_it {
            matches!(&body.kind, HirExprKind::Ident(name) if name == "it")
        } else if params.len() == 1 {
            matches!(&body.kind, HirExprKind::Ident(name) if name == &params[0])
        } else {
            false
        }
    }

    pub(crate) fn extract_filter_call_args_hir(
        expr: &action_frontend::hir::HirExpr,
    ) -> Option<(
        &action_frontend::hir::HirExpr,
        &action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::HirExprKind;
        let HirExprKind::Call {
            func,
            args,
            trailing_lambda,
        } = &expr.kind
        else {
            return None;
        };
        let is_filter = match &func.kind {
            HirExprKind::Ident(name) => name == "filter",
            HirExprKind::FieldAccess(_, method) => method == "filter",
            _ => false,
        };
        if !is_filter {
            return None;
        }
        match trailing_lambda {
            Some(lam) => {
                let inner = match &func.kind {
                    HirExprKind::Ident(_) if args.len() == 1 => Some(&args[0]),
                    HirExprKind::FieldAccess(obj, _) if args.is_empty() => Some(obj.as_ref()),
                    _ => None,
                };
                inner.map(|list| (lam.as_ref(), list))
            }
            None if args.len() == 2 => Some((&args[0], &args[1])),
            _ => None,
        }
    }

    pub(crate) fn extract_map_call_args_hir(
        expr: &action_frontend::hir::HirExpr,
    ) -> Option<(
        &action_frontend::hir::HirExpr,
        &action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::HirExprKind;
        let HirExprKind::Call {
            func,
            args,
            trailing_lambda,
        } = &expr.kind
        else {
            return None;
        };
        let is_map = match &func.kind {
            HirExprKind::Ident(name) => name == "map",
            HirExprKind::FieldAccess(_, method) => method == "map",
            _ => false,
        };
        if !is_map {
            return None;
        }
        match trailing_lambda {
            Some(lam) => {
                let inner = match &func.kind {
                    HirExprKind::Ident(_) if args.len() == 1 => Some(&args[0]),
                    HirExprKind::FieldAccess(obj, _) if args.is_empty() => Some(obj.as_ref()),
                    _ => None,
                };
                inner.map(|list| (lam.as_ref(), list))
            }
            None if args.len() == 2 => Some((&args[0], &args[1])),
            _ => None,
        }
    }

    pub(crate) fn extract_flatmap_call_args_hir(
        expr: &action_frontend::hir::HirExpr,
    ) -> Option<(
        &action_frontend::hir::HirExpr,
        &action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::HirExprKind;
        let HirExprKind::Call {
            func,
            args,
            trailing_lambda,
        } = &expr.kind
        else {
            return None;
        };
        let is_flat = match &func.kind {
            HirExprKind::Ident(name) => name == "flatMap",
            HirExprKind::FieldAccess(_, method) => method == "flatMap",
            _ => false,
        };
        if !is_flat {
            return None;
        }
        match trailing_lambda {
            Some(lam) => {
                let inner = match &func.kind {
                    HirExprKind::Ident(_) if args.len() == 1 => Some(&args[0]),
                    HirExprKind::FieldAccess(obj, _) if args.is_empty() => Some(obj.as_ref()),
                    _ => None,
                };
                inner.map(|list| (lam.as_ref(), list))
            }
            None if args.len() == 2 => Some((&args[0], &args[1])),
            _ => None,
        }
    }

    pub(crate) fn fused_map_filter_hir(
        &mut self,
        map_fn_expr: &action_frontend::hir::HirExpr,
        inner_list_expr: &action_frontend::hir::HirExpr,
        filter_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let map_fn_val = self.compile_hir_expr(map_fn_expr)?;
        let inner_list_val = self.compile_hir_expr(inner_list_expr)?;
        self.fused_map_filter_values(map_fn_val, inner_list_val, filter_fn_val)
    }

    pub(crate) fn fused_map_filter_values(
        &mut self,
        map_fn_val: TypedValue<'ctx>,
        inner_list_val: TypedValue<'ctx>,
        filter_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_fn_ptr = match filter_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused map+filter: filter function required".to_string()),
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

        Ok(TypedValue::List(result_alloca))
    }

    pub(crate) fn fused_map_filter_map_hir(
        &mut self,
        map_inner_expr: &action_frontend::hir::HirExpr,
        base_list_expr: &action_frontend::hir::HirExpr,
        filter_fn_val: TypedValue<'ctx>,
        map_outer_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let map_inner_val = self.compile_hir_expr(map_inner_expr)?;
        let base_list_val = self.compile_hir_expr(base_list_expr)?;
        self.fused_map_filter_map_values(map_inner_val, base_list_val, filter_fn_val, map_outer_val)
    }

    pub(crate) fn fused_map_filter_map_values(
        &mut self,
        map_inner_val: TypedValue<'ctx>,
        base_list_val: TypedValue<'ctx>,
        filter_fn_val: TypedValue<'ctx>,
        map_outer_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_fn_ptr = match filter_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused map+filter+map: filter function required".to_string()),
        };
        let map_inner_ptr = match map_inner_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused map+filter+map: inner map function required".to_string()),
        };
        let map_outer_ptr = match map_outer_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused map+filter+map: outer map function required".to_string()),
        };
        let inner_list_ptr = match base_list_val {
            TypedValue::List(p) => p,
            _ => return Err("fused map+filter+map: list required".to_string()),
        };

        let inner_list_struct = self.load_list(inner_list_ptr)?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "mfm_result")
            .map_err(llvm_err)?;

        let mfm_cc = self.call_rt(
            "action_list_map_filter_map_walk",
            &[
                inner_list_struct.into(),
                map_inner_ptr.into(),
                filter_fn_ptr.into(),
                map_outer_ptr.into(),
            ],
        )?;
        let result_bv = mfm_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_filter_map_walk failed")?;
        self.builder
            .build_store(result_alloca, result_bv)
            .map_err(llvm_err)?;

        Ok(TypedValue::List(result_alloca))
    }

    pub(crate) fn fused_map_take_while_hir(
        &mut self,
        map_fn_expr: &action_frontend::hir::HirExpr,
        inner_list_expr: &action_frontend::hir::HirExpr,
        take_while_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let map_fn_val = self.compile_hir_expr(map_fn_expr)?;
        let inner_list_val = self.compile_hir_expr(inner_list_expr)?;
        self.fused_map_take_while_values(map_fn_val, inner_list_val, take_while_fn_val)
    }

    pub(crate) fn fused_map_take_while_values(
        &mut self,
        map_fn_val: TypedValue<'ctx>,
        inner_list_val: TypedValue<'ctx>,
        take_while_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let map_fn_ptr = match map_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused map+takeWhile: map function required".to_string()),
        };
        let tw_fn_ptr = match take_while_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused map+takeWhile: predicate function required".to_string()),
        };
        let list_ptr = match inner_list_val {
            TypedValue::List(p) => p,
            _ => return Err("fused map+takeWhile: list required".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let mtw_cc = self.call_rt(
            "action_list_map_take_while_walk",
            &[list_struct.into(), map_fn_ptr.into(), tw_fn_ptr.into()],
        )?;
        let result_bv = mtw_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_take_while_walk failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "mtw_res")
            .map_err(llvm_err)?;
        self.builder
            .build_store(res_a, result_bv)
            .map_err(llvm_err)?;
        Ok(TypedValue::List(res_a))
    }

    pub(crate) fn fused_flatmap_filter_hir(
        &mut self,
        flat_fn_expr: &action_frontend::hir::HirExpr,
        inner_list_expr: &action_frontend::hir::HirExpr,
        filter_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let flat_fn_val = self.compile_hir_expr(flat_fn_expr)?;
        let inner_list_val = self.compile_hir_expr(inner_list_expr)?;
        self.fused_flatmap_filter_values(flat_fn_val, inner_list_val, filter_fn_val)
    }

    pub(crate) fn fused_flatmap_filter_values(
        &mut self,
        flat_fn_val: TypedValue<'ctx>,
        inner_list_val: TypedValue<'ctx>,
        filter_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let flat_fn_ptr = match flat_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused flatMap+filter: flatMap function required".to_string()),
        };
        let filter_fn_ptr = match filter_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused flatMap+filter: filter function required".to_string()),
        };
        let list_ptr = match inner_list_val {
            TypedValue::List(p) => p,
            _ => return Err("fused flatMap+filter: list required".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "ff_res")
            .map_err(llvm_err)?;
        self.builder
            .build_store(res_a, cc.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let outer_i = self.builder.build_alloca(i64, "ff_oi").map_err(llvm_err)?;
        self.builder
            .build_store(outer_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "ff_hdr");
        let bdy = self.context.append_basic_block(current_fn, "ff_bdy");
        let ext = self.context.append_basic_block(current_fn, "ff_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let oi = self
            .builder
            .build_load(i64, outer_i, "ff_oiv")
            .map_err(llvm_err)?
            .into_int_value();
        let ocond = self
            .builder
            .build_int_compare(IntPredicate::SLT, oi, input_len, "ff_ocond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(ocond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_fat = self.list_get_cached_fat(list_ptr, oi, cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_fat.into_struct_value(), 0, "ff_et")
            .map_err(llvm_err)?
            .into_int_value();
        let sublist_fat = self.call_list_fn_on_tag(flat_fn_ptr, elem_tag, "ff_flat")?;
        let sublist_ptr = self
            .builder
            .build_alloca(self.list_type, "ff_sub")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sublist_ptr, sublist_fat)
            .map_err(llvm_err)?;
        let sub_loaded = self.load_list(sublist_ptr)?;
        let sub_len = self.list_len_val(sub_loaded)?;
        let inner_i = self.builder.build_alloca(i64, "ff_ii").map_err(llvm_err)?;
        self.builder
            .build_store(inner_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let ihdr = self.context.append_basic_block(current_fn, "ff_ihdr");
        let ibdy = self.context.append_basic_block(current_fn, "ff_ibdy");
        let iaft = self.context.append_basic_block(current_fn, "ff_iaft");
        let _ = self.builder.build_unconditional_branch(ihdr);
        self.builder.position_at_end(ihdr);
        let ii = self
            .builder
            .build_load(i64, inner_i, "ff_iiv")
            .map_err(llvm_err)?
            .into_int_value();
        let icond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ii, sub_len, "ff_icond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(icond, ibdy, iaft);
        self.builder.position_at_end(ibdy);
        let inner_fat = self.list_get_cached_fat(sublist_ptr, ii, cache)?;
        let inner_tag = self
            .builder
            .build_extract_value(inner_fat.into_struct_value(), 0, "ff_it")
            .map_err(llvm_err)?
            .into_int_value();
        let keep = self.call_i64_fn_on_tag(filter_fn_ptr, inner_tag, "ff_filt")?;
        let keep_b = self
            .builder
            .build_int_compare(IntPredicate::NE, keep, i64.const_int(0, false), "ff_keep")
            .map_err(llvm_err)?;
        let push_bb = self.context.append_basic_block(current_fn, "ff_push");
        let skip_bb = self.context.append_basic_block(current_fn, "ff_skip");
        let _ = self
            .builder
            .build_conditional_branch(keep_b, push_bb, skip_bb);
        self.builder.position_at_end(push_bb);
        let rl = self
            .builder
            .build_load(self.list_type, res_a, "ff_rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rp = self.call_rt("action_list_push", &[rl.into(), inner_fat.into()])?;
        self.builder
            .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(skip_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(skip_bb);
        let nii = self
            .builder
            .build_int_add(ii, i64.const_int(1, false), "ff_nii")
            .map_err(llvm_err)?;
        self.builder.build_store(inner_i, nii).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ihdr);
        self.builder.position_at_end(iaft);
        let noi = self
            .builder
            .build_int_add(oi, i64.const_int(1, false), "ff_noi")
            .map_err(llvm_err)?;
        self.builder.build_store(outer_i, noi).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        Ok(TypedValue::List(res_a))
    }

    pub(crate) fn call_i64_fn_on_tag(
        &mut self,
        fn_ptr: inkwell::values::PointerValue<'ctx>,
        tag: IntValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let fn_type = i64.fn_type(&[i64.into()], false);
        self.call_predicate_on_tag(fn_ptr, fn_type, tag, name)
    }

    pub(crate) fn predicate_llvm_fn_type(
        &self,
        val: &TypedValue<'ctx>,
    ) -> Result<inkwell::types::FunctionType<'ctx>, String> {
        match val {
            TypedValue::Fn(_, ft) => Ok(*ft),
            TypedValue::Closure { actual_fn_type, .. } => Ok(*actual_fn_type),
            _ => Err("predicate must be a function".to_string()),
        }
    }

    pub(crate) fn predicate_returns_fat(&self, ft: inkwell::types::FunctionType<'ctx>) -> bool {
        matches!(
            ft.get_return_type(),
            Some(inkwell::types::BasicTypeEnum::StructType(st)) if st == self.string_type
        )
    }

    pub(crate) fn callback_fn_ptr(
        &self,
        val: &TypedValue<'ctx>,
        name: &str,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match val {
            TypedValue::Fn(p, _) => Ok(*p),
            TypedValue::Closure { fn_ptr, .. } => Ok(*fn_ptr),
            _ => Err(format!("{name}: first argument must be a function")),
        }
    }

    pub(crate) fn extract_callback_fn_and_list(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
        expected_args: usize,
        name: &str,
    ) -> Result<(TypedValue<'ctx>, TypedValue<'ctx>), String> {
        if let Some(lam) = trailing {
            if args.len() != expected_args {
                return Err(format!(
                    "{name} with trailing lambda expects {expected_args} argument(s) (list)"
                ));
            }
            let lv = self.compile_call_arg(args[0])?;
            let fv = self.compile_call_arg(lam)?;
            Ok((fv, lv))
        } else if args.len() == expected_args + 1 {
            let a0 = self.compile_call_arg(args[0])?;
            let a1 = self.compile_call_arg(args[1])?;
            match (&a0, &a1) {
                (TypedValue::List(_), _) => Ok((a1, a0)),
                (_, TypedValue::List(_)) => Ok((a0, a1)),
                _ => Err(format!("{name} expects one list and one function argument")),
            }
        } else {
            Err(format!(
                "{name} expects {} argument(s) (fn, list)",
                expected_args + 1
            ))
        }
    }

    pub(crate) fn call_predicate_on_tag_for_val(
        &mut self,
        fn_val: &TypedValue<'ctx>,
        fn_ptr: inkwell::values::PointerValue<'ctx>,
        fn_type: inkwell::types::FunctionType<'ctx>,
        tag: IntValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        match fn_val {
            TypedValue::Closure {
                closure_ptr,
                actual_fn_type,
                fn_ptr,
                ..
            } => {
                let call_r = self
                    .builder
                    .build_indirect_call(
                        *actual_fn_type,
                        *fn_ptr,
                        &[(*closure_ptr).into(), tag.into()],
                        name,
                    )
                    .map_err(llvm_err)?;
                self.predicate_call_result_to_i64(call_r, name)
            }
            _ => self.call_predicate_on_tag(fn_ptr, fn_type, tag, name),
        }
    }

    pub(crate) fn call_predicate_on_tag(
        &mut self,
        fn_ptr: inkwell::values::PointerValue<'ctx>,
        fn_type: inkwell::types::FunctionType<'ctx>,
        tag: IntValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[tag.into()], name)
            .map_err(llvm_err)?;
        self.predicate_call_result_to_i64(call_r, name)
    }

    pub(crate) fn predicate_call_result_to_i64(
        &mut self,
        call_r: inkwell::values::CallSiteValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let bv = call_r
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("{name} returned void"))?;
        if bv.is_struct_value() {
            return Ok(self
                .builder
                .build_extract_value(bv.into_struct_value(), 0, &format!("{name}_t"))
                .map_err(llvm_err)?
                .into_int_value());
        }
        let iv = bv.into_int_value();
        if iv.get_type().get_bit_width() == 1 {
            Ok(self
                .builder
                .build_int_z_extend(iv, self.i64_ty(), &format!("{name}_z"))
                .map_err(llvm_err)?)
        } else {
            Ok(iv)
        }
    }

    pub(crate) fn call_list_fn_on_tag(
        &mut self,
        fn_ptr: inkwell::values::PointerValue<'ctx>,
        tag: IntValue<'ctx>,
        name: &str,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let fn_type = self.list_type.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[tag.into()], name)
            .map_err(llvm_err)?;
        Ok(call_r
            .try_as_basic_value()
            .basic()
            .ok_or("flatMap call failed")?
            .into_struct_value())
    }
}
