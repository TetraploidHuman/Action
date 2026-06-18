//! Lower typed AST (`Program`) to HIR after type-checking.

use crate::ast::*;
use crate::hir::nodes::*;
use crate::typecheck::TypeChecker;
use action_span::Span;

/// Build HIR from a type-checked program.
pub fn lower_program(program: &Program, checker: &TypeChecker) -> HirModule {
    let lowerer = Lowerer { checker };
    HirModule {
        stmts: lowerer.lower_stmts(&program.stmts),
    }
}

struct Lowerer<'a> {
    checker: &'a TypeChecker,
}

impl<'a> Lowerer<'a> {
    fn lower_stmts(&self, stmts: &[Stmt]) -> Vec<HirStmt> {
        stmts.iter().map(|s| self.lower_stmt(s)).collect()
    }

    fn lower_stmt(&self, stmt: &Stmt) -> HirStmt {
        match stmt {
            Stmt::Let {
                mutable,
                lazy_init,
                name,
                type_ann,
                value,
                span,
            } => HirStmt::Let {
                mutable: *mutable,
                lazy_init: *lazy_init,
                name: name.clone(),
                type_ann: type_ann.clone(),
                value: self.lower_expr(value),
                span: *span,
            },
            Stmt::Destructure {
                mutable,
                names,
                renames,
                rest,
                is_list,
                is_struct,
                value,
                span,
            } => HirStmt::Destructure {
                mutable: *mutable,
                names: names.clone(),
                renames: renames.clone(),
                rest: rest.clone(),
                is_list: *is_list,
                is_struct: *is_struct,
                value: self.lower_expr(value),
                span: *span,
            },
            Stmt::Fun {
                name,
                params,
                return_type,
                body,
                type_params,
                is_single_expr,
                is_test,
                span,
            } => HirStmt::Fun {
                name: name.clone(),
                params: params.clone(),
                return_type: return_type.clone(),
                body: self.lower_expr(body),
                type_params: type_params.clone(),
                is_single_expr: *is_single_expr,
                is_test: *is_test,
                span: *span,
            },
            Stmt::Expr { expr, span } => HirStmt::Expr {
                expr: self.lower_expr(expr),
                span: *span,
            },
            Stmt::Return { value, span } => HirStmt::Return {
                value: value.as_ref().map(|e| self.lower_expr(e)),
                span: *span,
            },
            Stmt::Break { span } => HirStmt::Break { span: *span },
            Stmt::Continue { span } => HirStmt::Continue { span: *span },
            Stmt::TypeAlias {
                name,
                type_params,
                definition,
                span,
            } => HirStmt::TypeAlias {
                name: name.clone(),
                type_params: type_params.clone(),
                definition: definition.clone(),
                span: *span,
            },
            Stmt::Enum {
                name,
                type_params,
                variants,
                span,
            } => HirStmt::Enum {
                name: name.clone(),
                type_params: type_params.clone(),
                variants: variants.clone(),
                span: *span,
            },
            Stmt::Module {
                name,
                exports,
                body,
                span,
            } => HirStmt::Module {
                name: name.clone(),
                exports: exports.clone(),
                body: self.lower_stmts(body),
                span: *span,
            },
            Stmt::Export { stmt, span } => HirStmt::Export {
                stmt: Box::new(self.lower_stmt(stmt)),
                span: *span,
            },
            Stmt::Import {
                module,
                items,
                alias,
                span,
            } => HirStmt::Import {
                module: module.clone(),
                items: items.clone(),
                alias: alias.clone(),
                span: *span,
            },
            Stmt::Const {
                name,
                type_ann,
                value,
                span,
            } => HirStmt::Const {
                name: name.clone(),
                type_ann: type_ann.clone(),
                value: self.lower_expr(value),
                span: *span,
            },
            Stmt::Extension {
                type_name,
                methods,
                span,
            } => HirStmt::Extension {
                type_name: type_name.clone(),
                methods: self.lower_stmts(methods),
                span: *span,
            },
            Stmt::External {
                name,
                params,
                return_type,
                span,
            } => HirStmt::External {
                name: name.clone(),
                params: params.clone(),
                return_type: return_type.clone(),
                span: *span,
            },
            Stmt::ExternalType { name, span } => HirStmt::ExternalType {
                name: name.clone(),
                span: *span,
            },
        }
    }

