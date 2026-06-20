//! Unified call-argument source for AST and HIR codegen paths.

use action_frontend::ast::Type;
#[cfg(test)]
use action_frontend::ast::{Expr, ExprKind};
use action_frontend::hir::{HirExpr, HirExprKind};

use super::{CodeGen, TypedValue};

/// Synthetic HIR ident for lambda bodies (curry / nullable UFCS).
pub(super) fn synthetic_hir_ident(name: impl Into<String>) -> HirExpr {
    HirExpr {
        ty: Type::Named("Int".into()),
        span: default_hir_span(),
        kind: HirExprKind::Ident(name.into()),
    }
}

fn default_hir_span() -> action_frontend::lexer::Span {
    action_frontend::lexer::Span::default()
}

/// A call argument compiled from either AST or HIR.
#[derive(Clone, Copy)]
pub enum CallArg<'a> {
    #[cfg(test)]
    Ast(&'a Expr),
    Hir(&'a HirExpr),
}

impl<'a> CallArg<'a> {
    #[cfg(test)]
    pub fn ast(e: &'a Expr) -> Self {
        CallArg::Ast(e)
    }

    pub fn hir(e: &'a HirExpr) -> Self {
        CallArg::Hir(e)
    }
}

/// Trailing block body extracted from a zero-param lambda CallArg.
pub enum TrailingBody<'a> {
    #[cfg(test)]
    Ast(&'a Expr),
    Hir(&'a HirExpr),
}

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        match arg {
            #[cfg(test)]
            CallArg::Ast(e) => self.compile_expr(e),
            CallArg::Hir(e) => self.compile_hir_expr(e),
        }
    }

    pub(super) fn compile_and_load_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>, String> {
        match arg {
            #[cfg(test)]
            CallArg::Ast(e) => self.compile_and_load(e),
            CallArg::Hir(e) => self.compile_and_load_hir(e),
        }
    }

    #[cfg(test)]
    pub(super) fn call_args_from_ast(args: &[Expr]) -> Vec<CallArg<'_>> {
        args.iter().map(CallArg::ast).collect()
    }

    pub(super) fn call_args_from_hir(args: &[HirExpr]) -> Vec<CallArg<'_>> {
        args.iter().map(CallArg::hir).collect()
    }

    #[cfg(test)]
    pub(super) fn trailing_call_arg(trailing: &Option<Box<Expr>>) -> Option<CallArg<'_>> {
        trailing.as_ref().map(|b| CallArg::ast(b.as_ref()))
    }

    pub(super) fn trailing_call_arg_hir(trailing: Option<&Box<HirExpr>>) -> Option<CallArg<'_>> {
        trailing.map(|b| CallArg::hir(b.as_ref()))
    }

    pub(super) fn call_arg_ident_name(arg: CallArg<'_>) -> Option<&str> {
        match arg {
            #[cfg(test)]
            CallArg::Ast(e) => match &e.kind {
                ExprKind::Ident(name) => Some(name.as_str()),
                _ => None,
            },
            CallArg::Hir(h) => match &h.kind {
                HirExprKind::Ident(name) => Some(name.as_str()),
                _ => None,
            },
        }
    }

    pub(super) fn extract_trailing_block_body(
        trailing: CallArg<'_>,
    ) -> Result<TrailingBody<'_>, String> {
        match trailing {
            #[cfg(test)]
            CallArg::Ast(e) => match &e.kind {
                ExprKind::Lambda { params, body, .. } if params.is_empty() => {
                    Ok(TrailingBody::Ast(body))
                }
                _ => Err("expected a block body: trailing `{ ... }`".to_string()),
            },
            CallArg::Hir(h) => match &h.kind {
                HirExprKind::Lambda { params, body, .. } if params.is_empty() => {
                    Ok(TrailingBody::Hir(body))
                }
                _ => Err("expected a block body: trailing `{ ... }`".to_string()),
            },
        }
    }

    pub(super) fn compile_trailing_body(
        &mut self,
        body: TrailingBody<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        match body {
            #[cfg(test)]
            TrailingBody::Ast(e) => self.compile_expr(e),
            TrailingBody::Hir(h) => self.compile_hir_expr(h),
        }
    }

    pub(super) fn parse_launch_scheduler(arg: CallArg<'_>) -> Result<i64, String> {
        match arg {
            #[cfg(test)]
            CallArg::Ast(e) => match &e.kind {
                ExprKind::Ident(s) if s == "io" => Ok(1),
                ExprKind::Ident(s) if s == "cpu" => Ok(2),
                _ => Err("launch scheduler must be 'io' or 'cpu'".to_string()),
            },
            CallArg::Hir(h) => match &h.kind {
                HirExprKind::Ident(s) if s == "io" => Ok(1),
                HirExprKind::Ident(s) if s == "cpu" => Ok(2),
                _ => Err("launch scheduler must be 'io' or 'cpu'".to_string()),
            },
        }
    }

    pub(super) fn compile_synthetic_call_call_args(
        &mut self,
        func: CallArg<'_>,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let target = self.compile_call_arg(func)?;
        self.compile_indirect_call_from_call_args(target, args, trailing)
    }

    pub(super) fn compile_compose_call_args(
        &mut self,
        f: CallArg<'_>,
        g: CallArg<'_>,
        x: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let inner = self.compile_synthetic_call_call_args(g, &[x], None)?;
        let f_val = self.compile_call_arg(f)?;
        self.compile_indirect_call_with_precompiled_args(f_val, &[inner], None)
    }

    pub(super) fn compile_flip_call_args(
        &mut self,
        f: CallArg<'_>,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_synthetic_call_call_args(f, &[b, a], None)
    }

    pub(super) fn compile_uncurry_call_args(
        &mut self,
        f: CallArg<'_>,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let partial = self.compile_synthetic_call_call_args(f, &[a], None)?;
        self.compile_indirect_call_from_call_args(partial, &[b], None)
    }

    pub(super) fn compile_curry_call_args(
        &mut self,
        f: CallArg<'_>,
        a: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_lambda_impl(
            &["b".to_string()],
            |params, bound, free| {
                Self::collect_call_arg_free_vars(f, params, bound, free);
                Self::collect_call_arg_free_vars(a, params, bound, free);
            },
            |this| {
                let b_ident = synthetic_hir_ident("b");
                this.compile_synthetic_call_call_args(f, &[a, CallArg::hir(&b_ident)], None)
            },
        )
    }

    pub(super) fn collect_call_arg_free_vars(
        arg: CallArg<'_>,
        params: &[String],
        bound: &mut Vec<String>,
        free: &mut Vec<String>,
    ) {
        match arg {
            #[cfg(test)]
            CallArg::Ast(e) => super::expr::collect_free_vars(e, params, bound, free),
            CallArg::Hir(h) => super::expr::collect_free_vars_hir(h, params, bound, free),
        }
    }
}
