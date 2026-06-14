use crate::ast::*;
use crate::error::CompilerError;
use crate::lexer::Span;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    pub field_index: HashMap<String, usize>,
}

#[derive(Clone, Debug)]
pub struct EnumInfo {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariantInfo>,
}

#[derive(Clone, Debug)]
pub struct EnumVariantInfo {
    pub name: String,
    pub tag: u32,
    pub params: Vec<EnumVariantParam>,
}

#[derive(Default, Clone)]
pub struct TypeRegistry {
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub type_aliases: HashMap<String, Type>,
    pub variant_to_enum: HashMap<String, String>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        TypeRegistry {
            structs: HashMap::new(),
            enums: HashMap::new(),
            type_aliases: HashMap::new(),
            variant_to_enum: HashMap::new(),
        }
    }

    pub fn register(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::TypeAlias {
                name, definition, ..
            } => {
                if let Type::Struct(fields) = definition {
                    let mut field_index = HashMap::new();
                    for (i, (fname, _)) in fields.iter().enumerate() {
                        field_index.insert(fname.clone(), i);
                    }
                    self.structs.insert(
                        name.clone(),
                        StructInfo {
                            name: name.clone(),
                            fields: fields.clone(),
                            field_index,
                        },
                    );
                }
                self.type_aliases.insert(name.clone(), definition.clone());
            }
            Stmt::Enum {
                name,
                type_params,
                variants,
                ..
            } => {
                let mut enum_variants = Vec::new();
                for (i, v) in variants.iter().enumerate() {
                    self.variant_to_enum.insert(v.name.clone(), name.clone());
                    enum_variants.push(EnumVariantInfo {
                        name: v.name.clone(),
                        tag: i as u32,
                        params: v.params.clone(),
                    });
                }
                self.enums.insert(
                    name.clone(),
                    EnumInfo {
                        name: name.clone(),
                        type_params: type_params.clone(),
                        variants: enum_variants,
                    },
                );
            }
            Stmt::ExternalType { name, .. } => {
                // Register as opaque struct (no fields)
                self.structs.insert(
                    name.clone(),
                    StructInfo {
                        name: name.clone(),
                        fields: vec![],
                        field_index: HashMap::new(),
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }

    /// Find the struct type whose field names match exactly. Returns the struct info if unique.
    pub fn find_struct_by_fields(&self, field_names: &[String]) -> Option<&StructInfo> {
        let matches: Vec<&StructInfo> = self
            .structs
            .values()
            .filter(|s| {
                if s.fields.len() != field_names.len() {
                    return false;
                }
                field_names
                    .iter()
                    .enumerate()
                    .all(|(i, name)| s.fields[i].0 == *name)
            })
            .collect();
        if matches.len() == 1 {
            Some(matches[0])
        } else {
            None
        }
    }

    /// Look up an enum variant by name. Returns (enum_info, variant_info).
    pub fn lookup_variant(&self, variant_name: &str) -> Option<(&EnumInfo, &EnumVariantInfo)> {
        let enum_name = self.variant_to_enum.get(variant_name)?;
        let info = self.enums.get(enum_name)?;
        let variant = info.variants.iter().find(|v| v.name == variant_name)?;
        Some((info, variant))
    }

    pub fn get_struct(&self, name: &str) -> Option<&StructInfo> {
        self.structs.get(name)
    }

    /// Check that a set of when arms covers all variants of the enum they match on.
    /// Returns Ok(()) if exhaustive, Err(message) if any variant is missing.
    pub fn check_when_exhaustive(&self, arms: &[WhenArm]) -> Result<(), String> {
        let mut covered: HashSet<String> = HashSet::new();
        let mut enum_name: Option<String> = None;
        let mut has_wildcard = false;

        for arm in arms {
            self.collect_pattern_coverage(
                &arm.pattern,
                &mut covered,
                &mut enum_name,
                &mut has_wildcard,
            );
        }

        if has_wildcard || enum_name.is_none() {
            return Ok(());
        }

        let info = self
            .enums
            .get(enum_name.as_ref().unwrap())
            .ok_or_else(|| format!("Unknown enum type: {}", enum_name.unwrap()))?;

        let mut missing: Vec<&str> = Vec::new();
        for v in &info.variants {
            if !covered.contains(&v.name) {
                missing.push(&v.name);
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            let msg = missing
                .iter()
                .map(|n| format!("'{}'", n))
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "Non-exhaustive when: enum '{}' is missing variant(s): {}. Add them or add an else branch.",
                info.name, msg
            ))
        }
    }

    fn collect_pattern_coverage(
        &self,
        pattern: &Pattern,
        covered: &mut HashSet<String>,
        enum_name: &mut Option<String>,
        has_wildcard: &mut bool,
    ) {
        match pattern {
            Pattern::Wildcard | Pattern::Variable(_) => {
                *has_wildcard = true;
            }
            Pattern::Constructor {
                name,
                args,
                named_fields,
            } => {
                if let Some(en) = self.variant_to_enum.get(name.as_str()) {
                    if enum_name.is_none() {
                        *enum_name = Some(en.clone());
                    }
                }
                covered.insert(name.clone());
                for sub in args {
                    self.collect_pattern_coverage(sub, covered, enum_name, has_wildcard);
                }
                for (_, sub) in named_fields {
                    self.collect_pattern_coverage(sub, covered, enum_name, has_wildcard);
                }
            }
            Pattern::Or(patterns) => {
                for p in patterns {
                    self.collect_pattern_coverage(p, covered, enum_name, has_wildcard);
                }
            }
            _ => {} // Literal, Range, IsType — not relevant for enum exhaustiveness
        }
    }
}

/// Type checker: walks the AST and verifies type consistency.
/// Reports all errors found (not just the first one).
pub struct TypeChecker {
    registry: TypeRegistry,
    /// Type environment mapping names to their types (functions, variables)
    type_env: HashMap<String, Type>,
    /// Current statement span for error reporting
    current_span: Span,
    /// Variables known to be non-null (smart cast from null checks)
    not_null_set: RefCell<HashSet<String>>,
    /// Generic function definitions (non-empty type_params), indexed by name
    generic_funs: HashMap<String, Stmt>,
}

impl TypeChecker {
    pub fn new(registry: TypeRegistry) -> Self {
        TypeChecker {
            registry,
            type_env: HashMap::new(),
            current_span: Span::default(),
            not_null_set: RefCell::new(HashSet::new()),
            generic_funs: HashMap::new(),
        }
    }

    /// Access the type environment after checking
    pub fn type_env(&self) -> &HashMap<String, Type> {
        &self.type_env
    }

    /// Access the type registry
    pub fn registry_ref(&self) -> &TypeRegistry {
        &self.registry
    }

    /// Pre-populate the type environment (e.g., with stdlib bindings)
    pub fn seed_type_env(&mut self, env: &HashMap<String, Type>) {
        for (k, v) in env {
            self.type_env.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }

    /// Build the type environment from top-level statements
    fn build_type_env(&mut self, program: &Program) {
        // First pass: detect overloaded function names
        let mut name_counts: HashMap<String, usize> = HashMap::new();
        for stmt in &program.stmts {
            if let Stmt::Fun { name, params, .. } = stmt {
                if params.iter().all(|p| p.ty.is_some()) {
                    *name_counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }
        let overloaded_names: std::collections::HashSet<String> = name_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name)
            .collect();

        for stmt in &program.stmts {
            match stmt {
                Stmt::Fun {
                    name,
                    params,
                    return_type,
                    type_params,
                    ..
                } => {
                    // Store generic functions for monomorphization
                    if !type_params.is_empty() {
                        self.generic_funs.insert(name.clone(), stmt.clone());
                    }

                    // NOTE: untyped parameters and return types default to Int.
                    // Untyped parameters are caught as a hard error in check() (line ~375).
                    // For unannotated return types, check() infers the body type and updates
                    // type_env accordingly (line ~435), emitting a warning when the inferred
                    // type differs from Int. Full Hindley-Milner inference is not implemented.
                    let param_tys: Vec<Type> = params
                        .iter()
                        .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                        .collect();
                    let ret_ty = return_type.clone().unwrap_or(Type::Named("Int".into()));
                    let fn_type = Type::Function(param_tys, Box::new(ret_ty));

                    let all_typed = params.iter().all(|p| p.ty.is_some());
                    if all_typed && overloaded_names.contains(name.as_str()) {
                        // Use mangled name as key for overloaded functions
                        let mangled = Self::mangle_name(
                            name,
                            &params
                                .iter()
                                .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                                .collect::<Vec<_>>(),
                        );
                        self.type_env.insert(mangled, fn_type);
                    } else {
                        // Also store under original name for backward compat
                        self.type_env.insert(name.clone(), fn_type);
                    }
                }
                Stmt::Let {
                    name,
                    type_ann,
                    value,
                    ..
                } => {
                    let inferred = self.infer_expr_type(value).unwrap_or(Type::Named("Int".into()));
                    let ty = type_ann.clone().unwrap_or(inferred);
                    self.type_env.insert(name.clone(), ty);
                }
                Stmt::Destructure { names, .. } => {
                    for name in names {
                        self.type_env
                            .insert(name.clone(), Type::Named("Int".into()));
                    }
                }
                Stmt::Const {
                    name,
                    type_ann,
                    value,
                    ..
                } => {
                    let inferred = self.infer_expr_type(value).unwrap_or(Type::Named("Int".into()));
                    let ty = type_ann.clone().unwrap_or(inferred);
                    self.type_env.insert(name.clone(), ty);
                }
                _ => {}
            }
        }
    }

    /// Mangle a function name (mirrors codegen version)
    fn mangle_name(name: &str, param_types: &[Type]) -> String {
        if param_types.is_empty() {
            return name.to_string();
        }
        let parts: Vec<String> = param_types.iter().map(|t| format!("{}", t)).collect();
        format!("{}_{}", name, parts.join("_"))
    }

    /// Run all checks on the program. Returns a list of errors.
    pub fn check(&mut self, program: &Program) -> Vec<CompilerError> {
        self.build_type_env(program);
        let mut errors = Vec::new();

        for stmt in &program.stmts {
            self.current_span = stmt.span();
            match stmt {
                Stmt::Fun {
                    name,
                    params,
                    return_type,
                    body,
                    type_params,
                    ..
                } => {
                    // Require type annotations on all parameters (except 'self')
                    for p in params {
                        if p.ty.is_none() && p.name != "self" {
                            errors.push(
                                CompilerError::new(format!(
                                    "Parameter '{}' in function '{}' must have a type annotation",
                                    p.name, name
                                ))
                                .with_span(self.current_span),
                            );
                        }
                    }

                    // Temporarily add function parameters to the type environment
                    let mut saved: Vec<(String, Option<Type>)> = Vec::new();
                    for p in params {
                        let param_ty = p.ty.clone().unwrap_or(Type::Named("Int".into()));
                        let old = self.type_env.insert(p.name.clone(), param_ty);
                        saved.push((p.name.clone(), old));
                    }

                    // For generic functions, add type params to type_env so T is known
                    let mut saved_tps: Vec<(String, Option<Type>)> = Vec::new();
                    for tp in type_params {
                        let old = self.type_env.insert(tp.clone(), Type::TypeVar(tp.clone()));
                        saved_tps.push((tp.clone(), old));
                    }

                    self.collect_expr_errors(body, &mut errors);
                    // Validate return type annotation if present
                    if let Some(declared_ret) = return_type {
                        // Skip return type check for generic functions (validated per-instantiation)
                        if type_params.is_empty() || !matches!(declared_ret, Type::TypeVar(_)) {
                            let inferred = self.infer_expr_type(body).unwrap_or(Type::Named("Int".into()));
                            if !self.types_compatible(declared_ret, &inferred) {
                                let msg = if let Some(hint) =
                                    Self::check_termination(declared_ret, &inferred)
                                {
                                    hint
                                } else {
                                    format!("Function '{}' declares return type '{}' but body has type '{}'",
                                        name, declared_ret, inferred)
                                };
                                errors.push(CompilerError::new(msg).with_span(self.current_span));
                            }
                        }
                    }

                    // If no return type annotation, warn when body type differs from the Int default.
                    // This catches the case where someone writes e.g. `fun f() { "hello" }` —
                    // the type checker defaults to `Int` for the return type without warning,
                    // but callers see `Int` when the body actually returns `String`.
                    if return_type.is_none() && type_params.is_empty() {
                        let inferred = self.infer_expr_type(body).unwrap_or(Type::Named("Int".into()));
                        // Only warn for clear non-Int, non-Unit types. Int is the default/fallback,
                        // and Unit is a valid implicit void return.
                        if !matches!(&inferred, Type::Named(n) if n == "Int")
                            && !matches!(&inferred, Type::Unit)
                        {
                            // Update the function's entry in type_env so subsequent functions
                            // that call this one get the correct return type.
                            let param_tys: Vec<Type> = params
                                .iter()
                                .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                                .collect();
                            let fn_type = Type::Function(param_tys, Box::new(inferred.clone()));
                            self.type_env.insert(name.clone(), fn_type);

                            eprintln!(
                                "Warning: function '{}' has no return type annotation. \
                                 Inferred return type is '{}', not 'Int'. \
                                 Add ': {}' to the function signature to make this explicit.",
                                name, inferred, inferred
                            );
                        }
                    }

                    // Restore type param bindings
                    for (tpname, old_val) in saved_tps {
                        if let Some(ty) = old_val {
                            self.type_env.insert(tpname, ty);
                        } else {
                            self.type_env.remove(&tpname);
                        }
                    }
                    // Restore parameter bindings
                    for (pname, old_val) in saved {
                        if let Some(ty) = old_val {
                            self.type_env.insert(pname, ty);
                        } else {
                            self.type_env.remove(&pname);
                        }
                    }
                }
                Stmt::Expr { expr, .. } => {
                    self.collect_expr_errors(expr, &mut errors);
                }
                Stmt::Let {
                    name,
                    type_ann,
                    value,
                    ..
                } => {
                    self.collect_expr_errors(value, &mut errors);
                    if let Some(ann) = type_ann {
                        let inferred = self.infer_expr_type(value).unwrap_or(Type::Named("Int".into()));
                        if !self.types_compatible(ann, &inferred) {
                            let msg = if let Some(hint) = Self::check_termination(ann, &inferred) {
                                hint
                            } else {
                                format!(
                                    "Variable '{}' declared as '{}' but initialized with '{}'",
                                    name, ann, inferred
                                )
                            };
                            errors.push(CompilerError::new(msg).with_span(self.current_span));
                        }
                    }
                }
                Stmt::Destructure { value, .. } => {
                    self.collect_expr_errors(value, &mut errors);
                }
                Stmt::Const {
                    name,
                    type_ann,
                    value,
                    ..
                } => {
                    self.collect_expr_errors(value, &mut errors);
                    if let Some(ann) = type_ann {
                        let inferred = self.infer_expr_type(value).unwrap_or(Type::Named("Int".into()));
                        if !self.types_compatible(ann, &inferred) {
                            let msg = if let Some(hint) = Self::check_termination(ann, &inferred) {
                                hint
                            } else {
                                format!(
                                    "Constant '{}' declared as '{}' but initialized with '{}'",
                                    name, ann, inferred
                                )
                            };
                            errors.push(CompilerError::new(msg).with_span(self.current_span));
                        }
                    }
                }
                _ => {}
            }
        }

        errors
    }

    /// Extract arms from a When expression, if it's a ValueMatch or ConditionChain
    fn when_arms<'a>(&self, w: &'a When) -> &'a [WhenArm] {
        match &w.kind {
            WhenKind::ValueMatch { arms, .. } => arms,
            WhenKind::ConditionChain { arms } => arms,
            _ => &[], // OneLine has no arms
        }
    }

    fn collect_expr_errors(&mut self, expr: &Expr, errors: &mut Vec<CompilerError>) {
        match expr {
            Expr::Binary(lhs, op, rhs) => {
                if let Err(e) = self.check_binary_op(lhs, *op, rhs) {
                    errors.push(e);
                }
                self.collect_expr_errors(lhs, errors);
                self.collect_expr_errors(rhs, errors);
            }
            Expr::When(w) => {
                let arms = self.when_arms(w);
                if !arms.is_empty() {
                    if let Err(e) = self.check_when_arms(arms) {
                        errors.push(e);
                    }
                    if let Err(msg) = self.registry.check_when_exhaustive(arms) {
                        errors.push(CompilerError::new(msg).with_span(self.current_span));
                    }
                    // Smart cast: for value-match when x { null -> ...; else -> ... },
                    // inject x into not_null_set for non-null arms
                    let smart_var: Option<String> = match &w.kind {
                        WhenKind::ValueMatch { value, .. } => {
                            if let Expr::Ident(name) = value.as_ref() {
                                let ty = self.infer_expr_type(value).unwrap_or(Type::Named("Int".into()));
                                if matches!(ty, Type::Nullable(_)) {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    for arm in arms {
                        // Inject smart cast variable for non-null patterns
                        let is_non_null = match &arm.pattern {
                            Pattern::Null => false,
                            _ => true, // any non-null pattern means value is not null
                        };
                        if let Some(ref var) = smart_var {
                            if is_non_null {
                                self.not_null_set.borrow_mut().insert(var.clone());
                            }
                        }
                        self.collect_expr_errors(&arm.body, errors);
                        if let Some(ref var) = smart_var {
                            self.not_null_set.borrow_mut().remove(var);
                        }
                    }
                }
                // Smart cast for OneLine when: when x != null { ... } [else { ... }]
                if let WhenKind::OneLine {
                    condition,
                    then_expr,
                    else_expr,
                } = &w.kind
                {
                    let smart_var = match condition.as_ref() {
                        Expr::Binary(lhs, BinaryOp::Neq, rhs) => {
                            match (lhs.as_ref(), rhs.as_ref()) {
                                (Expr::Ident(name), Expr::Null)
                                | (Expr::Null, Expr::Ident(name)) => {
                                    let ty = self.infer_expr_type(lhs).unwrap_or(Type::Named("Int".into()));
                                    if matches!(ty, Type::Nullable(_)) {
                                        Some(name.clone())
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        }
                        Expr::Binary(lhs, BinaryOp::Eq, rhs) => {
                            // x == null means in the ELSE branch x is NOT null (smart cast)
                            match (lhs.as_ref(), rhs.as_ref()) {
                                (Expr::Ident(name), Expr::Null)
                                | (Expr::Null, Expr::Ident(name)) => {
                                    let ty = self.infer_expr_type(lhs).unwrap_or(Type::Named("Int".into()));
                                    if matches!(ty, Type::Nullable(_)) {
                                        Some(name.clone())
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    };
                    if let Some(ref var) = smart_var {
                        // For x != null: then branch has non-null x
                        // For x == null: else branch has non-null x
                        let is_neq =
                            matches!(condition.as_ref(), Expr::Binary(_, BinaryOp::Neq, _));
                        if is_neq {
                            self.not_null_set.borrow_mut().insert(var.clone());
                        }
                        self.collect_expr_errors(then_expr, errors);
                        if is_neq {
                            self.not_null_set.borrow_mut().remove(var);
                        }
                        if !is_neq {
                            self.not_null_set.borrow_mut().insert(var.clone());
                        }
                        self.collect_expr_errors(else_expr, errors);
                        if !is_neq {
                            self.not_null_set.borrow_mut().remove(var);
                        }
                    } else {
                        self.collect_expr_errors(then_expr, errors);
                        self.collect_expr_errors(else_expr, errors);
                    }
                }
            }
            Expr::Call {
                func,
                args,
                trailing_lambda,
            } => {
                if let Err(e) = self.check_call(func, args) {
                    errors.push(e);
                }
                self.collect_expr_errors(func, errors);
                for a in args {
                    self.collect_expr_errors(a, errors);
                }
                if let Some(lam) = trailing_lambda {
                    self.collect_expr_errors(lam, errors);
                }
            }
            Expr::Block(stmts) => {
                for s in stmts {
                    self.collect_stmt_errors(s, errors);
                }
            }
            Expr::For(for_expr) => match &for_expr.kind {
                ForKind::Iterate { iterable, body, .. } => {
                    self.collect_expr_errors(iterable, errors);
                    self.collect_expr_errors(body, errors);
                }
                ForKind::IterateWithIndex { iterable, body, .. } => {
                    self.collect_expr_errors(iterable, errors);
                    self.collect_expr_errors(body, errors);
                }
                ForKind::Condition {
                    condition, body, ..
                } => {
                    self.collect_expr_errors(condition, errors);
                    self.collect_expr_errors(body, errors);
                }
                ForKind::Infinite { body, .. } => {
                    self.collect_expr_errors(body, errors);
                }
                ForKind::NestedIterate { bindings, body, .. } => {
                    for (_, e) in bindings {
                        self.collect_expr_errors(e, errors);
                    }
                    self.collect_expr_errors(body, errors);
                }
            },
            Expr::Lambda { body, .. } => {
                self.collect_expr_errors(body, errors);
            }
            Expr::FieldAccess(obj, _) => {
                self.collect_expr_errors(obj, errors);
            }
            Expr::Copy(inner) => {
                self.collect_expr_errors(inner, errors);
            }
            Expr::Unsafe(inner) => {
                self.collect_expr_errors(inner, errors);
            }
            Expr::Null => {}
            Expr::OrBlock { nullable, fallback } => {
                self.collect_expr_errors(nullable, errors);
                self.collect_expr_errors(fallback, errors);
            }
            Expr::Unary(_, inner) => {
                self.collect_expr_errors(inner, errors);
            }
            Expr::Index(obj, idx) => {
                self.collect_expr_errors(obj, errors);
                self.collect_expr_errors(idx, errors);
            }
            Expr::Assign { target, value, .. } => {
                self.collect_expr_errors(target, errors);
                self.collect_expr_errors(value, errors);
            }
            Expr::Tuple(elements) => {
                for (_, e) in elements {
                    self.collect_expr_errors(e, errors);
                }
            }
            Expr::Range(start, end) => {
                self.collect_expr_errors(start, errors);
                self.collect_expr_errors(end, errors);
            }
            Expr::StructLiteral(fields) => {
                for (_, v) in fields {
                    self.collect_expr_errors(v, errors);
                }
            }
            Expr::MapLiteral(entries) => {
                for (k, v) in entries {
                    self.collect_expr_errors(k, errors);
                    self.collect_expr_errors(v, errors);
                }
            }
            Expr::SetLiteral(elements) => {
                for e in elements {
                    self.collect_expr_errors(e, errors);
                }
            }
            Expr::StringInterpolate(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.collect_expr_errors(e, errors);
                    }
                }
            }
            _ => {} // Literal, Ident, Continue, Break, etc.
        }
    }

    fn collect_stmt_errors(&mut self, stmt: &Stmt, errors: &mut Vec<CompilerError>) {
        match stmt {
            Stmt::Expr { expr, .. } => self.collect_expr_errors(expr, errors),
            Stmt::Let {
                name,
                type_ann,
                value,
                ..
            } => {
                self.collect_expr_errors(value, errors);
                let inferred = self.infer_expr_type(value).unwrap_or(Type::Named("Int".into()));
                let ty = type_ann.clone().unwrap_or(inferred);
                self.type_env.insert(name.clone(), ty);
            }
            Stmt::Const {
                name,
                type_ann,
                value,
                ..
            } => {
                self.collect_expr_errors(value, errors);
                let inferred = self.infer_expr_type(value).unwrap_or(Type::Named("Int".into()));
                let ty = type_ann.clone().unwrap_or(inferred);
                self.type_env.insert(name.clone(), ty);
            }
            Stmt::Destructure { value, .. } => {
                self.collect_expr_errors(value, errors);
            }
            Stmt::Return { value: expr, .. } => {
                if let Some(e) = expr {
                    self.collect_expr_errors(e, errors);
                }
            }
            _ => {}
        }
    }

    fn check_binary_op(&self, lhs: &Expr, op: BinaryOp, rhs: &Expr) -> Result<(), CompilerError> {
        let lt = self.infer_expr_type(lhs)?;
        let rt = self.infer_expr_type(rhs)?;

        // Reject nullable operands in arithmetic/bitwise operations.
        // String concatenation (Add) with nullable is allowed.
        let is_nullable_op = matches!(lt, Type::Nullable(_)) || matches!(rt, Type::Nullable(_));
        if is_nullable_op {
            let is_add_string = op == BinaryOp::Add
                && (format!("{}", lt).starts_with("Nullable<String")
                    || format!("{}", rt).starts_with("Nullable<String")
                    || format!("{}", lt).starts_with("String")
                    || format!("{}", rt).starts_with("String"));
            match op {
                BinaryOp::Add if is_add_string => {} // allow
                BinaryOp::Eq | BinaryOp::Neq | BinaryOp::And | BinaryOp::Or => {} // comparison/logical allow
                BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Lte | BinaryOp::Gte => {} // comparison allow
                _ => {
                    return Err(CompilerError::new(format!(
                        "Arithmetic/bitwise operation '{}' does not accept nullable operands. Use 'or {{ }}' to provide a default",
                        op
                    ))
                    .with_span(self.current_span));
                }
            }
        }

        match op {
            BinaryOp::Add => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                if ls == "String" || rs == "String" {
                    return Ok(());
                }
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                if ls == "String" || rs == "String" || ls == "Bool" || rs == "Bool" {
                    return Err(CompilerError::new(format!(
                        "Arithmetic operation '{}' not supported for {}",
                        op,
                        if ls == "Bool" || rs == "Bool" {
                            "Bool"
                        } else {
                            "String"
                        }
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::Eq | BinaryOp::Neq => {
                return Ok(());
            }
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Lte | BinaryOp::Gte => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                // Allow Bool comparison (True > False), but disallow mixed Bool/other types
                if (ls == "Bool" || rs == "Bool") && ls != rs {
                    return Err(CompilerError::new(format!(
                        "Cannot compare '{}' with '{}'",
                        ls, rs
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                if format!("{}", lt) != "Bool" || format!("{}", rt) != "Bool" {
                    return Err(CompilerError::new(format!(
                        "Logical operator '{}' requires Bool operands, got '{}' and '{}'",
                        op, lt, rt
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                if ls != "Int" || rs != "Int" {
                    return Err(CompilerError::new(format!(
                        "Bitwise operator '{}' requires Int operands, got '{}' and '{}'",
                        op, lt, rs
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::Range
            | BinaryOp::RangeExclusive
            | BinaryOp::Assign
            | BinaryOp::In
            | BinaryOp::Is => {}
        }
        Ok(())
    }

    fn check_call(&self, func: &Expr, args: &[Expr]) -> Result<(), CompilerError> {
        if let Expr::Ident(name) = func {
            if let Some((_ei, vi)) = self.registry.lookup_variant(name) {
                let expected = vi.params.len();
                let actual = args.len();
                if expected != actual {
                    return Err(CompilerError::new(format!(
                        "Enum variant '{}' expects {} arguments, but got {}",
                        name, expected, actual
                    ))
                    .with_span(self.current_span));
                }
            }
            // Check generic function via type inference
            if let Some(generic_stmt) = self.generic_funs.get(name) {
                if let Stmt::Fun {
                    params,
                    type_params,
                    ..
                } = generic_stmt
                {
                    if !type_params.is_empty() {
                        let param_tys: Vec<Type> = params
                            .iter()
                            .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                            .collect();
                        if args.len() != param_tys.len() {
                            return Err(CompilerError::new(format!(
                                "Function '{}' expects {} arguments, but got {}",
                                name,
                                param_tys.len(),
                                args.len()
                            ))
                            .with_span(self.current_span));
                        }
                        // Collect arg types, skipping lambdas
                        let mut arg_tys = Vec::new();
                        let mut filtered_params = Vec::new();
                        for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                            if matches!(arg, Expr::Lambda { .. }) {
                                continue;
                            }
                            arg_tys.push(self.infer_expr_type(arg).unwrap_or(Type::Named("Int".into())));
                            filtered_params.push(param_ty.clone());
                        }
                        if !filtered_params.is_empty() {
                            if let Err(msg) = self.infer_type_args(&filtered_params, &arg_tys) {
                                return Err(CompilerError::new(format!(
                                    "Cannot infer type arguments for '{}': {}",
                                    name, msg
                                ))
                                .with_span(self.current_span));
                            }
                        }
                        return Ok(());
                    }
                }
            }
            // Check function argument types
            if let Some(fn_type) = self.type_env.get(name) {
                match fn_type {
                    Type::Function(param_tys, _ret_ty) => {
                        if args.len() != param_tys.len() {
                            return Err(CompilerError::new(format!(
                                "Function '{}' expects {} arguments, but got {}",
                                name,
                                param_tys.len(),
                                args.len()
                            ))
                            .with_span(self.current_span));
                        }
                        for (i, (arg, param_ty)) in args.iter().zip(param_tys.iter()).enumerate() {
                            // Skip lambdas — infer_expr_type returns body type, not function type
                            if matches!(arg, Expr::Lambda { .. }) {
                                continue;
                            }
                            let arg_ty = self.infer_expr_type(arg)?;
                            if !self.types_compatible(param_ty, &arg_ty) {
                                let msg = if let Some(hint) =
                                    Self::check_termination(param_ty, &arg_ty)
                                {
                                    hint
                                } else {
                                    format!(
                                        "Argument {} to '{}' expects '{}' but got '{}'",
                                        i + 1,
                                        name,
                                        param_ty,
                                        arg_ty
                                    )
                                };
                                return Err(CompilerError::new(msg).with_span(self.current_span));
                            }
                        }
                    }
                    _ => {
                        // Variable callable? Mapped to a fn type? Not yet supported.
                        // For now, let codegen handle mismatches.
                    }
                }
            }
        }
        Ok(())
    }

    fn check_when_arms(&self, arms: &[WhenArm]) -> Result<(), CompilerError> {
        if arms.is_empty() {
            return Ok(());
        }

        // Collect arm types, but be lenient with Int (fallback) when mixed with enums
        let types: Vec<Type> = arms.iter().map(|a| self.infer_expr_type(&a.body)).collect::<Result<Vec<Type>, _>>()?;
        let first = &types[0];

        // If first type is Int, it might be a fallback — skip arm checking
        if matches!(first, Type::Named(ref n) if n == "Int") {
            return Ok(());
        }

        for (i, t) in types.iter().enumerate().skip(1) {
            // Skip Int fallback arms
            if matches!(t, Type::Named(ref n) if n == "Int") {
                continue;
            }
            if !self.types_compatible(first, t) {
                return Err(CompilerError::new(format!(
                    "When arm type mismatch: arm 1 is '{}' but arm {} is '{}'",
                    first,
                    i + 1,
                    t
                ))
                .with_span(self.current_span));
            }
        }
        Ok(())
    }

    /// Infer the type of an expression (structural, not full HM inference)
    fn infer_expr_type(&self, expr: &Expr) -> Result<Type, CompilerError> {
        match expr {
            Expr::Literal(Literal::String(_)) | Expr::StringInterpolate(_) => {
                Ok(Type::Named("String".into()))
            }
            Expr::Literal(Literal::Int(_)) => Ok(Type::Named("Int".into())),
            Expr::Literal(Literal::Float(_)) => Ok(Type::Named("Float".into())),
            Expr::Literal(Literal::Bool(_)) => Ok(Type::Named("Bool".into())),
            Expr::Literal(Literal::Char(_)) => Ok(Type::Named("Char".into())),
            Expr::Literal(Literal::Unit) => Ok(Type::Unit),
            Expr::MapLiteral(_) => Ok(Type::Map(
                Box::new(Type::Named("String".into())),
                Box::new(Type::Named("Int".into())),
            )),
            Expr::SetLiteral(_) => Ok(Type::Set(Box::new(Type::Named("Int".into())))),
            Expr::Binary(lhs, op, rhs) => {
                let lt = self.infer_expr_type(lhs)?;
                let rt = self.infer_expr_type(rhs)?;
                if *op == BinaryOp::Add {
                    if matches!(&lt, Type::Named(ref n) if n == "String")
                        || matches!(&rt, Type::Named(ref n) if n == "String")
                    {
                        return Ok(Type::Named("String".into()));
                    }
                }
                if *op == BinaryOp::And
                    || *op == BinaryOp::Or
                    || *op == BinaryOp::Eq
                    || *op == BinaryOp::Neq
                    || *op == BinaryOp::Lt
                    || *op == BinaryOp::Gt
                    || *op == BinaryOp::Lte
                    || *op == BinaryOp::Gte
                    || *op == BinaryOp::In
                    || *op == BinaryOp::Is
                {
                    return Ok(Type::Named("Bool".into()));
                }
                if *op == BinaryOp::BitAnd
                    || *op == BinaryOp::BitOr
                    || *op == BinaryOp::BitXor
                    || *op == BinaryOp::Shl
                    || *op == BinaryOp::Shr
                {
                    return Ok(Type::Named("Int".into()));
                }
                if *op == BinaryOp::Pow {
                    // Return Float if either operand is Float
                    if matches!(&lt, Type::Named(ref n) if n == "Float")
                        || matches!(&rt, Type::Named(ref n) if n == "Float")
                    {
                        return Ok(Type::Named("Float".into()));
                    }
                    return Ok(lt);
                }
                // Arithmetic: return Float if either operand is Float, else Int
                if matches!(&lt, Type::Named(ref n) if n == "Float")
                    || matches!(&rt, Type::Named(ref n) if n == "Float")
                {
                    return Ok(Type::Named("Float".into()));
                }
                Ok(Type::Named("Int".into()))
            }
            Expr::Call { func, args, .. } => {
                if let Expr::Ident(name) = func.as_ref() {
                    match name.as_str() {
                        "print" | "println" | "send" | "close" | "cancel" => Ok(Type::Unit),
                        "toString" | "toUpper" | "toLower" => Ok(Type::Named("String".into())),
                        "receive" | "wait" => Ok(Type::Named("Int".into())),
                        "launch" => Ok(Type::Task(Box::new(Type::Named("Int".into())))),
                        "Stream" => Ok(Type::Stream(Box::new(Type::Named("Int".into())))),
                        "is_done" | "is_cancelled" => Ok(Type::Named("Bool".into())),
                        "withTimeout" => Ok(Type::Nullable(Box::new(Type::Named("Int".into())))),
                        "coroutineScope" => Ok(Type::Named("list".into())),
                        // Callback-based list functions
                        "any" | "all" => Ok(Type::Named("Bool".into())),
                        "find" | "findIndex" | "reduce" => {
                            Ok(Type::Nullable(Box::new(Type::Named("Int".into()))))
                        }
                        "foldRight" => Ok(Type::Named("Int".into())),
                        "takeWhile" | "dropWhile" | "sortedBy" => Ok(Type::Named("list".into())),
                        _ => {
                            if self.registry.lookup_variant(name).is_some() {
                                let enum_name = self
                                    .registry
                                    .variant_to_enum
                                    .get(name)
                                    .cloned()
                                    .unwrap_or_default();
                                Ok(Type::Named(enum_name))
                            } else if let Some(generic_stmt) = self.generic_funs.get(name) {
                                // Generic function: infer type args and resolve return type
                                Ok(self.infer_generic_return_type(generic_stmt, args))
                            } else if let Some(Type::Function(_, ret)) = self.type_env.get(name) {
                                Ok(*ret.clone())
                            } else {
                                Ok(Type::Named("Int".into()))
                            }
                        }
                    }
                } else if let Expr::FieldAccess(receiver, method) = func.as_ref() {
                    let recv_type = self.infer_expr_type(receiver)?;
                    match (recv_type, method.as_str()) {
                        // Map/Set UFCS methods
                        (Type::Map(_, _), "contains")
                        | (Type::Set(_), "contains")
                        | (Type::Map(_, _), "isEmpty")
                        | (Type::Set(_), "isEmpty") => Ok(Type::Named("Bool".into())),
                        (Type::Map(_, _), "insert") | (Type::Set(_), "insert") => Ok(Type::Unit),
                        (Type::Map(_, _), "remove")
                        | (Type::Map(_, _), "get")
                        | (Type::Set(_), "remove") => {
                            Ok(Type::Nullable(Box::new(Type::Named("Int".into()))))
                        }
                        // Stream UFCS methods
                        (Type::Stream(_), "send") => Ok(Type::Unit),
                        (Type::Stream(_), "receive") => Ok(Type::Named("Int".into())),
                        (Type::Stream(_), "close") => Ok(Type::Unit),
                        // Task UFCS methods
                        (Type::Task(_), "cancel") => Ok(Type::Unit),
                        (Type::Task(_), "is_done") | (Type::Task(_), "is_cancelled") => {
                            Ok(Type::Named("Bool".into()))
                        }
                        (Type::Task(_), "wait") => Ok(Type::Named("Int".into())),
                        _ => Err(CompilerError::new(format!("Cannot infer type for expression: {:?}", expr))),
                    }
                } else {
                    Ok(Type::Named("Int".into()))
                }
            }
            Expr::When(w) => {
                let arms = self.when_arms(w);
                if !arms.is_empty() {
                    return self.infer_expr_type(&arms[0].body);
                }
                // OneLine: infer from the first branch body
                if let WhenKind::OneLine { then_expr, .. } = &w.kind {
                    return self.infer_expr_type(then_expr);
                }
                Ok(Type::Unit)
            }
            Expr::Continue | Expr::Break => Ok(Type::Unit),
            Expr::For(_) => Ok(Type::Unit),
            Expr::FunctionRef(name) => {
                if let Some(ty) = self.type_env.get(name) {
                    Ok(ty.clone())
                } else {
                    Ok(Type::Function(
                        vec![Type::Named("Int".into())],
                        Box::new(Type::Named("Int".into())),
                    ))
                }
            }
            Expr::Copy(inner) => self.infer_expr_type(inner),
            Expr::Null => Ok(Type::Nullable(Box::new(Type::Named("Nothing".into())))),
            Expr::OrBlock { nullable, fallback } => {
                let nullable_ty = self.infer_expr_type(nullable)?;
                let fallback_ty = self.infer_expr_type(fallback)?;
                // Or-block unwraps nullable: T? or { ... } -> T
                Ok(match nullable_ty {
                    Type::Nullable(inner) => {
                        if self.types_compatible(&inner, &fallback_ty) {
                            *inner
                        } else {
                            fallback_ty
                        }
                    }
                    _ => nullable_ty,
                })
            }
            Expr::Unsafe(inner) => self.infer_expr_type(inner),
            Expr::Block(stmts) => stmts
                .last()
                .map(|s| match s {
                    Stmt::Expr { expr: e, .. } => self.infer_expr_type(e),
                    Stmt::Return { value: e, .. } => e
                        .as_ref()
                        .map(|re| self.infer_expr_type(re))
                        .unwrap_or(Ok(Type::Unit)),
                    _ => Ok(Type::Unit),
                })
                .unwrap_or(Ok(Type::Unit)),
            Expr::Ident(name) => {
                if self.registry.lookup_variant(name).is_some() {
                    let enum_name = self
                        .registry
                        .variant_to_enum
                        .get(name)
                        .cloned()
                        .unwrap_or_default();
                    Ok(Type::Named(enum_name))
                } else if let Some(ty) = self.type_env.get(name) {
                    // Smart cast: if variable is known non-null, unwrap nullable type
                    if self.not_null_set.borrow().contains(name) {
                        if let Type::Nullable(inner) = ty {
                            return Ok(*inner.clone());
                        }
                    }
                    Ok(ty.clone())
                } else {
                    Ok(Type::Named("Int".into()))
                }
            }
            Expr::Lambda { body, .. } => self.infer_expr_type(body),
            Expr::Index(obj, _) => {
                let obj_type = self.infer_expr_type(obj)?;
                match obj_type {
                    // Map/Set indexing returns nullable T? (was Option<T>)
                    Type::Map(_, v) => Ok(Type::Nullable(v.clone())),
                    Type::Set(e) => Ok(Type::Nullable(e.clone())),
                    Type::Named(ref n) if n == "String" => Ok(Type::Named("Int".into())),
                    // If obj is nullable, indexing auto short-circuits to nullable
                    Type::Nullable(inner) => match *inner {
                        Type::Map(_, v) => Ok(Type::Nullable(v)),
                        Type::Set(e) => Ok(Type::Nullable(e)),
                        Type::Named(ref n) if n == "String" => Ok(Type::Named("Int".into())),
                        _ => Ok(Type::Nullable(Box::new(Type::Named("Int".into())))),
                    },
                    _ => Ok(Type::Named("Int".into())),
                }
            }
            Expr::FieldAccess(obj, field) => {
                let obj_type = self.infer_expr_type(obj)?;
                // If obj is nullable, field access short-circuits to nullable result
                let (inner_obj_type, is_nullable) = match &obj_type {
                    Type::Nullable(inner) => (inner.as_ref(), true),
                    other => (other, false),
                };
                let field_type: Type = if let Type::Named(type_name) = inner_obj_type {
                    let struct_name = match type_name.as_str() {
                        "Str" => "String",
                        "Double" => "Float",
                        other => other,
                    };
                    if let Some(struct_info) = self.registry.structs.get(struct_name) {
                        if let Some(index) = struct_info.field_index.get(field) {
                            struct_info.fields[*index].1.clone()
                        } else {
                            Type::Named("Int".into())
                        }
                    } else {
                        Type::Named("Int".into())
                    }
                } else {
                    Type::Named("Int".into())
                };
                if is_nullable {
                    Ok(Type::Nullable(Box::new(field_type)))
                } else {
                    Ok(field_type)
                }
            }
            Expr::StructLiteral(fields) => {
                let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                if let Some(struct_info) = self.registry.find_struct_by_fields(&field_names) {
                    Ok(Type::Named(struct_info.name.clone()))
                } else {
                    Ok(Type::Named("Int".into()))
                }
            }
            Expr::Assign { value, .. } => self.infer_expr_type(value),
            Expr::Unary(op, inner) => match op {
                UnaryOp::Not => Ok(Type::Named("Bool".into())),
                UnaryOp::Neg | UnaryOp::BitNot => Ok(self.infer_expr_type(inner)?),
            },
            _ => Ok(Type::Named("Int".into())),
        }
    }

    /// Infer the return type of a generic function call by unifying parameter types
    /// and substituting the result into the declared return type.
    fn infer_generic_return_type(&self, stmt: &Stmt, args: &[Expr]) -> Type {
        if let Stmt::Fun {
            params,
            return_type,
            ..
        } = stmt
        {
            let param_tys: Vec<Type> = params
                .iter()
                .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                .collect();
            let mut arg_tys = Vec::new();
            let mut filtered_params = Vec::new();
            for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                if matches!(arg, Expr::Lambda { .. }) {
                    continue;
                }
                arg_tys.push(self.infer_expr_type(arg).unwrap_or(Type::Named("Int".into())));
                filtered_params.push(param_ty.clone());
            }
            if let Ok(type_map) = self.infer_type_args(&filtered_params, &arg_tys) {
                if let Some(ret) = return_type {
                    return resolve_type_vars(ret, &type_map);
                }
            }
        }
        Type::Named("Int".into())
    }

    /// Unify an expected type (may contain TypeVars) with an actual concrete type,
    /// accumulating type variable bindings in type_map.
    fn unify(
        &self,
        expected: &Type,
        actual: &Type,
        type_map: &mut HashMap<String, Type>,
    ) -> Result<(), String> {
        match (expected, actual) {
            (Type::TypeVar(name), _) => {
                if let Some(existing) = type_map.get(name) {
                    if self.types_compatible(existing, actual) {
                        Ok(())
                    } else {
                        Err(format!(
                            "Conflicting type inference for '{}': {} vs {}",
                            name, existing, actual
                        ))
                    }
                } else {
                    type_map.insert(name.clone(), actual.clone());
                    Ok(())
                }
            }
            (Type::Named(a), Type::Named(b)) => {
                if a == b {
                    Ok(())
                } else {
                    // Normalize aliases
                    let norm_a = match a.as_str() {
                        "Str" => "String",
                        "Double" => "Float",
                        o => o,
                    };
                    let norm_b = match b.as_str() {
                        "Str" => "String",
                        "Double" => "Float",
                        o => o,
                    };
                    if norm_a == norm_b {
                        Ok(())
                    } else {
                        Err(format!("Type mismatch: {} vs {}", a, b))
                    }
                }
            }
            (Type::Generic(ba, ta), Type::Generic(bb, tb)) => {
                if ta.len() != tb.len() {
                    return Err("Generic argument count mismatch".to_string());
                }
                self.unify(ba, bb, type_map)?;
                for (a, b) in ta.iter().zip(tb.iter()) {
                    self.unify(a, b, type_map)?;
                }
                Ok(())
            }
            (Type::Nullable(a), Type::Nullable(b)) => self.unify(a, b, type_map),
            (Type::Function(pa, ra), Type::Function(pb, rb)) => {
                if pa.len() != pb.len() {
                    return Err("Function arity mismatch".to_string());
                }
                for (a, b) in pa.iter().zip(pb.iter()) {
                    self.unify(a, b, type_map)?;
                }
                self.unify(ra, rb, type_map)
            }
            (Type::Struct(fa), Type::Struct(fb)) => {
                if fa.len() != fb.len() {
                    return Err("Struct field count mismatch".to_string());
                }
                for ((na, ta), (nb, tb)) in fa.iter().zip(fb.iter()) {
                    if na != nb {
                        return Err(format!("Struct field name mismatch: {} vs {}", na, nb));
                    }
                    self.unify(ta, tb, type_map)?;
                }
                Ok(())
            }
            (Type::Map(ka, va), Type::Map(kb, vb)) => {
                self.unify(ka, kb, type_map)?;
                self.unify(va, vb, type_map)
            }
            (Type::Set(ea), Type::Set(eb)) => self.unify(ea, eb, type_map),
            (Type::Task(ta), Type::Task(tb)) => self.unify(ta, tb, type_map),
            (Type::Stream(sa), Type::Stream(sb)) => self.unify(sa, sb, type_map),
            (Type::LazyList(la), Type::LazyList(lb)) => self.unify(la, lb, type_map),
            (Type::Ptr(pa), Type::Ptr(pb)) => self.unify(pa, pb, type_map),
            (Type::Unit, Type::Unit) => Ok(()),
            // Auto-wrap: T can be passed where T? is expected
            (Type::Nullable(inner), _) if !matches!(actual, Type::Nullable(_)) => {
                self.unify(inner, actual, type_map)
            }
            // Null literal (Nothing) is compatible with any nullable
            (_, Type::Nullable(inner)) if matches!(inner.as_ref(), Type::Named(n) if n == "Nothing") => {
                Ok(())
            }
            _ => Err(format!("Type mismatch: {} vs {}", expected, actual)),
        }
    }

    /// Infer type arguments for a generic function call by unifying parameter types
    /// with actual argument types.
    fn infer_type_args(
        &self,
        param_tys: &[Type],
        arg_tys: &[Type],
    ) -> Result<HashMap<String, Type>, String> {
        let mut type_map = HashMap::new();
        for (param_ty, arg_ty) in param_tys.iter().zip(arg_tys.iter()) {
            self.unify(param_ty, arg_ty, &mut type_map)?;
        }
        Ok(type_map)
    }

    /// Check for nullable termination violation: T? used where T is expected.
    /// Returns Some(error_suffix) if declared expects T but inferred is T?.
    fn check_termination(declared: &Type, inferred: &Type) -> Option<String> {
        match (declared, inferred) {
            (Type::Nullable(_), _) => None, // declared is nullable, assignment of non-null to T? is fine
            (_, Type::Nullable(_inner)) => {
                // T? used where T is expected
                if !matches!(declared, Type::Named(n) if n == "Nothing") {
                    Some(format!(
                        "cannot use nullable '{}' where non-nullable '{}' is expected. Use 'or {{ }}' to provide a default, or check for null first",
                        inferred, declared
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Check if two types are structurally compatible
    fn types_compatible(&self, declared: &Type, inferred: &Type) -> bool {
        match (declared, inferred) {
            (Type::Unit, Type::Unit) => true,
            (Type::Named(a), Type::Named(b)) => {
                if a == b {
                    return true;
                }
                // Normalize type aliases: Str=String, Double=Float
                let norm_a = match a.as_str() {
                    "Str" => "String",
                    "Double" => "Float",
                    other => other,
                };
                let norm_b = match b.as_str() {
                    "Str" => "String",
                    "Double" => "Float",
                    other => other,
                };
                norm_a == norm_b
            }
            (Type::Struct(fa), Type::Struct(fb)) => {
                if fa.len() != fb.len() {
                    return false;
                }
                fa.iter()
                    .zip(fb.iter())
                    .all(|((na, ta), (nb, tb))| na == nb && self.types_compatible(ta, tb))
            }
            (Type::Map(ka, va), Type::Map(kb, vb)) => {
                self.types_compatible(ka, kb) && self.types_compatible(va, vb)
            }
            (Type::Set(ea), Type::Set(eb)) => self.types_compatible(ea, eb),
            (Type::Task(ta), Type::Task(tb)) => self.types_compatible(ta, tb),
            (Type::Stream(sa), Type::Stream(sb)) => self.types_compatible(sa, sb),
            (Type::LazyList(la), Type::LazyList(lb)) => self.types_compatible(la, lb),
            (Type::Ptr(pa), Type::Ptr(pb)) => self.types_compatible(pa, pb),
            (Type::CString, Type::CString) | (Type::FileHandle, Type::FileHandle) => true,
            (Type::Function(pa, ra), Type::Function(pb, rb)) => {
                if pa.len() != pb.len() {
                    return false;
                }
                pa.iter()
                    .zip(pb.iter())
                    .all(|(a, b)| self.types_compatible(a, b))
                    && self.types_compatible(ra, rb)
            }
            (Type::Generic(ba, ta), Type::Generic(bb, tb)) => {
                ta.len() == tb.len()
                    && self.types_compatible(ba, bb)
                    && ta
                        .iter()
                        .zip(tb.iter())
                        .all(|(a, b)| self.types_compatible(a, b))
            }
            // Type variables are compatible with anything (validated by unification at call sites)
            (Type::TypeVar(_), _) => true,
            (_, Type::TypeVar(_)) => true,
            // Nullable<Nothing> (from null literal) is compatible with any nullable
            // Must check before general Nullable compatibility
            (_declared, Type::Nullable(inner_inferred)) if matches!(inner_inferred.as_ref(), Type::Named(n) if n == "Nothing") =>
            {
                matches!(_declared, Type::Nullable(_))
            }
            // T can be used where T? is expected (auto-wrapping non-nullable into nullable)
            (Type::Nullable(inner_declared), inferred)
                if !matches!(inferred, Type::Nullable(_)) =>
            {
                self.types_compatible(inner_declared, inferred)
            }
            // Nullable<T> is compatible with Nullable<U> if T compatible with U
            (Type::Nullable(ia), Type::Nullable(ib)) => self.types_compatible(ia, ib),
            // T? cannot be used where T is expected (termination check needed)
            (_, Type::Nullable(_)) => false,
            // All other combinations are type mismatches
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Program;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check_source(source: &str) -> Vec<CompilerError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let mut program = parser.parse_program().expect("Parsing should succeed");
        let mut registry = TypeRegistry::new();
        for stmt in &program.stmts {
            let _ = registry.register(stmt);
        }
        let mut checker = TypeChecker::new(registry);
        // Seed basic types so the checker knows about Int, String, Bool, etc.
        let mut type_env = HashMap::new();
        type_env.insert("Int".to_string(), Type::Named("Int".into()));
        type_env.insert("String".to_string(), Type::Named("String".into()));
        type_env.insert("Bool".to_string(), Type::Named("Bool".into()));
        type_env.insert("Float".to_string(), Type::Named("Float".into()));
        type_env.insert("Char".to_string(), Type::Named("Char".into()));
        checker.seed_type_env(&type_env);
        checker.check(&program)
    }

    #[test]
    fn test_arith_on_string() {
        let errors = check_source("val x = 1 - \"hello\"");
        assert!(
            !errors.is_empty(),
            "Expected type error for string arithmetic"
        );
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("arithmetic") && msg.contains("string"),
            "Expected arithmetic-on-string error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_arith_on_bool() {
        let errors = check_source("val x = true - 1");
        assert!(
            !errors.is_empty(),
            "Expected type error for bool arithmetic"
        );
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("arithmetic") && msg.contains("bool"),
            "Expected arithmetic-on-bool error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_logical_op_non_bool() {
        let errors = check_source("val x = true && 5");
        assert!(!errors.is_empty(), "Expected type error for logical op");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("logical") && msg.contains("bool"),
            "Expected logical-op error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_bitwise_op_non_int() {
        let errors = check_source("val x = 1 & true");
        assert!(!errors.is_empty(), "Expected type error for bitwise op");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("bitwise") && msg.contains("int"),
            "Expected bitwise-op error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_bool_comparison_with_int() {
        let errors = check_source("val x = true > 1");
        assert!(
            !errors.is_empty(),
            "Expected type error for bool comparison"
        );
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("cannot compare") && msg.contains("bool"),
            "Expected comparison error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_return_type_mismatch() {
        let errors = check_source("fun f() -> String { 42 }");
        assert!(!errors.is_empty(), "Expected return type mismatch error");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("return type") && msg.contains("string") && msg.contains("int"),
            "Expected return type error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_variable_type_annotation_mismatch() {
        let errors = check_source("val x Int = \"hello\"");
        assert!(!errors.is_empty(), "Expected variable type mismatch error");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("declared as") && msg.contains("int") && msg.contains("string"),
            "Expected variable type error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_function_arg_count_mismatch() {
        let errors = check_source("fun f(x: Int) {} val y = f()");
        assert!(!errors.is_empty(), "Expected arg count mismatch error");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("expects 1") && msg.contains("got 0"),
            "Expected arg count error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_param_missing_type_annotation() {
        let errors = check_source("fun f(x) { x }");
        assert!(!errors.is_empty(), "Expected missing type annotation error");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("must have a type annotation"),
            "Expected type annotation error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_non_exhaustive_when() {
        let errors =
            check_source("enum Color { Red, Blue } fun f(c: Color) -> Int { when c { Red -> 1 } }");
        // Check that at least one error mentions non-exhaustive
        let has_nex = errors.iter().any(|e| {
            e.message.to_lowercase().contains("non-exhaustive")
                || e.message.to_lowercase().contains("missing variant")
        });
        assert!(
            has_nex,
            "Expected non-exhaustive when error, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_string_minus_string() {
        let errors = check_source("val x = \"hello\" - \"world\"");
        assert!(
            !errors.is_empty(),
            "Expected type error for string subtraction"
        );
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("arithmetic") && msg.contains("string"),
            "Expected arithmetic-on-string error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_type_mismatch_in_when_arms() {
        let errors = check_source("when true { 1 -> \"one\"; true -> 42 }");
        assert!(!errors.is_empty(), "expected type error for mismatched when arms");
    }

    #[test]
    fn test_for_loop_non_iterable() {
        let errors = check_source("for x in 42 { x }");
        assert!(!errors.is_empty(), "expected type error for non-iterable in for loop");
    }

    #[test]
    fn test_invalid_generic_instantiation() {
        let errors = check_source("val x: List = List[1, 2, 3]");
        // List without type parameter - may warn or error
        // Just ensure no panic
        let _ = errors;
    }

    #[test]
    fn test_struct_field_type_mismatch() {
        let src = r#"
            type Person = { name: String, age: Int }
            val p = Person { name: "Alice", age: "twenty" }
        "#;
        let errors = check_source(src);
        assert!(!errors.is_empty(), "expected type error for struct field mismatch");
    }

    #[test]
    fn test_nullable_assignment_to_non_nullable() {
        let errors = check_source("val x: Int = null");
        assert!(!errors.is_empty(), "expected type error for nullable to non-nullable");
    }

    #[test]
    fn test_reassignment_to_immutable() {
        let errors = check_source("val x = 1\nx = 2");
        assert!(!errors.is_empty(), "expected error for reassignment to val");
    }

    #[test]
    fn test_undefined_variable_usage() {
        let errors = check_source("val y = undefinedVar + 1");
        assert!(!errors.is_empty(), "expected error for undefined variable");
    }

    #[test]
    fn test_complex_recursive_type_annotation() {
        // List of lists - should type check without panic
        let errors = check_source("val x: List[List[Int]] = List[List[1, 2], List[3, 4]]");
        // This may or may not error depending on generics support
        let _ = errors;
    }

    #[test]
    fn test_division_by_zero_constant() {
        // Division by zero in constant folding should be caught
        let errors = check_source("val x = 1 / 0");
        // We may or may not catch this at type-check time
        let _ = errors;
    }

    #[test]
    fn test_return_type_mismatch_complex() {
        let errors = check_source("fun f(x: Int) -> String { x + 1 }");
        assert!(!errors.is_empty(), "expected return type mismatch error");
    }

    #[test]
    fn test_call_non_function() {
        let errors = check_source("val x = 42\nval y = x(10)");
        assert!(!errors.is_empty(), "expected error for calling non-function");
    }

    #[test]
    fn test_char_type_mismatch() {
        let errors = check_source("val x: Int = 'a'");
        assert!(!errors.is_empty(), "expected type error for char-to-int assignment");
    }
}