    fn lower_expr(&self, expr: &Expr) -> HirExpr {
        let ty = self.checker.inferred_type(expr);
        let kind = match expr {
            Expr::Literal(l) => HirExprKind::Literal(l.clone()),
            Expr::Ident(n) => HirExprKind::Ident(n.clone()),
            Expr::Binary(lhs, op, rhs) => HirExprKind::Binary(
                Box::new(self.lower_expr(lhs)),
                *op,
                Box::new(self.lower_expr(rhs)),
            ),
            Expr::Unary(op, inner) => HirExprKind::Unary(*op, Box::new(self.lower_expr(inner))),
            Expr::Call {
                func,
                args,
                trailing_lambda,
            } => HirExprKind::Call {
                func: Box::new(self.lower_expr(func)),
                args: args.iter().map(|a| self.lower_expr(a)).collect(),
                trailing_lambda: trailing_lambda
                    .as_ref()
                    .map(|l| Box::new(self.lower_expr(l))),
            },
            Expr::Lambda {
                params,
                body,
                implicit_it,
            } => HirExprKind::Lambda {
                params: params.clone(),
                body: Box::new(self.lower_expr(body)),
                implicit_it: *implicit_it,
            },
            Expr::When(w) => HirExprKind::When(Box::new(self.lower_when(w))),
            Expr::For(f) => HirExprKind::For(Box::new(self.lower_for(f))),
            Expr::Block(stmts) => HirExprKind::Block(self.lower_stmts(stmts)),
            Expr::StructLiteral(fields) => HirExprKind::StructLiteral(
                fields
                    .iter()
                    .map(|(n, e)| (n.clone(), self.lower_expr(e)))
                    .collect(),
            ),
            Expr::MapLiteral(entries) => HirExprKind::MapLiteral(
                entries
                    .iter()
                    .map(|(k, v)| (self.lower_expr(k), self.lower_expr(v)))
                    .collect(),
            ),
            Expr::SetLiteral(items) => {
                HirExprKind::SetLiteral(items.iter().map(|e| self.lower_expr(e)).collect())
            }
            Expr::FieldAccess(obj, field) => {
                HirExprKind::FieldAccess(Box::new(self.lower_expr(obj)), field.clone())
            }
            Expr::Index(obj, idx) => HirExprKind::Index(
                Box::new(self.lower_expr(obj)),
                Box::new(self.lower_expr(idx)),
            ),
            Expr::Range(start, end) => HirExprKind::Range(
                Box::new(self.lower_expr(start)),
                Box::new(self.lower_expr(end)),
            ),
            Expr::Tuple(items) => HirExprKind::Tuple(
                items
                    .iter()
                    .map(|(n, e)| (n.clone(), self.lower_expr(e)))
                    .collect(),
            ),
            Expr::Null => HirExprKind::Null,
            Expr::OrBlock { nullable, fallback } => HirExprKind::OrBlock {
                nullable: Box::new(self.lower_expr(nullable)),
                fallback: Box::new(self.lower_expr(fallback)),
            },
            Expr::Assign { target, value } => HirExprKind::Assign {
                target: Box::new(self.lower_expr(target)),
                value: Box::new(self.lower_expr(value)),
            },
            Expr::StringInterpolate(parts) => HirExprKind::StringInterpolate(
                parts
                    .iter()
                    .map(|p| match p {
                        StringPart::Literal(s) => HirStringPart::Literal(s.clone()),
                        StringPart::Expr(e) => HirStringPart::Expr(Box::new(self.lower_expr(e))),
                    })
                    .collect(),
            ),
            Expr::Continue => HirExprKind::Continue,
            Expr::Break => HirExprKind::Break,
            Expr::FunctionRef(n) => HirExprKind::FunctionRef(n.clone()),
            Expr::Copy(inner) => HirExprKind::Copy(Box::new(self.lower_expr(inner))),
            Expr::Unsafe(inner) => HirExprKind::Unsafe(Box::new(self.lower_expr(inner))),
        };
        HirExpr {
            ty,
            span: Span::default(),
            kind,
        }
    }

