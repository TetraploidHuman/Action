//! Convert HIR back to untyped AST (for dual-track codegen during transition).

use crate::ast::*;
use crate::hir::nodes::*;

impl HirModule {
    /// Strip type annotations and rebuild the source AST.
    pub fn to_program(&self) -> Program {
        Program {
            stmts: self.stmts.iter().map(HirStmt::to_stmt).collect(),
        }
    }
}

impl HirStmt {
    /// Convert to AST for codegen helpers that still accept `Stmt`.
    pub fn as_stmt(&self) -> Stmt {
        self.to_stmt()
    }

    fn to_stmt(&self) -> Stmt {
        match self {
            HirStmt::Let {
                mutable,
                lazy_init,
                name,
                type_ann,
                value,
                span,
            } => Stmt::Let {
                mutable: *mutable,
                lazy_init: *lazy_init,
                name: name.clone(),
                type_ann: type_ann.clone(),
                value: value.to_expr(),
                span: *span,
            },
            HirStmt::Destructure {
                mutable,
                names,
                renames,
                rest,
                is_list,
                is_struct,
                value,
                span,
            } => Stmt::Destructure {
                mutable: *mutable,
                names: names.clone(),
                renames: renames.clone(),
                rest: rest.clone(),
                is_list: *is_list,
                is_struct: *is_struct,
                value: value.to_expr(),
                span: *span,
            },
            HirStmt::Fun {
                name,
                params,
                return_type,
                body,
                type_params,
                is_single_expr,
                is_test,
                span,
            } => Stmt::Fun {
                name: name.clone(),
                params: params.clone(),
                return_type: return_type.clone(),
                body: body.to_expr(),
                type_params: type_params.clone(),
                is_single_expr: *is_single_expr,
                is_test: *is_test,
                span: *span,
            },
            HirStmt::Expr { expr, span } => Stmt::Expr {
                expr: expr.to_expr(),
                span: *span,
            },
            HirStmt::Return { value, span } => Stmt::Return {
                value: value.as_ref().map(|e| e.to_expr()),
                span: *span,
            },
            HirStmt::Break { span } => Stmt::Break { span: *span },
            HirStmt::Continue { span } => Stmt::Continue { span: *span },
            HirStmt::TypeAlias {
                name,
                type_params,
                definition,
                span,
            } => Stmt::TypeAlias {
                name: name.clone(),
                type_params: type_params.clone(),
                definition: definition.clone(),
                span: *span,
            },
            HirStmt::Enum {
                name,
                type_params,
                variants,
                span,
            } => Stmt::Enum {
                name: name.clone(),
                type_params: type_params.clone(),
                variants: variants.clone(),
                span: *span,
            },
            HirStmt::Module {
                name,
                exports,
                body,
                span,
            } => Stmt::Module {
                name: name.clone(),
                exports: exports.clone(),
                body: body.iter().map(HirStmt::to_stmt).collect(),
                span: *span,
            },
            HirStmt::Export { stmt, span } => Stmt::Export {
                stmt: Box::new(stmt.to_stmt()),
                span: *span,
            },
            HirStmt::Import {
                module,
                items,
                alias,
                span,
            } => Stmt::Import {
                module: module.clone(),
                items: items.clone(),
                alias: alias.clone(),
                span: *span,
            },
            HirStmt::Const {
                name,
                type_ann,
                value,
                span,
            } => Stmt::Const {
                name: name.clone(),
                type_ann: type_ann.clone(),
                value: value.to_expr(),
                span: *span,
            },
            HirStmt::Extension {
                type_name,
                methods,
                span,
            } => Stmt::Extension {
                type_name: type_name.clone(),
                methods: methods.iter().map(HirStmt::to_stmt).collect(),
                span: *span,
            },
            HirStmt::External {
                name,
                params,
                return_type,
                span,
            } => Stmt::External {
                name: name.clone(),
                params: params.clone(),
                return_type: return_type.clone(),
                span: *span,
            },
            HirStmt::ExternalType { name, span } => Stmt::ExternalType {
                name: name.clone(),
                span: *span,
            },
        }
    }
}

