//! HIR node definitions: typed AST mirror for codegen / bootstrap boundary.

use crate::ast::{BinaryOp, EnumVariant, ExportItem, Literal, Param, Type, UnaryOp};
use action_span::Span;
use serde::{Deserialize, Serialize};

/// A fully type-checked program ready for lowering to LLVM (or serialization).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirModule {
    pub stmts: Vec<HirStmt>,
}

/// Top-level or nested statement with resolved types on expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirStmt {
    Let {
        mutable: bool,
        lazy_init: bool,
        name: String,
        type_ann: Option<Type>,
        value: HirExpr,
        span: Span,
    },
    Destructure {
        mutable: bool,
        names: Vec<String>,
        renames: Vec<(String, String)>,
        rest: Option<String>,
        is_list: bool,
        is_struct: bool,
        value: HirExpr,
        span: Span,
    },
    Fun {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        body: HirExpr,
        type_params: Vec<String>,
        is_single_expr: bool,
        is_test: bool,
        fn_or_fallback: Option<HirExpr>,
        span: Span,
    },
    Expr {
        expr: HirExpr,
        span: Span,
    },
    Return {
        value: Option<HirExpr>,
        span: Span,
    },
    Break {
        span: Span,
    },
    Continue {
        span: Span,
    },
    TypeAlias {
        name: String,
        type_params: Vec<String>,
        definition: Type,
        span: Span,
    },
    Enum {
        name: String,
        type_params: Vec<String>,
        variants: Vec<EnumVariant>,
        span: Span,
    },
    Module {
        name: String,
        exports: Vec<ExportItem>,
        body: Vec<HirStmt>,
        span: Span,
    },
    Export {
        stmt: Box<HirStmt>,
        span: Span,
    },
    Import {
        module: String,
        items: Option<Vec<String>>,
        alias: Option<String>,
        span: Span,
    },
    Const {
        name: String,
        type_ann: Option<Type>,
        value: HirExpr,
        span: Span,
    },
    Extension {
        type_name: String,
        methods: Vec<HirStmt>,
        span: Span,
    },
    External {
        name: String,
        params: Vec<Param>,
        return_type: Option<Type>,
        span: Span,
    },
    ExternalType {
        name: String,
        span: Span,
    },
}

impl HirStmt {
    pub fn span(&self) -> Span {
        match self {
            HirStmt::Let { span, .. }
            | HirStmt::Destructure { span, .. }
            | HirStmt::Fun { span, .. }
            | HirStmt::Expr { span, .. }
            | HirStmt::Return { span, .. }
            | HirStmt::Break { span }
            | HirStmt::Continue { span }
            | HirStmt::TypeAlias { span, .. }
            | HirStmt::Enum { span, .. }
            | HirStmt::Module { span, .. }
            | HirStmt::Export { span, .. }
            | HirStmt::Import { span, .. }
            | HirStmt::Const { span, .. }
            | HirStmt::Extension { span, .. }
            | HirStmt::External { span, .. }
            | HirStmt::ExternalType { span, .. } => *span,
        }
    }
}

/// Expression with inferred type annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirExpr {
    pub ty: Type,
    pub span: Span,
    pub kind: HirExprKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirExprKind {
    Literal(Literal),
    Ident(String),
    Binary(Box<HirExpr>, BinaryOp, Box<HirExpr>),
    Unary(UnaryOp, Box<HirExpr>),
    Call {
        func: Box<HirExpr>,
        args: Vec<HirExpr>,
        trailing_lambda: Option<Box<HirExpr>>,
    },
    Lambda {
        params: Vec<String>,
        body: Box<HirExpr>,
        implicit_it: bool,
    },
    When(Box<HirWhen>),
    For(Box<HirFor>),
    Block(Vec<HirStmt>),
    StructLiteral(Vec<(String, HirExpr)>),
    MapLiteral(Vec<(HirExpr, HirExpr)>),
    SetLiteral(Vec<HirExpr>),
    FieldAccess(Box<HirExpr>, String),
    Index(Box<HirExpr>, Box<HirExpr>),
    Range(Box<HirExpr>, Box<HirExpr>),
    Tuple(Vec<(Option<String>, HirExpr)>),
    Null,
    OrBlock {
        nullable: Box<HirExpr>,
        fallback: Box<HirExpr>,
    },
    Assign {
        target: Box<HirExpr>,
        value: Box<HirExpr>,
    },
    StringInterpolate(Vec<HirStringPart>),
    Continue,
    Break,
    FunctionRef(String),
    Copy(Box<HirExpr>),
    Unsafe(Box<HirExpr>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirStringPart {
    Literal(String),
    Expr(Box<HirExpr>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirWhen {
    pub kind: HirWhenKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirWhenKind {
    OneLine {
        condition: Box<HirExpr>,
        then_expr: Box<HirExpr>,
        else_expr: Box<HirExpr>,
    },
    ValueMatch {
        value: Box<HirExpr>,
        arms: Vec<HirWhenArm>,
    },
    ConditionChain {
        arms: Vec<HirWhenArm>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirWhenArm {
    pub pattern: HirPattern,
    pub guard: Option<Box<HirExpr>>,
    pub body: Box<HirExpr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirPattern {
    Wildcard,
    Literal(Literal),
    Variable(String),
    Constructor {
        name: String,
        args: Vec<HirPattern>,
        named_fields: Vec<(String, HirPattern)>,
    },
    Range(Box<HirExpr>, Box<HirExpr>),
    IsType(String),
    Or(Vec<HirPattern>),
    Expr(Box<HirExpr>),
    Null,
    Tuple(Vec<HirPattern>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirFor {
    pub kind: HirForKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirForKind {
    Iterate {
        var: String,
        iterable: Box<HirExpr>,
        body: Box<HirExpr>,
        collect: bool,
    },
    IterateWithIndex {
        vars: Vec<String>,
        iterable: Box<HirExpr>,
        body: Box<HirExpr>,
    },
    Condition {
        condition: Box<HirExpr>,
        body: Box<HirExpr>,
    },
    Infinite {
        body: Box<HirExpr>,
    },
    NestedIterate {
        bindings: Vec<(String, HirExpr)>,
        body: Box<HirExpr>,
        collect: bool,
    },
}

impl HirModule {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
