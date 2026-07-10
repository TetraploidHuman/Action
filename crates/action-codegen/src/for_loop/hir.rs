//! For-loop codegen (R4-4).

use action_frontend::hir::HirExpr;

use super::ForExprSrc;
use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_hir_for(
        &mut self,
        f: &action_frontend::hir::HirFor,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirForKind;
        match &f.kind {
            HirForKind::Iterate {
                var,
                iterable,
                body,
                collect,
            } => self.compile_for_iterate_hir(var, iterable, body, *collect),
            HirForKind::Condition { condition, body } => {
                if let Some(result) =
                    self.try_compile_for_sequential_list_get_hir(condition, body)?
                {
                    return Ok(result);
                }
                if let Some(result) = self.try_compile_for_map_insert_build_hir(condition, body)? {
                    return Ok(result);
                }
                if let Some(result) =
                    self.try_compile_for_invariant_contains_hir(condition, body)?
                {
                    return Ok(result);
                }
                if let Some(result) = self.try_compile_for_invariant_map_hir(condition, body)? {
                    return Ok(result);
                }
                if let Some(result) =
                    self.try_compile_for_invariant_map_filter_hir(condition, body)?
                {
                    return Ok(result);
                }
                if let Some(result) = self.try_compile_for_invariant_filter_hir(condition, body)? {
                    return Ok(result);
                }
                if let Some(result) = self.try_compile_for_invariant_fold_hir(condition, body)? {
                    return Ok(result);
                }
                if let Some(result) =
                    self.try_compile_for_invariant_map_fold_hir(condition, body)?
                {
                    return Ok(result);
                }
                if let Some(result) =
                    self.try_compile_for_invariant_filter_fold_hir(condition, body)?
                {
                    return Ok(result);
                }
                if let Some(result) =
                    self.try_compile_for_invariant_filter_map_fold_hir(condition, body)?
                {
                    return Ok(result);
                }
                self.compile_for_condition_hir(condition, body)
            }
            HirForKind::Infinite { body } => self.compile_for_infinite_hir(body),
            HirForKind::IterateWithIndex {
                vars,
                iterable,
                body,
            } => self.compile_for_with_index_hir(vars, iterable, body),
            HirForKind::NestedIterate {
                bindings,
                body,
                collect,
            } => self.compile_for_nested_iterate_hir(bindings, body, *collect),
        }
    }

    pub(crate) fn compile_for_condition_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let saved_cache = self.loop_control.list_loop_get_cache;
        if let Some((_, idx_var)) = Self::find_sequential_list_access_in_hir(body) {
            if Self::body_increments_var_hir(body, &idx_var) {
                if let HirExprKind::Binary(lhs, BinaryOp::Lt, _) = &condition.kind {
                    if let HirExprKind::Ident(cond_var) = &lhs.kind {
                        if cond_var == &idx_var {
                            self.loop_control.list_loop_get_cache =
                                Some(self.alloc_list_get_cache()?);
                        }
                    }
                }
            }
        }

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;

        let header = self.context.append_basic_block(current_fn, "for_cond_hdr");
        let body_block = self.context.append_basic_block(current_fn, "for_cond_body");
        let exit = self.context.append_basic_block(current_fn, "for_cond_exit");

        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(header);
        self.loop_control.break_target = Some(exit);

        let _ = self.builder.build_unconditional_branch(header);
        self.builder.position_at_end(header);
        let cv = self.compile_hir_expr(condition)?;
        let cond_val = match cv {
            TypedValue::Bool(b) => b,
            TypedValue::Int(v) => self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::NE,
                    v,
                    self.i64_ty().const_int(0, false),
                    "cond",
                )
                .map_err(llvm_err)?,
            _ => return Err("for condition must evaluate to Bool or Int".to_string()),
        };
        let _ = self
            .builder
            .build_conditional_branch(cond_val, body_block, exit);

        self.builder.position_at_end(body_block);
        let saved_narrowing = self.narrowing.clone();
        self.narrowing =
            action_frontend::fallibility_narrowing::NarrowingContext::from_hir_loop_condition(
                condition,
            );
        let body_val = self.compile_hir_expr(body)?;
        self.narrowing = saved_narrowing;
        self.rc_discard_value(&body_val)?;
        let _ = self.builder.build_unconditional_branch(header);

        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;
        self.loop_control.list_loop_get_cache = saved_cache;

        Ok(TypedValue::Unit)
    }

    pub(crate) fn compile_for_infinite_hir(
        &mut self,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;

        let body_block = self.context.append_basic_block(current_fn, "for_inf_body");
        let exit = self.context.append_basic_block(current_fn, "for_inf_exit");

        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(body_block);
        self.loop_control.break_target = Some(exit);

        let _ = self.builder.build_unconditional_branch(body_block);
        self.builder.position_at_end(body_block);
        let body_val = self.compile_hir_expr(body)?;
        self.rc_discard_value(&body_val)?;
        let _ = self.builder.build_unconditional_branch(body_block);

        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;

        Ok(TypedValue::Unit)
    }

    pub(crate) fn compile_for_iterate_hir(
        &mut self,
        variable: &str,
        iterator: &HirExpr,
        body: &HirExpr,
        collect: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_for_iterate(
            variable,
            ForExprSrc::Hir(iterator),
            ForExprSrc::Hir(body),
            collect,
        )
    }

    pub(crate) fn compile_for_with_index_hir(
        &mut self,
        vars: &[String],
        iterator: &HirExpr,
        body: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_for_with_index(vars, ForExprSrc::Hir(iterator), ForExprSrc::Hir(body))
    }

    pub(crate) fn compile_for_nested_iterate_hir(
        &mut self,
        bindings: &[(String, HirExpr)],
        body: &HirExpr,
        collect: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let hir_bindings: Vec<(String, ForExprSrc<'_>)> = bindings
            .iter()
            .map(|(n, e)| (n.clone(), ForExprSrc::Hir(e)))
            .collect();
        self.compile_for_nested_iterate(&hir_bindings, ForExprSrc::Hir(body), collect)
    }

    pub(crate) fn try_compile_for_invariant_map_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use crate::call_arg::CallArg;
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let idx_var = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, _) => match &lhs.kind {
                HirExprKind::Ident(v) => v.clone(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, map_lam_hir, inc_body) =
            match Self::extract_invariant_map_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&list_hir, &idx_var)
            || Self::hir_expr_refs_var(&map_lam_hir, &idx_var)
        {
            return Ok(None);
        }

        let mapped =
            self.builtin_map(&[CallArg::Hir(&list_hir)], Some(CallArg::Hir(&map_lam_hir)))?;
        self.rc_free_intermediate(&mapped)?;

        self.compile_for_condition_hir(condition, &inc_body)
            .map(Some)
    }

    pub(crate) fn try_compile_for_invariant_filter_map_fold_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let idx_var = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, _) => match &lhs.kind {
                HirExprKind::Ident(v) => v.clone(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, filter_lam, map_lam, fold_init, fold_lam, inc_body) =
            match Self::extract_invariant_filter_map_fold_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&list_hir, &idx_var)
            || Self::hir_expr_refs_var(&filter_lam, &idx_var)
            || Self::hir_expr_refs_var(&map_lam, &idx_var)
            || Self::hir_expr_refs_var(&fold_init, &idx_var)
            || Self::hir_expr_refs_var(&fold_lam, &idx_var)
        {
            return Ok(None);
        }

        let _sum = self.fused_filter_map_fold_hir(
            &filter_lam,
            &map_lam,
            &fold_lam,
            &list_hir,
            &fold_init,
        )?;
        self.compile_for_condition_hir(condition, &inc_body)
            .map(Some)
    }

    pub(crate) fn try_compile_for_invariant_map_filter_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let idx_var = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, _) => match &lhs.kind {
                HirExprKind::Ident(v) => v.clone(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, map_lam, filter_lam, inc_body) =
            match Self::extract_invariant_map_filter_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&list_hir, &idx_var)
            || Self::hir_expr_refs_var(&map_lam, &idx_var)
            || Self::hir_expr_refs_var(&filter_lam, &idx_var)
        {
            return Ok(None);
        }

        let filter_fn = self.compile_hir_expr(&filter_lam)?;
        let _mapped = self.fused_map_filter_hir(&map_lam, &list_hir, filter_fn)?;
        self.rc_free_intermediate(&_mapped)?;
        self.compile_for_condition_hir(condition, &inc_body)
            .map(Some)
    }

    pub(crate) fn try_compile_for_invariant_contains_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use crate::ValKind;
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let (idx_var, end_hir) = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, rhs) => match &lhs.kind {
                HirExprKind::Ident(v) => (v.clone(), rhs.as_ref().clone()),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, key_hir, inc_hir) =
            match Self::extract_invariant_contains_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&list_hir, &idx_var) {
            return Ok(None);
        }

        let end_val = self.compile_hir_expr(&end_hir)?;
        let end_bound = match end_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };
        if let Some(n) = end_bound.get_zero_extended_constant() {
            if n < Self::CONTAINS_HT_FUSION_MIN_ITERS {
                return Ok(None);
            }
        }

        let list_val = self.compile_hir_expr(&list_hir)?;
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Ok(None),
        };
        let idx_scope = match self.scope.get(&idx_var) {
            Some(v) if v.kind == ValKind::Int => v.ptr,
            _ => return Ok(None),
        };

        self.compile_invariant_contains_loop(list_ptr, end_bound, idx_scope, &key_hir, &inc_hir)
            .map(Some)
    }

    pub(crate) fn extract_invariant_contains_loop_body(
        body: &action_frontend::hir::HirExpr,
        idx_var: &str,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        if stmts.len() != 2 {
            return None;
        }
        let contains_expr = match &stmts[0] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => value,
            _ => return None,
        };
        let (list_hir, key_hir) = Self::extract_list_contains_call_hir(contains_expr)?;
        let inc_hir = match &stmts[1] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                if Self::hir_stmt_is_increment_expr(value, idx_var) {
                    Some(value.clone())
                } else {
                    None
                }
            }
            _ => None,
        }?;
        Some((list_hir, key_hir, inc_hir))
    }

    pub(crate) fn extract_list_contains_call_hir(
        expr: &action_frontend::hir::HirExpr,
    ) -> Option<(action_frontend::hir::HirExpr, action_frontend::hir::HirExpr)> {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Call {
                func,
                args,
                trailing_lambda,
            } if trailing_lambda.is_none() => {
                if let HirExprKind::FieldAccess(obj, method) = &func.kind {
                    if method == "contains" && args.len() == 1 {
                        return Some((obj.as_ref().clone(), args[0].clone()));
                    }
                }
                if let HirExprKind::Ident(name) = &func.kind {
                    if name == "contains" && args.len() == 2 {
                        return Some((args[0].clone(), args[1].clone()));
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn try_compile_for_invariant_filter_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use crate::call_arg::CallArg;
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let idx_var = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, _) => match &lhs.kind {
                HirExprKind::Ident(v) => v.clone(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, filter_lam, inc_body) =
            match Self::extract_invariant_filter_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&list_hir, &idx_var)
            || Self::hir_expr_refs_var(&filter_lam, &idx_var)
        {
            return Ok(None);
        }

        let filtered =
            self.builtin_filter(&[CallArg::Hir(&list_hir)], Some(CallArg::Hir(&filter_lam)))?;
        self.rc_free_intermediate(&filtered)?;
        self.compile_for_condition_hir(condition, &inc_body)
            .map(Some)
    }

    pub(crate) fn try_compile_for_invariant_fold_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use crate::call_arg::CallArg;
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let idx_var = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, _) => match &lhs.kind {
                HirExprKind::Ident(v) => v.clone(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, fold_init, fold_lam, inc_body) =
            match Self::extract_invariant_fold_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&list_hir, &idx_var)
            || Self::hir_expr_refs_var(&fold_init, &idx_var)
            || Self::hir_expr_refs_var(&fold_lam, &idx_var)
        {
            return Ok(None);
        }

        let _sum = self.builtin_fold(
            &[CallArg::Hir(&fold_init), CallArg::Hir(&list_hir)],
            Some(CallArg::Hir(&fold_lam)),
        )?;
        self.compile_for_condition_hir(condition, &inc_body)
            .map(Some)
    }

    pub(crate) fn try_compile_for_invariant_map_fold_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let idx_var = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, _) => match &lhs.kind {
                HirExprKind::Ident(v) => v.clone(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, map_lam, fold_init, fold_lam, inc_body) =
            match Self::extract_invariant_map_fold_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&list_hir, &idx_var)
            || Self::hir_expr_refs_var(&map_lam, &idx_var)
            || Self::hir_expr_refs_var(&fold_init, &idx_var)
            || Self::hir_expr_refs_var(&fold_lam, &idx_var)
        {
            return Ok(None);
        }

        let _sum = self.fused_map_fold_hir(&map_lam, &fold_lam, &list_hir, &fold_init)?;
        self.compile_for_condition_hir(condition, &inc_body)
            .map(Some)
    }

    pub(crate) fn try_compile_for_invariant_filter_fold_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let idx_var = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, _) => match &lhs.kind {
                HirExprKind::Ident(v) => v.clone(),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, filter_lam, fold_init, fold_lam, inc_body) =
            match Self::extract_invariant_filter_fold_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&list_hir, &idx_var)
            || Self::hir_expr_refs_var(&filter_lam, &idx_var)
            || Self::hir_expr_refs_var(&fold_init, &idx_var)
            || Self::hir_expr_refs_var(&fold_lam, &idx_var)
        {
            return Ok(None);
        }

        let _sum = self.fused_filter_fold_hir(&filter_lam, &fold_lam, &list_hir, &fold_init)?;
        self.compile_for_condition_hir(condition, &inc_body)
            .map(Some)
    }

    pub(crate) fn extract_invariant_filter_loop_body(
        body: &action_frontend::hir::HirExpr,
        idx_var: &str,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        if stmts.len() != 2 {
            return None;
        }
        let (filter_lam, list_hir) = match &stmts[0] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                Self::extract_filter_trailing_lambda_hir(value)?
            }
            _ => return None,
        };
        let inc_expr = match &stmts[1] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                if Self::hir_stmt_is_increment_expr(value, idx_var) {
                    Some(value.clone())
                } else {
                    None
                }
            }
            _ => None,
        }?;
        Some((list_hir, filter_lam, inc_expr))
    }

    pub(crate) fn extract_invariant_fold_loop_body(
        body: &action_frontend::hir::HirExpr,
        idx_var: &str,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        if stmts.len() != 2 {
            return None;
        };
        let (fold_init, fold_lam, list_hir) = match &stmts[0] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                Self::extract_fold_trailing_lambda_hir(value, None)?
            }
            _ => return None,
        };
        let inc_expr = match &stmts[1] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                if Self::hir_stmt_is_increment_expr(value, idx_var) {
                    Some(value.clone())
                } else {
                    None
                }
            }
            _ => None,
        }?;
        Some((list_hir, fold_init, fold_lam, inc_expr))
    }

    pub(crate) fn extract_invariant_map_fold_loop_body(
        body: &action_frontend::hir::HirExpr,
        idx_var: &str,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        if stmts.len() != 3 {
            return None;
        };
        let (list_hir, map_lam) = match &stmts[0] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                Self::extract_map_trailing_lambda_hir(value)?
            }
            _ => return None,
        };
        let map_bind = match &stmts[0] {
            HirStmt::Let { name, .. } => name.clone(),
            _ => return None,
        };
        let (fold_init, fold_lam, _) = match &stmts[1] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                Self::extract_fold_trailing_lambda_hir(value, Some(&map_bind))?
            }
            _ => return None,
        };
        let inc_expr = match &stmts[2] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                if Self::hir_stmt_is_increment_expr(value, idx_var) {
                    Some(value.clone())
                } else {
                    None
                }
            }
            _ => None,
        }?;
        Some((list_hir, map_lam, fold_init, fold_lam, inc_expr))
    }

    pub(crate) fn extract_invariant_filter_fold_loop_body(
        body: &action_frontend::hir::HirExpr,
        idx_var: &str,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        if stmts.len() != 3 {
            return None;
        }
        let (filter_lam, list_hir) = match &stmts[0] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                Self::extract_filter_trailing_lambda_hir(value)?
            }
            _ => return None,
        };
        let filter_bind = match &stmts[0] {
            HirStmt::Let { name, .. } => name.clone(),
            _ => return None,
        };
        let (fold_init, fold_lam, _) = match &stmts[1] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                Self::extract_fold_trailing_lambda_hir(value, Some(&filter_bind))?
            }
            _ => return None,
        };
        let inc_expr = match &stmts[2] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                if Self::hir_stmt_is_increment_expr(value, idx_var) {
                    Some(value.clone())
                } else {
                    None
                }
            }
            _ => None,
        }?;
        Some((list_hir, filter_lam, fold_init, fold_lam, inc_expr))
    }

    pub(crate) fn extract_filter_map_fold_stmt_chain(
        stmts: &[action_frontend::hir::HirStmt],
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        String,
        String,
        String,
    )> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        if stmts.len() < 3 {
            return None;
        }
        let filter_pair = match &stmts[0] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                Self::extract_filter_trailing_lambda_hir(value)?
            }
            _ => return None,
        };
        let (filter_lam, list_hir) = filter_pair;
        let filter_bind = match &stmts[0] {
            HirStmt::Let { name, .. } => name.clone(),
            _ => return None,
        };
        let map_pair = match &stmts[1] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                let (map_input, map_lam) = Self::extract_map_trailing_lambda_hir(value)?;
                match &map_input.kind {
                    HirExprKind::Ident(name) if name == &filter_bind => Some((map_lam, map_input)),
                    _ => None,
                }
            }
            _ => return None,
        };
        let (map_lam, _map_inner) = map_pair?;
        let map_bind = match &stmts[1] {
            HirStmt::Let { name, .. } => name.clone(),
            _ => return None,
        };
        let fold_pair = match &stmts[2] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                Self::extract_fold_trailing_lambda_hir(value, Some(&map_bind))?
            }
            _ => return None,
        };
        let (fold_init, fold_lam, fold_inner) = fold_pair;
        if !matches!(&fold_inner.kind, HirExprKind::Ident(n) if n == &map_bind) {
            return None;
        }
        let fold_bind = match &stmts[2] {
            HirStmt::Let { name, .. } => name.clone(),
            _ => return None,
        };
        Some((
            list_hir,
            filter_lam,
            map_lam,
            fold_init,
            fold_lam,
            fold_bind,
            filter_bind,
            map_bind,
        ))
    }

    pub(crate) fn extract_invariant_filter_map_fold_loop_body(
        body: &action_frontend::hir::HirExpr,
        idx_var: &str,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        if stmts.len() != 4 {
            return None;
        }
        let (list_hir, filter_lam, map_lam, fold_init, fold_lam, _, _, _) =
            Self::extract_filter_map_fold_stmt_chain(stmts)?;
        let inc_expr = match &stmts[3] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                if Self::hir_stmt_is_increment_expr(value, idx_var) {
                    Some(value.clone())
                } else {
                    None
                }
            }
            _ => None,
        }?;
        Some((list_hir, filter_lam, map_lam, fold_init, fold_lam, inc_expr))
    }

    /// Fuse consecutive `filter` → `map` → `fold` let-bindings in a block (e.g. bench_for_chain tail).
    pub(crate) fn try_compile_filter_map_fold_stmt_chain(
        &mut self,
        stmts: &[action_frontend::hir::HirStmt],
    ) -> Result<Option<usize>, String> {
        let (list_hir, filter_lam, map_lam, fold_init, fold_lam, fold_bind, filter_bind, map_bind) =
            match Self::extract_filter_map_fold_stmt_chain(stmts) {
                Some(v) => v,
                None => return Ok(None),
            };
        for s in &stmts[3..] {
            if Self::hir_stmt_refs_var(s, &filter_bind) || Self::hir_stmt_refs_var(s, &map_bind) {
                return Ok(None);
            }
        }
        let sum = self.fused_filter_map_fold_hir(
            &filter_lam,
            &map_lam,
            &fold_lam,
            &list_hir,
            &fold_init,
        )?;
        self.bind_hir_immutable_int(&fold_bind, sum)?;
        Ok(Some(3))
    }

    pub(crate) fn extract_filter_trailing_lambda_hir(
        expr: &action_frontend::hir::HirExpr,
    ) -> Option<(action_frontend::hir::HirExpr, action_frontend::hir::HirExpr)> {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => {
                let is_filter = matches!(&func.kind, HirExprKind::Ident(name) if name == "filter");
                if !is_filter || args.len() != 1 {
                    return None;
                }
                let lam = trailing_lambda.as_ref()?;
                Some((lam.as_ref().clone(), args[0].clone()))
            }
            _ => None,
        }
    }

    pub(crate) fn extract_fold_trailing_lambda_hir(
        expr: &action_frontend::hir::HirExpr,
        expected_list: Option<&str>,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => {
                let is_fold = matches!(&func.kind, HirExprKind::Ident(name) if name == "fold");
                if !is_fold || args.len() != 2 {
                    return None;
                }
                let lam = trailing_lambda.as_ref()?;
                if let Some(expected) = expected_list {
                    match &args[1].kind {
                        HirExprKind::Ident(name) if name == expected => {}
                        _ => return None,
                    }
                }
                Some((args[0].clone(), lam.as_ref().clone(), args[1].clone()))
            }
            _ => None,
        }
    }

    pub(crate) fn extract_invariant_map_loop_body(
        body: &action_frontend::hir::HirExpr,
        idx_var: &str,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExpr, HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        let mut map_list: Option<HirExpr> = None;
        let mut map_lam: Option<HirExpr> = None;
        let mut inc_expr: Option<HirExpr> = None;
        for stmt in stmts {
            match stmt {
                HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                    if let Some((list, lam)) = Self::extract_map_trailing_lambda_hir(value) {
                        if map_list.is_some() {
                            return None;
                        }
                        map_list = Some(list);
                        map_lam = Some(lam);
                    } else if Self::hir_stmt_is_increment_expr(value, idx_var) {
                        if inc_expr.is_some() {
                            return None;
                        }
                        inc_expr = Some(value.clone());
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some((map_list?, map_lam?, inc_expr?))
    }

    pub(crate) fn extract_map_filter_trailing_lambda_hir(
        expr: &action_frontend::hir::HirExpr,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        Self::extract_map_filter_trailing_lambda_hir_inner(expr, true)
    }

    fn extract_map_filter_trailing_lambda_hir_inner(
        expr: &action_frontend::hir::HirExpr,
        allow_ufcs: bool,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::HirExprKind;
        let HirExprKind::Call {
            func: filter_func,
            args: filter_args,
            trailing_lambda: filter_lam,
        } = &expr.kind
        else {
            return None;
        };
        let is_filter = match &filter_func.kind {
            HirExprKind::Ident(name) => name == "filter",
            HirExprKind::FieldAccess(_, method) if allow_ufcs => method == "filter",
            _ => false,
        };
        if !is_filter {
            return None;
        }
        let filter_lam = filter_lam.as_ref()?.as_ref().clone();
        let map_expr = match &filter_func.kind {
            HirExprKind::Ident(_) if filter_args.len() == 1 => &filter_args[0],
            HirExprKind::FieldAccess(_, _) if allow_ufcs && filter_args.is_empty() => {
                match &filter_func.kind {
                    HirExprKind::FieldAccess(map_call, _) => map_call.as_ref(),
                    _ => return None,
                }
            }
            _ => return None,
        };
        let (list_hir, map_lam) =
            Self::extract_map_trailing_lambda_hir_inner(map_expr, allow_ufcs)?;
        Some((list_hir, map_lam, filter_lam))
    }

    pub(crate) fn extract_map_trailing_lambda_hir(
        expr: &action_frontend::hir::HirExpr,
    ) -> Option<(action_frontend::hir::HirExpr, action_frontend::hir::HirExpr)> {
        Self::extract_map_trailing_lambda_hir_inner(expr, true)
    }

    fn extract_map_trailing_lambda_hir_inner(
        expr: &action_frontend::hir::HirExpr,
        allow_ufcs: bool,
    ) -> Option<(action_frontend::hir::HirExpr, action_frontend::hir::HirExpr)> {
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
            HirExprKind::FieldAccess(_, method) if allow_ufcs => method == "map",
            _ => false,
        };
        if !is_map {
            return None;
        }
        let lam = trailing_lambda.as_ref()?.as_ref().clone();
        let list = match &func.kind {
            HirExprKind::Ident(_) if args.len() == 1 => args[0].clone(),
            HirExprKind::FieldAccess(obj, _) if allow_ufcs && args.is_empty() => {
                obj.as_ref().clone()
            }
            _ => return None,
        };
        Some((list, lam))
    }

    /// Fuse `val x = lst.map{…}.filter{…}` (call or UFCS) in a block.
    pub(crate) fn try_compile_map_filter_let_fusion(
        &mut self,
        stmts: &[action_frontend::hir::HirStmt],
    ) -> Result<Option<usize>, String> {
        use action_frontend::hir::HirStmt;
        if stmts.is_empty() {
            return Ok(None);
        }
        let value = match &stmts[0] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => value,
            _ => return Ok(None),
        };
        let (list_hir, map_lam, filter_lam) =
            match Self::extract_map_filter_trailing_lambda_hir(value) {
                Some(v) => v,
                None => return Ok(None),
            };
        let bind = match &stmts[0] {
            HirStmt::Let { name, .. } => name.clone(),
            _ => return Ok(None),
        };
        let filter_fn = self.compile_hir_expr(&filter_lam)?;
        let mapped = self.fused_map_filter_hir(&map_lam, &list_hir, filter_fn)?;
        self.bind_hir_list(&bind, mapped)?;
        Ok(Some(1))
    }

    pub(crate) fn extract_invariant_map_filter_loop_body(
        body: &action_frontend::hir::HirExpr,
        idx_var: &str,
    ) -> Option<(
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExpr, HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        let mut list_hir: Option<HirExpr> = None;
        let mut map_lam: Option<HirExpr> = None;
        let mut filter_lam: Option<HirExpr> = None;
        let mut inc_expr: Option<HirExpr> = None;
        for stmt in stmts {
            match stmt {
                HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                    if let Some((list, map, filter)) =
                        Self::extract_map_filter_trailing_lambda_hir(value)
                    {
                        if list_hir.is_some() {
                            return None;
                        }
                        list_hir = Some(list);
                        map_lam = Some(map);
                        filter_lam = Some(filter);
                    } else if Self::hir_stmt_is_increment_expr(value, idx_var) {
                        if inc_expr.is_some() {
                            return None;
                        }
                        inc_expr = Some(value.clone());
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some((list_hir?, map_lam?, filter_lam?, inc_expr?))
    }

    pub(crate) fn hir_stmt_is_increment_expr(
        expr: &action_frontend::hir::HirExpr,
        var: &str,
    ) -> bool {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Assign { target, value } => Self::is_var_increment_hir(target, value, var),
            _ => false,
        }
    }

    pub(crate) fn hir_expr_refs_var(expr: &action_frontend::hir::HirExpr, var: &str) -> bool {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Ident(name) => name == var,
            HirExprKind::Unary(_, inner) => Self::hir_expr_refs_var(inner, var),
            HirExprKind::Binary(lhs, _, rhs) => {
                Self::hir_expr_refs_var(lhs, var) || Self::hir_expr_refs_var(rhs, var)
            }
            HirExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => {
                Self::hir_expr_refs_var(func, var)
                    || args.iter().any(|a| Self::hir_expr_refs_var(a, var))
                    || trailing_lambda
                        .as_ref()
                        .is_some_and(|t| Self::hir_expr_refs_var(t, var))
            }
            HirExprKind::FieldAccess(obj, _) => Self::hir_expr_refs_var(obj, var),
            HirExprKind::Index(obj, idx) => {
                Self::hir_expr_refs_var(obj, var) || Self::hir_expr_refs_var(idx, var)
            }
            HirExprKind::Block(stmts) => stmts.iter().any(|s| Self::hir_stmt_refs_var(s, var)),
            HirExprKind::Lambda { body, .. } => Self::hir_expr_refs_var(body, var),
            HirExprKind::When(w) => Self::hir_when_refs_var(w, var),
            HirExprKind::StructLiteral(fields) => {
                fields.iter().any(|(_, v)| Self::hir_expr_refs_var(v, var))
            }
            HirExprKind::MapLiteral(entries) => entries
                .iter()
                .any(|(k, v)| Self::hir_expr_refs_var(k, var) || Self::hir_expr_refs_var(v, var)),
            HirExprKind::SetLiteral(items) => items.iter().any(|i| Self::hir_expr_refs_var(i, var)),
            HirExprKind::Range(start, end) => {
                Self::hir_expr_refs_var(start, var) || Self::hir_expr_refs_var(end, var)
            }
            HirExprKind::Tuple(items) => items.iter().any(|(_, v)| Self::hir_expr_refs_var(v, var)),
            HirExprKind::OrBlock { fallible, fallback } => {
                Self::hir_expr_refs_var(fallible, var) || Self::hir_expr_refs_var(fallback, var)
            }
            HirExprKind::Assign { target, value } => {
                Self::hir_expr_refs_var(target, var) || Self::hir_expr_refs_var(value, var)
            }
            HirExprKind::StringInterpolate(parts) => parts.iter().any(|part| match part {
                action_frontend::hir::HirStringPart::Literal(_) => false,
                action_frontend::hir::HirStringPart::Expr(e) => Self::hir_expr_refs_var(e, var),
            }),
            HirExprKind::Copy(inner) | HirExprKind::Unsafe(inner) => {
                Self::hir_expr_refs_var(inner, var)
            }
            _ => false,
        }
    }

    pub(crate) fn hir_when_refs_var(w: &action_frontend::hir::HirWhen, var: &str) -> bool {
        use action_frontend::hir::HirWhenKind;
        match &w.kind {
            HirWhenKind::OneLine {
                condition,
                then_expr,
                else_expr,
            } => {
                Self::hir_expr_refs_var(condition, var)
                    || Self::hir_expr_refs_var(then_expr, var)
                    || Self::hir_expr_refs_var(else_expr, var)
            }
            HirWhenKind::ValueMatch { value, arms } => {
                Self::hir_expr_refs_var(value, var)
                    || arms.iter().any(|a| Self::hir_when_arm_refs_var(a, var))
            }
            HirWhenKind::ConditionChain { arms } => {
                arms.iter().any(|a| Self::hir_when_arm_refs_var(a, var))
            }
        }
    }

    pub(crate) fn hir_when_arm_refs_var(arm: &action_frontend::hir::HirWhenArm, var: &str) -> bool {
        arm.guard
            .as_ref()
            .is_some_and(|g| Self::hir_expr_refs_var(g, var))
            || Self::hir_expr_refs_var(&arm.body, var)
    }

    pub(crate) fn hir_stmt_refs_var(stmt: &action_frontend::hir::HirStmt, var: &str) -> bool {
        use action_frontend::hir::HirStmt;
        match stmt {
            HirStmt::Let { value, .. } => Self::hir_expr_refs_var(value, var),
            HirStmt::Expr { expr, .. } => Self::hir_expr_refs_var(expr, var),
            HirStmt::Return { value, .. } => value
                .as_ref()
                .is_some_and(|v| Self::hir_expr_refs_var(v, var)),
            _ => false,
        }
    }

    pub(crate) fn try_compile_for_sequential_list_get_hir(
        &mut self,
        condition: &HirExpr,
        body: &HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;
        let (idx_var, end_hir): (String, HirExpr) = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, rhs) => match (&lhs.kind, &rhs.kind) {
                (HirExprKind::Ident(v), _) => (v.clone(), rhs.as_ref().clone()),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, get_idx_var) = match Self::find_sequential_list_access_in_hir(body) {
            Some(v) => v,
            None => return Ok(None),
        };
        if get_idx_var != idx_var {
            return Ok(None);
        }
        if !Self::body_increments_var_hir(body, &idx_var) {
            return Ok(None);
        }
        let list_val = self.compile_hir_expr(&list_hir)?;
        let list_ptr = match &list_val {
            TypedValue::List(p) => *p,
            _ => return Ok(None),
        };
        let end_val = self.compile_hir_expr(&end_hir)?;
        let end_bound = match end_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };
        self.compile_sequential_list_get_loop(list_ptr, end_bound)
            .map(Some)
    }

    pub(crate) fn find_sequential_list_access_in_hir(body: &HirExpr) -> Option<(HirExpr, String)> {
        Self::find_list_get_in_hir(body).or_else(|| Self::find_list_index_in_hir(body))
    }

    pub(crate) fn find_list_index_in_hir(body: &HirExpr) -> Option<(HirExpr, String)> {
        use action_frontend::hir::HirExprKind;
        match &body.kind {
            HirExprKind::Block(stmts) => {
                for stmt in stmts {
                    if let Some(v) = Self::find_list_index_in_hir_stmt(stmt) {
                        return Some(v);
                    }
                }
                None
            }
            _ => Self::find_list_index_in_hir_inner(body),
        }
    }

    pub(crate) fn find_list_index_in_hir_stmt(
        stmt: &action_frontend::hir::HirStmt,
    ) -> Option<(HirExpr, String)> {
        use action_frontend::hir::HirStmt;
        match stmt {
            HirStmt::Let { value, .. } => Self::find_list_index_in_hir_inner(value),
            HirStmt::Expr { expr, .. } => Self::find_list_index_in_hir_inner(expr),
            _ => None,
        }
    }

    pub(crate) fn find_list_index_in_hir_inner(expr: &HirExpr) -> Option<(HirExpr, String)> {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Index(obj, idx) => {
                if let HirExprKind::Ident(name) = &idx.kind {
                    Some((obj.as_ref().clone(), name.clone()))
                } else {
                    None
                }
            }
            HirExprKind::Block(_) => Self::find_list_index_in_hir(expr),
            _ => None,
        }
    }

    pub(crate) fn find_list_get_in_hir(body: &HirExpr) -> Option<(HirExpr, String)> {
        use action_frontend::hir::HirExprKind;
        match &body.kind {
            HirExprKind::Block(stmts) => {
                for stmt in stmts {
                    if let Some(v) = Self::find_list_get_in_hir_stmt(stmt) {
                        return Some(v);
                    }
                }
                None
            }
            _ => Self::find_list_get_in_hir_inner(body),
        }
    }

    pub(crate) fn find_list_get_in_hir_stmt(
        stmt: &action_frontend::hir::HirStmt,
    ) -> Option<(HirExpr, String)> {
        use action_frontend::hir::HirStmt;
        match stmt {
            HirStmt::Let { value, .. } => Self::find_list_get_in_hir_inner(value),
            HirStmt::Expr { expr, .. } => Self::find_list_get_in_hir_inner(expr),
            _ => None,
        }
    }

    pub(crate) fn find_list_get_in_hir_inner(expr: &HirExpr) -> Option<(HirExpr, String)> {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Call { func, args, .. } => {
                if let HirExprKind::FieldAccess(obj, method) = &func.kind {
                    if method == "get" && args.len() == 1 {
                        if let HirExprKind::Ident(idx) = &args[0].kind {
                            return Some((obj.as_ref().clone(), idx.clone()));
                        }
                    }
                }
                if let HirExprKind::Ident(name) = &func.kind {
                    if name == "get" && args.len() == 2 {
                        if let HirExprKind::Ident(idx) = &args[1].kind {
                            return Some((args[0].clone(), idx.clone()));
                        }
                    }
                }
                None
            }
            HirExprKind::Block(_) => Self::find_list_get_in_hir(expr),
            HirExprKind::OrBlock { fallible, .. } => Self::find_list_get_in_hir_inner(fallible),
            _ => None,
        }
    }

    pub(crate) fn body_increments_var_hir(body: &HirExpr, var: &str) -> bool {
        use action_frontend::hir::HirExprKind;
        match &body.kind {
            HirExprKind::Block(stmts) => {
                stmts.iter().any(|s| Self::hir_stmt_increments_var(s, var))
            }
            HirExprKind::Assign { target, value } => Self::is_var_increment_hir(target, value, var),
            _ => false,
        }
    }

    pub(crate) fn hir_stmt_increments_var(stmt: &action_frontend::hir::HirStmt, var: &str) -> bool {
        use action_frontend::hir::{HirExprKind, HirStmt};
        match stmt {
            HirStmt::Expr { expr, .. } => match &expr.kind {
                HirExprKind::Assign { target, value } => {
                    Self::is_var_increment_hir(target, value, var)
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub(crate) fn try_compile_for_map_insert_build_hir(
        &mut self,
        condition: &HirExpr,
        body: &HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use crate::ValKind;
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;

        let (idx_var, end_hir) = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, rhs) => match &lhs.kind {
                HirExprKind::Ident(v) => (v.clone(), rhs.as_ref().clone()),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (coll_var, key_hir, val_hir, inc_hir) =
            match Self::extract_collection_insert_loop_body(body, &idx_var) {
                Some(v) => v,
                None => return Ok(None),
            };
        if Self::hir_expr_refs_var(&key_hir, &idx_var) {
            return Ok(None);
        }
        if val_hir
            .as_ref()
            .is_some_and(|v| Self::hir_expr_refs_var(v, &idx_var))
        {
            return Ok(None);
        }

        let end_val = self.compile_hir_expr(&end_hir)?;
        let end_bound = match end_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };
        if let Some(n) = end_bound.get_zero_extended_constant() {
            if n < Self::MAP_INSERT_PRESIZE_MIN_ITERS {
                return Ok(None);
            }
        }

        let coll_scope = match self.scope.get(&coll_var) {
            Some(v) if matches!(v.kind, ValKind::Map | ValKind::Set) => v,
            _ => return Ok(None),
        };
        if !coll_scope.mutable {
            return Ok(None);
        }
        let idx_scope = match self.scope.get(&idx_var) {
            Some(v) if v.kind == ValKind::Int => v,
            _ => return Ok(None),
        };

        self.compile_collection_insert_build_loop(
            coll_scope.ptr,
            end_bound,
            idx_scope.ptr,
            &key_hir,
            val_hir.as_ref(),
            &inc_hir,
            coll_scope.kind,
        )
        .map(Some)
    }

    pub(crate) fn extract_collection_insert_loop_body(
        body: &HirExpr,
        idx_var: &str,
    ) -> Option<(
        String,
        action_frontend::hir::HirExpr,
        Option<action_frontend::hir::HirExpr>,
        action_frontend::hir::HirExpr,
    )> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        let stmts = match &body.kind {
            HirExprKind::Block(stmts) => stmts,
            _ => return None,
        };
        if stmts.len() != 2 {
            return None;
        }
        let insert_expr = match &stmts[0] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => value,
            _ => return None,
        };
        let (coll_var, key_hir, val_hir) = Self::extract_self_insert_assign_hir(insert_expr)?;
        let inc_hir = match &stmts[1] {
            HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
                if Self::hir_stmt_is_increment_expr(value, idx_var) {
                    Some(value.clone())
                } else {
                    None
                }
            }
            _ => None,
        }?;
        Some((coll_var, key_hir, val_hir, inc_hir))
    }

    pub(crate) fn extract_self_insert_assign_hir(
        expr: &action_frontend::hir::HirExpr,
    ) -> Option<(
        String,
        action_frontend::hir::HirExpr,
        Option<action_frontend::hir::HirExpr>,
    )> {
        use action_frontend::hir::HirExprKind;
        let HirExprKind::Assign { target, value } = &expr.kind else {
            return None;
        };
        let HirExprKind::Ident(coll_var) = &target.kind else {
            return None;
        };
        let HirExprKind::Call {
            func,
            args,
            trailing_lambda,
        } = &value.kind
        else {
            return None;
        };
        if trailing_lambda.is_some() {
            return None;
        }
        let HirExprKind::FieldAccess(obj, method) = &func.kind else {
            return None;
        };
        if method != "insert" {
            return None;
        }
        let HirExprKind::Ident(recv) = &obj.kind else {
            return None;
        };
        if recv != coll_var {
            return None;
        }
        match args.len() {
            1 => Some((coll_var.clone(), args[0].clone(), None)),
            2 => Some((coll_var.clone(), args[0].clone(), Some(args[1].clone()))),
            _ => None,
        }
    }

    pub(crate) fn is_var_increment_hir(target: &HirExpr, value: &HirExpr, var: &str) -> bool {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;
        match (&target.kind, &value.kind) {
            (HirExprKind::Ident(t), HirExprKind::Binary(lhs, BinaryOp::Add, rhs)) if t == var => {
                matches!(&lhs.kind, HirExprKind::Ident(v) if v == var)
                    || matches!(&rhs.kind, HirExprKind::Ident(v) if v == var)
            }
            _ => false,
        }
    }
}