impl HirExpr {
    /// Convert to AST for codegen helpers that still accept `Expr`.
    pub fn as_expr(&self) -> Expr {
        self.to_expr()
    }

    fn to_expr(&self) -> Expr {
        match &self.kind {
            HirExprKind::Literal(l) => ExprKind::Literal(l.clone()).into(),
            HirExprKind::Ident(n) => ExprKind::Ident(n.clone()).into(),
            HirExprKind::Binary(lhs, op, rhs) => Expr::binary(lhs.to_expr(), *op, rhs.to_expr()),
            HirExprKind::Unary(op, inner) => Expr::unary(*op, inner.to_expr()),
            HirExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => ExprKind::Call {
                func: Box::new(func.to_expr()),
                args: args.iter().map(HirExpr::to_expr).collect(),
                trailing_lambda: trailing_lambda.as_ref().map(|l| Box::new(l.to_expr())),
            }
            .into(),
            HirExprKind::Lambda {
                params,
                body,
                implicit_it,
            } => ExprKind::Lambda {
                params: params.clone(),
                body: Box::new(body.to_expr()),
                implicit_it: *implicit_it,
            }
            .into(),
            HirExprKind::When(w) => ExprKind::When(Box::new(w.to_when())).into(),
            HirExprKind::For(f) => ExprKind::For(Box::new(f.to_for())).into(),
            HirExprKind::Block(stmts) => {
                ExprKind::Block(stmts.iter().map(HirStmt::to_stmt).collect()).into()
            }
            HirExprKind::StructLiteral(fields) => ExprKind::StructLiteral(
                fields
                    .iter()
                    .map(|(n, e)| (n.clone(), e.to_expr()))
                    .collect(),
            )
            .into(),
            HirExprKind::MapLiteral(entries) => ExprKind::MapLiteral(
                entries
                    .iter()
                    .map(|(k, v)| (k.to_expr(), v.to_expr()))
                    .collect(),
            )
            .into(),
            HirExprKind::SetLiteral(items) => {
                ExprKind::SetLiteral(items.iter().map(HirExpr::to_expr).collect()).into()
            }
            HirExprKind::FieldAccess(obj, field) => {
                ExprKind::FieldAccess(Box::new(obj.to_expr()), field.clone()).into()
            }
            HirExprKind::Index(obj, idx) => {
                ExprKind::Index(Box::new(obj.to_expr()), Box::new(idx.to_expr())).into()
            }
            HirExprKind::Range(start, end) => {
                ExprKind::Range(Box::new(start.to_expr()), Box::new(end.to_expr())).into()
            }
            HirExprKind::Tuple(items) => ExprKind::Tuple(
                items
                    .iter()
                    .map(|(n, e)| (n.clone(), e.to_expr()))
                    .collect(),
            )
            .into(),
            HirExprKind::Null => ExprKind::Null.into(),
            HirExprKind::OrBlock { nullable, fallback } => ExprKind::OrBlock {
                nullable: Box::new(nullable.to_expr()),
                fallback: Box::new(fallback.to_expr()),
            }
            .into(),
            HirExprKind::Assign { target, value } => ExprKind::Assign {
                target: Box::new(target.to_expr()),
                value: Box::new(value.to_expr()),
            }
            .into(),
            HirExprKind::StringInterpolate(parts) => ExprKind::StringInterpolate(
                parts
                    .iter()
                    .map(|p| match p {
                        HirStringPart::Literal(s) => StringPart::Literal(s.clone()),
                        HirStringPart::Expr(e) => StringPart::Expr(Box::new(e.to_expr())),
                    })
                    .collect(),
            )
            .into(),
            HirExprKind::Continue => ExprKind::Continue.into(),
            HirExprKind::Break => ExprKind::Break.into(),
            HirExprKind::FunctionRef(n) => ExprKind::FunctionRef(n.clone()).into(),
            HirExprKind::Copy(inner) => ExprKind::Copy(Box::new(inner.to_expr())).into(),
            HirExprKind::Unsafe(inner) => ExprKind::Unsafe(Box::new(inner.to_expr())).into(),
        }
    }
}

