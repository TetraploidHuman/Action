use crate::{CodeGen, TypedValue};
use action_frontend::ast::*;
use action_frontend::hir::*;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_hir_binary(
        &mut self,
        lhs: &HirExpr,
        op: BinaryOp,
        rhs: &HirExpr,
        result_ty: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        if matches!(
            op,
            BinaryOp::And | BinaryOp::Or | BinaryOp::Is | BinaryOp::In
        ) {
            return match op {
                BinaryOp::And => self.compile_and_hir(lhs, rhs),
                BinaryOp::Or => self.compile_or_hir(lhs, rhs),
                BinaryOp::Is => self.bin_is_hir(lhs, rhs),
                BinaryOp::In => self.bin_in_hir(lhs, rhs),
                _ => unreachable!(),
            };
        }
        let left = self.compile_hir_expr(lhs)?;
        let right = self.compile_hir_expr(rhs)?;
        self.compile_binary_values(op, &left, &right, result_ty)
    }

    pub(crate) fn compile_hir_unary(
        &mut self,
        op: UnaryOp,
        inner: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_hir_expr(inner)?;
        self.compile_unary_values(op, val)
    }

    pub(crate) fn compile_hir_assign(
        &mut self,
        target: &HirExpr,
        value: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_assign_hir(target, value)
    }

    pub(crate) fn compile_hir_field_access(
        &mut self,
        obj: &HirExpr,
        field: &str,
    ) -> Result<TypedValue<'ctx>, String> {
        let obj_val = self.compile_hir_expr(obj)?;
        self.compile_field_access_value(obj_val, field)
    }

    pub(crate) fn compile_hir_lambda(
        &mut self,
        params: &[String],
        body: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_lambda_hir(params, body)
    }

    pub(crate) fn compile_hir_index(
        &mut self,
        obj: &HirExpr,
        idx: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::ast::Literal;
        use action_frontend::fallible_safety::hir_index_access_is_compile_time_safe;
        use action_frontend::hir::HirExprKind;

        if self.in_fallible_region() {
            let region_index = HirExpr {
                ty: obj.ty.clone(),
                span: obj.span,
                kind: HirExprKind::Index(Box::new(obj.clone()), Box::new(idx.clone())),
            };
            if let Some(v) = self.try_compile_fallible_expr(&region_index)? {
                return self.unwrap_fallible_value(v);
            }
        }

        let obj_val = self.compile_hir_expr(obj)?;
        let result = match obj_val {
            TypedValue::Map(map_ptr) => {
                let key_val = self.compile_hir_expr(idx)?;
                self.compile_map_index_key(map_ptr, key_val)?
            }
            TypedValue::Set(set_ptr) => {
                let elem_val = self.compile_hir_expr(idx)?;
                self.compile_set_index_key(set_ptr, elem_val)?
            }
            TypedValue::Struct(ptr, struct_ty) => {
                let index = match &idx.kind {
                    HirExprKind::Literal(Literal::Int(n)) => *n as u32,
                    _ => return Err("Tuple/struct index must be an integer literal".to_string()),
                };
                let bt: inkwell::types::BasicTypeEnum = struct_ty.into();
                let loaded = self
                    .builder
                    .build_load(bt, ptr, "tuple_ld")
                    .map_err(super::llvm_err)?;
                let struct_val = loaded.into_struct_value();
                let field_val = self
                    .builder
                    .build_extract_value(struct_val, index, "tuple_idx")
                    .map_err(super::llvm_err)?;
                self.bv_to_typed(field_val)?
            }
            other => {
                let idx_val = self.compile_hir_expr(idx)?;
                self.compile_index_values(other, idx_val)?
            }
        };
        if hir_index_access_is_compile_time_safe(obj, idx) {
            self.unwrap_fallible_value(result)
        } else {
            Ok(result)
        }
    }

    pub(crate) fn compile_hir_range(
        &mut self,
        start: &HirExpr,
        end: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let start_val = self.compile_hir_expr(start)?;
        let end_val = self.compile_hir_expr(end)?;
        self.compile_range_values(start_val, end_val)
    }

    pub(crate) fn compile_hir_struct_lit(
        &mut self,
        fields: &[(String, HirExpr)],
    ) -> Result<TypedValue<'ctx>, String> {
        let names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
        let mut vals = Vec::with_capacity(fields.len());
        for (_, e) in fields {
            vals.push(self.compile_hir_expr(e)?);
        }
        self.compile_struct_lit_values(&names, vals)
    }

    pub(crate) fn compile_hir_map_lit(
        &mut self,
        entries: &[(HirExpr, HirExpr)],
    ) -> Result<TypedValue<'ctx>, String> {
        let mut keys = Vec::with_capacity(entries.len());
        let mut vals = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            keys.push(self.compile_hir_expr(k)?);
            vals.push(self.compile_hir_expr(v)?);
        }
        self.compile_map_lit_values(&keys, &vals)
    }

    pub(crate) fn compile_hir_set_lit(
        &mut self,
        elements: &[HirExpr],
    ) -> Result<TypedValue<'ctx>, String> {
        let mut vals = Vec::with_capacity(elements.len());
        for e in elements {
            vals.push(self.compile_hir_expr(e)?);
        }
        self.compile_set_lit_values(&vals)
    }

    pub(crate) fn compile_hir_tuple(
        &mut self,
        items: &[(Option<String>, HirExpr)],
    ) -> Result<TypedValue<'ctx>, String> {
        let mut compiled = Vec::with_capacity(items.len());
        for (n, e) in items {
            compiled.push((n.clone(), self.compile_hir_expr(e)?));
        }
        self.compile_tuple_values(&compiled)
    }

    pub(crate) fn compile_hir_or_block(
        &mut self,
        fallible: &HirExpr,
        fallback: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_or_block_hir(fallible, fallback)
    }

    pub(crate) fn compile_hir_string_interp(
        &mut self,
        parts: &[HirStringPart],
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_string_interp_hir(parts)
    }

    pub(crate) fn compile_hir_block(
        &mut self,
        stmts: &[HirStmt],
        _result_ty: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        let mut saved = crate::Scope::new();
        std::mem::swap(&mut self.scope, &mut saved);
        self.scope = crate::Scope::with_parent(saved);

        self.block_did_rc_inc = false;

        let mut last = TypedValue::Unit;
        let mut i = 0;
        while i < stmts.len() {
            if let Some(n) = self.try_compile_filter_map_fold_stmt_chain(&stmts[i..])? {
                i += n;
                last = TypedValue::Unit;
                continue;
            }
            if let Some(n) = self.try_compile_map_filter_let_fusion(&stmts[i..])? {
                i += n;
                last = TypedValue::Unit;
                continue;
            }
            let s = &stmts[i];
            match s {
                HirStmt::Expr { expr, .. } => {
                    if self.try_compile_mutating_ufcs_stmt_writeback(expr)? {
                        last = TypedValue::Unit;
                        i += 1;
                        continue;
                    }
                    self.rc_discard_value(&last)?;
                    last = self.compile_hir_expr(expr)?;
                }
                _ => self.compile_hir_stmt(s)?,
            }
            i += 1;
        }

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("compile_hir_block: builder has no insert block")?;
        if current_block.get_terminator().is_none() {
            if self.is_scope_variable(&last) {
                self.rc_inc_typed_value(&last)?;
                self.block_did_rc_inc = true;
            } else {
                self.block_did_rc_inc = false;
            }
            self.emit_scope_cleanup()?;
        } else {
            self.block_did_rc_inc = false;
        }

        let mut parent = crate::Scope::new();
        std::mem::swap(&mut self.scope, &mut parent);
        if let Some(p) = parent.parent {
            self.scope = *p;
        }
        Ok(last)
    }

    pub(crate) fn compile_hir_call(
        &mut self,
        func: &HirExpr,
        args: &[HirExpr],
        trailing_lambda: Option<&Box<HirExpr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_call_hir(func, args, trailing_lambda)
    }
}