    fn lower_when(&self, w: &When) -> HirWhen {
        HirWhen {
            kind: match &w.kind {
                WhenKind::OneLine {
                    condition,
                    then_expr,
                    else_expr,
                } => HirWhenKind::OneLine {
                    condition: Box::new(self.lower_expr(condition)),
                    then_expr: Box::new(self.lower_expr(then_expr)),
                    else_expr: Box::new(self.lower_expr(else_expr)),
                },
                WhenKind::ValueMatch { value, arms } => HirWhenKind::ValueMatch {
                    value: Box::new(self.lower_expr(value)),
                    arms: arms.iter().map(|a| self.lower_when_arm(a)).collect(),
                },
                WhenKind::ConditionChain { arms } => HirWhenKind::ConditionChain {
                    arms: arms.iter().map(|a| self.lower_when_arm(a)).collect(),
                },
            },
        }
    }

    fn lower_when_arm(&self, arm: &WhenArm) -> HirWhenArm {
        HirWhenArm {
            pattern: self.lower_pattern(&arm.pattern),
            guard: arm.guard.as_ref().map(|g| Box::new(self.lower_expr(g))),
            body: Box::new(self.lower_expr(&arm.body)),
        }
    }

    fn lower_pattern(&self, pattern: &Pattern) -> HirPattern {
        match pattern {
            Pattern::Wildcard => HirPattern::Wildcard,
            Pattern::Literal(l) => HirPattern::Literal(l.clone()),
            Pattern::Variable(n) => HirPattern::Variable(n.clone()),
            Pattern::Constructor {
                name,
                args,
                named_fields,
            } => HirPattern::Constructor {
                name: name.clone(),
                args: args.iter().map(|p| self.lower_pattern(p)).collect(),
                named_fields: named_fields
                    .iter()
                    .map(|(n, p)| (n.clone(), self.lower_pattern(p)))
                    .collect(),
            },
            Pattern::Range(start, end) => HirPattern::Range(
                Box::new(self.lower_expr(start)),
                Box::new(self.lower_expr(end)),
            ),
            Pattern::IsType(n) => HirPattern::IsType(n.clone()),
            Pattern::Or(ps) => HirPattern::Or(ps.iter().map(|p| self.lower_pattern(p)).collect()),
            Pattern::Expr(e) => HirPattern::Expr(Box::new(self.lower_expr(e))),
            Pattern::Null => HirPattern::Null,
            Pattern::Tuple(ps) => {
                HirPattern::Tuple(ps.iter().map(|p| self.lower_pattern(p)).collect())
            }
        }
    }

    fn lower_for(&self, f: &For) -> HirFor {
        HirFor {
            kind: match &f.kind {
                ForKind::Iterate {
                    var,
                    iterable,
                    body,
                    collect,
                } => HirForKind::Iterate {
                    var: var.clone(),
                    iterable: Box::new(self.lower_expr(iterable)),
                    body: Box::new(self.lower_expr(body)),
                    collect: *collect,
                },
                ForKind::IterateWithIndex {
                    vars,
                    iterable,
                    body,
                } => HirForKind::IterateWithIndex {
                    vars: vars.clone(),
                    iterable: Box::new(self.lower_expr(iterable)),
                    body: Box::new(self.lower_expr(body)),
                },
                ForKind::Condition { condition, body } => HirForKind::Condition {
                    condition: Box::new(self.lower_expr(condition)),
                    body: Box::new(self.lower_expr(body)),
                },
                ForKind::Infinite { body } => HirForKind::Infinite {
                    body: Box::new(self.lower_expr(body)),
                },
                ForKind::NestedIterate {
                    bindings,
                    body,
                    collect,
                } => HirForKind::NestedIterate {
                    bindings: bindings
                        .iter()
                        .map(|(v, e)| (v.clone(), self.lower_expr(e)))
                        .collect(),
                    body: Box::new(self.lower_expr(body)),
                    collect: *collect,
                },
            },
        }
    }
}