impl HirWhen {
    pub fn to_when(&self) -> When {
        When {
            kind: match &self.kind {
                HirWhenKind::OneLine {
                    condition,
                    then_expr,
                    else_expr,
                } => WhenKind::OneLine {
                    condition: Box::new(condition.to_expr()),
                    then_expr: Box::new(then_expr.to_expr()),
                    else_expr: Box::new(else_expr.to_expr()),
                },
                HirWhenKind::ValueMatch { value, arms } => WhenKind::ValueMatch {
                    value: Box::new(value.to_expr()),
                    arms: arms.iter().map(HirWhenArm::to_when_arm).collect(),
                },
                HirWhenKind::ConditionChain { arms } => WhenKind::ConditionChain {
                    arms: arms.iter().map(HirWhenArm::to_when_arm).collect(),
                },
            },
        }
    }
}

impl HirWhenArm {
    fn to_when_arm(&self) -> WhenArm {
        WhenArm {
            pattern: self.pattern.to_pattern(),
            guard: self.guard.as_ref().map(|g| Box::new(g.to_expr())),
            body: Box::new(self.body.to_expr()),
        }
    }
}

impl HirPattern {
    fn to_pattern(&self) -> Pattern {
        match self {
            HirPattern::Wildcard => Pattern::Wildcard,
            HirPattern::Literal(l) => Pattern::Literal(l.clone()),
            HirPattern::Variable(n) => Pattern::Variable(n.clone()),
            HirPattern::Constructor {
                name,
                args,
                named_fields,
            } => Pattern::Constructor {
                name: name.clone(),
                args: args.iter().map(HirPattern::to_pattern).collect(),
                named_fields: named_fields
                    .iter()
                    .map(|(n, p)| (n.clone(), p.to_pattern()))
                    .collect(),
            },
            HirPattern::Range(start, end) => {
                Pattern::Range(Box::new(start.to_expr()), Box::new(end.to_expr()))
            }
            HirPattern::IsType(n) => Pattern::IsType(n.clone()),
            HirPattern::Or(ps) => Pattern::Or(ps.iter().map(HirPattern::to_pattern).collect()),
            HirPattern::Expr(e) => Pattern::Expr(Box::new(e.to_expr())),
            HirPattern::Null => Pattern::Null,
            HirPattern::Tuple(ps) => {
                Pattern::Tuple(ps.iter().map(HirPattern::to_pattern).collect())
            }
        }
    }
}

impl HirFor {
    pub fn to_for(&self) -> For {
        For {
            kind: match &self.kind {
                HirForKind::Iterate {
                    var,
                    iterable,
                    body,
                    collect,
                } => ForKind::Iterate {
                    var: var.clone(),
                    iterable: Box::new(iterable.to_expr()),
                    body: Box::new(body.to_expr()),
                    collect: *collect,
                },
                HirForKind::IterateWithIndex {
                    vars,
                    iterable,
                    body,
                } => ForKind::IterateWithIndex {
                    vars: vars.clone(),
                    iterable: Box::new(iterable.to_expr()),
                    body: Box::new(body.to_expr()),
                },
                HirForKind::Condition { condition, body } => ForKind::Condition {
                    condition: Box::new(condition.to_expr()),
                    body: Box::new(body.to_expr()),
                },
                HirForKind::Infinite { body } => ForKind::Infinite {
                    body: Box::new(body.to_expr()),
                },
                HirForKind::NestedIterate {
                    bindings,
                    body,
                    collect,
                } => ForKind::NestedIterate {
                    bindings: bindings
                        .iter()
                        .map(|(v, e)| (v.clone(), e.to_expr()))
                        .collect(),
                    body: Box::new(body.to_expr()),
                    collect: *collect,
                },
            },
        }
    }
}
