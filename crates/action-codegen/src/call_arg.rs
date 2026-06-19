//! Unified call-argument source for AST and HIR codegen paths.

use action_frontend::ast::Expr;
use action_frontend::hir::HirExpr;

use super::{CodeGen, TypedValue};

/// A call argument compiled from either AST or HIR.
#[derive(Clone, Copy)]
pub enum CallArg<'a> {
    Ast(&'a Expr),
    Hir(&'a HirExpr),
}

impl<'a> CallArg<'a> {
    pub fn ast(e: &'a Expr) -> Self {
        CallArg::Ast(e)
    }

    pub fn hir(e: &'a HirExpr) -> Self {
        CallArg::Hir(e)
    }
}

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        match arg {
            CallArg::Ast(e) => self.compile_expr(e),
            CallArg::Hir(e) => self.compile_hir_expr(e),
        }
    }

    pub(super) fn compile_and_load_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>, String> {
        match arg {
            CallArg::Ast(e) => self.compile_and_load(e),
            CallArg::Hir(e) => self.compile_and_load_hir(e),
        }
    }

    pub(super) fn call_args_from_ast(args: &[Expr]) -> Vec<CallArg<'_>> {
        args.iter().map(CallArg::ast).collect()
    }

    pub(super) fn call_args_from_hir(args: &[HirExpr]) -> Vec<CallArg<'_>> {
        args.iter().map(CallArg::hir).collect()
    }

    pub(super) fn trailing_call_arg(trailing: &Option<Box<Expr>>) -> Option<CallArg<'_>> {
        trailing.as_ref().map(|b| CallArg::ast(b.as_ref()))
    }

    pub(super) fn call_arg_to_expr(arg: CallArg<'_>) -> Expr {
        match arg {
            CallArg::Ast(e) => e.clone(),
            CallArg::Hir(h) => h.as_expr(),
        }
    }

    pub(super) fn trailing_call_arg_hir(trailing: Option<&Box<HirExpr>>) -> Option<CallArg<'_>> {
        trailing.map(|b| CallArg::hir(b.as_ref()))
    }
}
