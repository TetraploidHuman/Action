//! Type checking: HM inference engine (`inference`) + expression inference (`expr_infer`).

mod check_stmt;
mod expr_infer;
mod fallibility;
mod inference;

pub use fallibility::FallibilityContext;

pub use crate::type_registry::{EnumInfo, EnumVariantInfo, StructInfo, TypeRegistry};

use crate::ast::*;
use crate::builtin;
use crate::error::CompilerError;
use crate::types::{mangle_name, types_compatible};
use action_span::Span;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// Type checker: walks the AST and verifies type consistency.
pub struct TypeChecker {
    pub(crate) registry: TypeRegistry,
    pub(crate) type_env: HashMap<String, Type>,
    pub(crate) current_span: Span,
    pub(crate) not_null_set: RefCell<HashSet<String>>,
    pub(crate) generic_funs: HashMap<String, Stmt>,
    pub(crate) mutable_vars: HashSet<String>,
    pub fallibility: FallibilityContext,
}

impl TypeChecker {
    pub fn new(registry: TypeRegistry) -> Self {
        TypeChecker {
            registry,
            type_env: HashMap::new(),
            current_span: Span::default(),
            not_null_set: RefCell::new(HashSet::new()),
            generic_funs: HashMap::new(),
            mutable_vars: HashSet::new(),
            fallibility: FallibilityContext::new(),
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

    /// Inferred type of an expression (for HIR lowering after `check`).
    pub fn inferred_type(&self, expr: &Expr) -> Type {
        self.infer_expr_type(expr)
            .unwrap_or(Type::Named("Int".into()))
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
                        let mangled = mangle_name(
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
                    let inferred = self.try_infer_expr_type(value);
                    let ty = type_ann.clone().unwrap_or(inferred);
                    self.type_env.insert(name.clone(), ty);
                }
                Stmt::Destructure {
                    names,
                    renames,
                    is_list,
                    is_struct,
                    value,
                    rest,
                    ..
                } => {
                    let value_ty = self.try_infer_expr_type(value);
                    let field_types: Vec<Type> = if *is_struct {
                        // Struct destructuring: val {x, y} = point
                        // names are local names, renames maps (field_name, local_name)
                        if let Type::Named(ref struct_name) = value_ty {
                            if let Some(info) = self.registry.get_struct(struct_name) {
                                names
                                    .iter()
                                    .map(|name| {
                                        let field_name = renames
                                            .iter()
                                            .find(|(_, local)| local == name)
                                            .map(|(f, _)| f)
                                            .unwrap_or(name);
                                        info.fields
                                            .iter()
                                            .find(|(fname, _)| fname == field_name)
                                            .map(|(_, ty)| ty.clone())
                                            .unwrap_or(Type::Named("Int".into()))
                                    })
                                    .collect()
                            } else {
                                vec![Type::Named("Int".into()); names.len()]
                            }
                        } else {
                            vec![Type::Named("Int".into()); names.len()]
                        }
                    } else if *is_list {
                        // List destructuring: val [a, b, ...rest] = list
                        let elem_ty = match &value_ty {
                            Type::Set(elem) | Type::LazyList(elem) => *elem.clone(),
                            Type::Generic(_, args) if args.len() == 1 => args[0].clone(),
                            _ => Type::Named("Int".into()),
                        };
                        let mut tys = vec![elem_ty; names.len()];
                        if let Some(rest_name) = rest {
                            // rest variable gets the list type
                            tys.push(value_ty.clone());
                            self.type_env.insert(rest_name.clone(), value_ty.clone());
                        }
                        tys
                    } else {
                        // Tuple destructuring: val (x, y) = expr
                        // Try struct field lookup by position, then by field name matching
                        if let Type::Named(ref struct_name) = value_ty {
                            if let Some(info) = self.registry.get_struct(struct_name) {
                                names
                                    .iter()
                                    .enumerate()
                                    .map(|(i, _)| {
                                        info.fields
                                            .get(i)
                                            .map(|(_, ty)| ty.clone())
                                            .unwrap_or(Type::Named("Int".into()))
                                    })
                                    .collect()
                            } else {
                                vec![Type::Named("Int".into()); names.len()]
                            }
                        } else if let Some(info) = self.registry.find_struct_by_fields(names) {
                            names
                                .iter()
                                .map(|name| {
                                    info.fields
                                        .iter()
                                        .find(|(fname, _)| fname == name)
                                        .map(|(_, ty)| ty.clone())
                                        .unwrap_or(Type::Named("Int".into()))
                                })
                                .collect()
                        } else {
                            vec![Type::Named("Int".into()); names.len()]
                        }
                    };
                    for (name, ty) in names.iter().zip(field_types) {
                        self.type_env.insert(name.clone(), ty);
                    }
                }
                Stmt::Const {
                    name,
                    type_ann,
                    value,
                    ..
                } => {
                    let inferred = self.try_infer_expr_type(value);
                    let ty = type_ann.clone().unwrap_or(inferred);
                    self.type_env.insert(name.clone(), ty);
                }
                Stmt::External {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let param_tys: Vec<Type> = params
                        .iter()
                        .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                        .collect();
                    let ret_ty = return_type.clone().unwrap_or(Type::Unit);
                    let fn_type = Type::Function(param_tys, Box::new(ret_ty));
                    self.type_env.insert(name.clone(), fn_type);
                }
                Stmt::Extension {
                    type_name, methods, ..
                } => {
                    for method in methods {
                        if let Stmt::Fun {
                            name,
                            params,
                            return_type,
                            ..
                        } = method
                        {
                            let param_tys: Vec<Type> = params
                                .iter()
                                .filter(|p| p.name != "self")
                                .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                                .collect();
                            let ret_ty = return_type.clone().unwrap_or(Type::Named("Int".into()));
                            let fn_type = Type::Function(param_tys, Box::new(ret_ty));
                            let lookup_key = format!("{}.{}", type_name, name);
                            self.type_env.insert(lookup_key, fn_type);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Lenient inference for environment building (tolerates forward references).
    fn try_infer_expr_type(&self, expr: &Expr) -> Type {
        self.infer_expr_type(expr)
            .unwrap_or(Type::Named("Int".into()))
    }
    /// Run all checks on the program. Returns a list of errors.
    pub fn check(&mut self, program: &Program) -> Vec<CompilerError> {
        self.build_type_env(program);

        // Fixpoint: infer user-function fallibility before E001 checks (supports mutual recursion).
        loop {
            let before = self.fallibility.symbols.clone();
            for stmt in &program.stmts {
                if let Stmt::Fun {
                    name,
                    return_type,
                    body,
                    fn_or_fallback,
                    ..
                } = stmt
                {
                    self.fallibility
                        .analyze_function(name, return_type, fn_or_fallback, body);
                }
            }
            if self.fallibility.symbols == before {
                break;
            }
        }

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
                    fn_or_fallback,
                    ..
                } => {
                    // Temporarily add function parameters to the type environment.
                    let mut saved: Vec<(String, Option<Type>)> = Vec::new();
                    for p in params {
                        let param_ty = p.ty.clone().unwrap_or_else(|| Type::Named("Int".into()));
                        let old = self.type_env.insert(p.name.clone(), param_ty);
                        saved.push((p.name.clone(), old));
                    }

                    // For generic functions, add type params to type_env so T is known
                    let mut saved_tps: Vec<(String, Option<Type>)> = Vec::new();
                    for tp in type_params {
                        let old = self.type_env.insert(tp.clone(), Type::TypeVar(tp.clone()));
                        saved_tps.push((tp.clone(), old));
                    }

                    if let Some(fb) = fn_or_fallback {
                        if let Some(ret) = return_type {
                            if let Some(err) = self.fallibility.check_r3_fn_or_return_match(
                                ret,
                                &self.try_infer_expr_type(fb),
                                fb.span,
                            ) {
                                errors.push(err);
                            }
                        }
                    }
                    let had_fn_or = self.fallibility.fn_or_fallback;
                    if fn_or_fallback.is_some() {
                        self.fallibility.fn_or_fallback = true;
                    }
                    let allows_propagate = fn_or_fallback.is_none()
                        && name != "main"
                        && self
                            .fallibility
                            .symbols
                            .get(name)
                            .is_some_and(|s| s.is_fallible);
                    let saved_allow = self.fallibility.allow_bare_fallible_in_fn;
                    self.fallibility.allow_bare_fallible_in_fn = allows_propagate;
                    self.collect_expr_errors(body, &mut errors);
                    self.fallibility.allow_bare_fallible_in_fn = saved_allow;
                    self.fallibility.fn_or_fallback = had_fn_or;
                    self.fallibility
                        .analyze_function(name, return_type, fn_or_fallback, body);

                    // HM: patch unannotated param types in type_env from body usage
                    let resolved_param_tys: Vec<Type> = params
                        .iter()
                        .map(|p| {
                            if p.ty.is_some() {
                                p.ty.clone().unwrap()
                            } else if p.name == "self" {
                                Type::Named("Int".into())
                            } else {
                                self.infer_param_type_from_body(&p.name, params, body)
                            }
                        })
                        .collect();
                    if params.iter().any(|p| p.ty.is_none() && p.name != "self") {
                        let ret_ty = return_type
                            .clone()
                            .unwrap_or_else(|| self.try_infer_expr_type(body));
                        let fn_type =
                            Type::Function(resolved_param_tys.clone(), Box::new(ret_ty.clone()));
                        self.type_env.insert(name.clone(), fn_type);
                    }

                    // Validate return type annotation if present
                    if let Some(declared_ret) = return_type {
                        // Skip return type check for generic functions (validated per-instantiation)
                        if type_params.is_empty() || !matches!(declared_ret, Type::TypeVar(_)) {
                            let inferred = self
                                .infer_expr_type(body)
                                .unwrap_or(Type::Named("Int".into()));
                            if !types_compatible(declared_ret, &inferred) {
                                if let Some(err) = Self::check_termination(
                                    declared_ret,
                                    &inferred,
                                    self.current_span,
                                ) {
                                    errors.push(err);
                                } else {
                                    errors.push(CompilerError::new(format!(
                                        "Function '{}' declares return type '{}' but body has type '{}'",
                                        name, declared_ret, inferred
                                    ))
                                    .with_span(self.current_span));
                                }
                            }
                        }
                    }

                    // If no return type annotation, warn when body type differs from the Int default.
                    // This catches the case where someone writes e.g. `fun f() { "hello" }` —
                    // the type checker defaults to `Int` for the return type without warning,
                    // but callers see `Int` when the body actually returns `String`.
                    if return_type.is_none() && type_params.is_empty() {
                        let inferred = self
                            .infer_expr_type(body)
                            .unwrap_or(Type::Named("Int".into()));
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
                        let inferred = self
                            .infer_expr_type(value)
                            .unwrap_or(Type::Named("Int".into()));
                        if !types_compatible(ann, &inferred) {
                            if let Some(err) =
                                Self::check_termination(ann, &inferred, self.current_span)
                            {
                                errors.push(err);
                            } else {
                                errors.push(
                                    CompilerError::new(format!(
                                        "Variable '{}' declared as '{}' but initialized with '{}'",
                                        name, ann, inferred
                                    ))
                                    .with_span(self.current_span),
                                );
                            }
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
                        let inferred = self
                            .infer_expr_type(value)
                            .unwrap_or(Type::Named("Int".into()));
                        if !types_compatible(ann, &inferred) {
                            if let Some(err) =
                                Self::check_termination(ann, &inferred, self.current_span)
                            {
                                errors.push(err);
                            } else {
                                errors.push(
                                    CompilerError::new(format!(
                                        "Constant '{}' declared as '{}' but initialized with '{}'",
                                        name, ann, inferred
                                    ))
                                    .with_span(self.current_span),
                                );
                            }
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

    /// Check for nullable termination violation: T? used where T is expected.
    fn check_termination(
        declared: &Type,
        inferred: &Type,
        span: action_span::Span,
    ) -> Option<CompilerError> {
        FallibilityContext::check_r4_nullable_termination(declared, inferred, span)
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check_source(source: &str) -> Vec<CompilerError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parsing should succeed");
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
    fn test_e006_list_index_needs_or() {
        for src in [
            "fun main() { println(List[][0]) }",
            "fun main() { List[][0] }",
            "fun main() { val lst = List[1, 2]; val i = 0; println(lst[i]) }",
        ] {
            let errors = check_source(src);
            assert!(
                errors
                    .iter()
                    .any(|e| e.code == Some(crate::error::DiagnosticCode::E006)),
                "expected E006 for {:?}, got: {:?}",
                src,
                errors
            );
        }
    }

    #[test]
    fn test_e007_or_unnecessary() {
        let errors = check_source("fun main() { 42 or { 0 } }");
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E007)),
            "expected E007, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_e008_map_var_key_needs_or() {
        let errors =
            check_source("fun main() { val m = Map[\"a\": 1]; val k = \"a\"; println(m[k]) }");
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E008)),
            "expected E008, got: {:?}",
            errors
        );
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
        let errors = check_source("val x: Int = \"hello\"");
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
        let errors = check_source("fun f(x) { x + 1 }");
        assert!(
            errors.is_empty(),
            "HM should infer unannotated param type, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_param_inferred_as_float() {
        let errors = check_source("fun f(x) { x + 1.0 }");
        assert!(
            errors.is_empty(),
            "Expected x inferred as Float, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
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
        assert!(
            !errors.is_empty(),
            "expected type error for mismatched when arms"
        );
    }

    #[test]
    fn test_for_loop_non_iterable() {
        let errors = check_source("for x in 42 { x }");
        // This may or may not produce an error depending on the type checker
        let _ = errors;
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
            val p = Person { name = "Alice", age = "twenty" }
        "#;
        let errors = check_source(src);
        assert!(
            !errors.is_empty(),
            "expected type error for struct field mismatch"
        );
    }

    #[test]
    fn test_null_literal_rejected_at_parse_e010() {
        let mut lexer = Lexer::new("val x: Int = null");
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let result = parser.parse_program();
        assert!(result.is_err(), "null should be rejected at parse time");
        let err = result.unwrap_err();
        assert_eq!(err.code, Some(crate::error::DiagnosticCode::E010));
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
        // This may or may not produce an error depending on the type checker
        let _ = errors;
    }

    #[test]
    fn test_char_type_mismatch() {
        let errors = check_source("val x: Int = 'a'");
        assert!(
            !errors.is_empty(),
            "expected type error for char-to-int assignment"
        );
    }
}
