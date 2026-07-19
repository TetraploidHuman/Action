//! Type checking: HM inference engine (`inference`) + expression inference (`expr_infer`).

mod check_stmt;
mod expr_infer;
mod fallibility;
mod inference;

use crate::fallibility_narrowing::NarrowingContext;

pub use fallibility::FallibilityContext;

pub use crate::type_registry::{EnumInfo, EnumVariantInfo, StructInfo, TypeRegistry};

use crate::ast::*;
use crate::builtin;
use crate::error::CompilerError;
use crate::types::{mangle_name, types_compatible};
use action_span::Span;
use std::collections::{HashMap, HashSet};

/// Type checker: walks the AST and verifies type consistency.
pub struct TypeChecker {
    pub(crate) registry: TypeRegistry,
    pub(crate) type_env: HashMap<String, Type>,
    pub(crate) current_span: Span,
    pub(crate) generic_funs: HashMap<String, Stmt>,
    pub(crate) mutable_vars: HashSet<String>,
    /// Function names registered under mangled keys (arity/type overloads).
    pub(crate) overloaded_names: HashSet<String>,
    /// Declared return type of the function currently being checked (M68).
    pub(crate) current_return_type: Option<Type>,
    pub fallibility: FallibilityContext,
    pub(crate) narrowing: NarrowingContext,
}

impl TypeChecker {
    pub fn new(registry: TypeRegistry) -> Self {
        TypeChecker {
            registry,
            type_env: HashMap::new(),
            current_span: Span::default(),
            generic_funs: HashMap::new(),
            mutable_vars: HashSet::new(),
            overloaded_names: HashSet::new(),
            current_return_type: None,
            fallibility: FallibilityContext::new(),
            narrowing: NarrowingContext::default(),
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
        self.infer_expr_type(expr).unwrap_or(Type::Unit)
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
        let overloaded_names: HashSet<String> = name_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name)
            .collect();
        self.overloaded_names = overloaded_names.clone();

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
                    let saved_ret = self.current_return_type.take();
                    self.current_return_type = return_type.clone();
                    self.collect_expr_errors(body, &mut errors);
                    if let Some(declared_ret) = return_type {
                        self.check_expr_struct_literal_against_expected(
                            declared_ret,
                            body,
                            &mut errors,
                        );
                    }
                    self.current_return_type = saved_ret;
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
                                errors.push(CompilerError::new(format!(
                                    "Function '{}' declares return type '{}' but body has type '{}'",
                                    name, declared_ret, inferred
                                ))
                                .with_span(self.current_span));
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
                        if let (Type::Named(struct_name), ExprKind::StructLiteral(fields)) =
                            (ann, &value.kind)
                        {
                            self.check_struct_literal_against_named(
                                struct_name,
                                fields,
                                value.span,
                                &mut errors,
                            );
                        }
                        let inferred = self
                            .infer_expr_type(value)
                            .unwrap_or(Type::Named("Int".into()));
                        if !types_compatible(ann, &inferred) {
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
                        if let (Type::Named(struct_name), ExprKind::StructLiteral(fields)) =
                            (ann, &value.kind)
                        {
                            self.check_struct_literal_against_named(
                                struct_name,
                                fields,
                                value.span,
                                &mut errors,
                            );
                        }
                        let inferred = self
                            .infer_expr_type(value)
                            .unwrap_or(Type::Named("Int".into()));
                        if !types_compatible(ann, &inferred) {
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
}

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
    fn test_list_index_assign_lvalue_no_e006() {
        let errors = check_source(
            "fun main() { var lst = List[10, 20, 30]; lst[1] = 42; println(lst[0] or { -1 }) }",
        );
        assert!(
            !errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E006)),
            "index assign target must not require or {{}}: {:?}",
            errors
        );
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
    fn test_compile_time_safe_index_no_or() {
        let errors = check_source("fun main() { println(List[1, 2, 3].get(0)) }");
        assert!(
            !errors.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E006) | Some(crate::error::DiagnosticCode::E001)
            )),
            "compile-time safe get should not require or: {:?}",
            errors
        );
    }

    #[test]
    fn test_narrowing_for_i_lt_len_no_or() {
        let errors = check_source(
            "fun f(lst: List[Int]) -> Int { var i = 0; for i < len(lst) { val x = lst.get(i); i = i + 1 }; 0 }",
        );
        assert!(
            !errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E006)),
            "i < lst.len() should narrow lst.get(i): {:?}",
            errors
        );
    }

    #[test]
    fn test_block_or_region_no_inner_or() {
        let errors =
            check_source("fun f() -> Int { { val n = parseInt(\"42\"); return n } or { -1 } }");
        assert!(
            !errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E001)),
            "block or region should cover inner fallible calls: {:?}",
            errors
        );
    }

    #[test]
    fn test_e006_still_required_for_dynamic_index() {
        let errors =
            check_source("fun main() { val lst = List[1, 2]; val i = 0; println(lst[i]) }");
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E006)),
            "dynamic index without or should still error: {:?}",
            errors
        );
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
    fn test_call_arg_index_or_covers_nested_e006() {
        // R6 / R7 share one tree walk: fallible Index inside Call args is covered by outer `or`.
        let errors = check_source(
            "fun main() { val lst = List[1, 2]; val i = 0; println(lst[i]) or { 0 } }",
        );
        assert!(
            !errors.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E006) | Some(crate::error::DiagnosticCode::E007)
            )),
            "print(lst[i]) or {{}} should cover nested index: {:?}",
            errors
        );
    }

    #[test]
    fn test_nested_index_or_covers_inner_e006() {
        let errors = check_source(
            "fun main() { val parts = List[List[1, 2], List[3]]; print(parts[0][0] or { 0 }) }",
        );
        assert!(
            !errors.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E006) | Some(crate::error::DiagnosticCode::E007)
            )),
            "parts[0][0] or {{}} should cover inner index: {:?}",
            errors
        );
    }

    #[test]
    fn test_e004_unknown_call() {
        let errors = check_source("fun main() { noSuchBuiltin(1) }");
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E004)),
            "expected E004, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_e004_unknown_ufcs_method() {
        let errors = check_source("fun main() { val x = List[1, 2]; x.noSuchMethod() }");
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E004)),
            "expected E004 for unknown UFCS, got: {:?}",
            errors
        );
        let ok = check_source(
            r#"fun main() {
                val lst = List[1, 2, 3]
                println(lst.len())
                val m = Map[1: 10]
                println(m.keys().len())
                println(m.union(Map[2: 20]).len())
            }"#,
        );
        assert!(
            !ok
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E004)),
            "known UFCS must not E004: {:?}",
            ok
        );
    }

    #[test]
    fn test_m56_registered_map_hof_and_delay_typecheck() {
        let errors = check_source(
            r#"fun main() {
                val m = Map[1: 10, 2: 20]
                val f = mapFilter(m) { k, v -> true }
                val g = mapMapValues(m) { v -> v }
                val s = mapFold(0, m) { acc, k, v -> acc + v }
                delay(1)
                println(f.len())
                println(g.len())
                println(s)
            }"#,
        );
        assert!(
            !errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E004)),
            "registered Map HOFs / delay must not E004: {:?}",
            errors
        );
    }

    #[test]
    fn test_m57_datetime_apis_typecheck() {
        let errors = check_source(
            r#"fun main() {
                val d = date(2026, 6, 1) or { {year = 0, month = 0, day = 0} }
                val dt = datetime(2026, 6, 1, 12, 0, 0) or {
                    {year = 0, month = 0, day = 0, hour = 0, minute = 0, second = 0}
                }
                val p = parseDate("%d-%d-%d", "2026-06-15") or { {year = 0, month = 0, day = 0} }
                val _t = today()
                val _n = now()
                println(year(d))
                println(month(d))
                println(day(d))
                println(weekday(d))
                println(hour(dt))
                println(day(addDays(d, 14)))
                println(year(p))
                println(format(dt, "%Y-%m-%d"))
            }"#,
        );
        assert!(
            !errors.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E004) | Some(crate::error::DiagnosticCode::E001)
                    | Some(crate::error::DiagnosticCode::E007)
            )),
            "datetime APIs must typecheck with or{{}} on fallible forms: {:?}",
            errors
        );
    }

    #[test]
    fn test_m61_partition_pair_index_fallibility() {
        // Slot index on the pair is safe; nested list index still needs or {}.
        let ok = check_source(
            r#"fun main() {
                val nums = List[1, 2, 3]
                val parts = partition(nums) { x -> x % 2 == 0 }
                println(parts[0][0] or { 0 })
                println(parts[1].len())
            }"#,
        );
        assert!(
            !ok.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E006)
                    | Some(crate::error::DiagnosticCode::E007)
                    | Some(crate::error::DiagnosticCode::E004)
            )),
            "honest partition pair should typecheck: {:?}",
            ok
        );
        let bare = check_source(
            r#"fun main() {
                val nums = List[1, 2, 3]
                val parts = partition(nums) { x -> true }
                println(parts[0][0])
            }"#,
        );
        assert!(
            bare
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E006)),
            "nested list index without or should E006: {:?}",
            bare
        );
        let false_or = check_source(
            r#"fun main() {
                val nums = List[1, 2, 3]
                val parts = partition(nums) { x -> true }
                println((parts[1] or { List[] }).len())
            }"#,
        );
        assert!(
            false_or
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E007)),
            "or on tuple slot should E007: {:?}",
            false_or
        );
    }

    #[test]
    fn test_m63_next_int_pair_index() {
        let ok = check_source(
            r#"fun main() {
                val p = nextInt(Random_new(42), 1, 10)
                println(p[1])
                val p2 = nextInt(p[0], 1, 10)
                println(p2[1])
            }"#,
        );
        assert!(
            !ok.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E006)
                    | Some(crate::error::DiagnosticCode::E007)
                    | Some(crate::error::DiagnosticCode::E004)
                    | Some(crate::error::DiagnosticCode::E005)
            )),
            "honest nextInt pair should typecheck: {:?}",
            ok
        );
    }

    #[test]
    fn test_m64_struct_index_e005() {
        let oob = check_source(
            r#"fun main() {
                val p = nextInt(Random_new(1), 0, 5)
                println(p[2])
            }"#,
        );
        assert!(
            oob.iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E005)),
            "OOB tuple index should E005: {:?}",
            oob
        );
        assert!(
            !oob.iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E006)),
            "tuple OOB must not be E006: {:?}",
            oob
        );

        let neg = check_source(
            r#"fun main() {
                val p = nextInt(Random_new(1), 0, 5)
                println(p[-1])
            }"#,
        );
        assert!(
            neg.iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E005)),
            "negative tuple index should E005: {:?}",
            neg
        );

        let dyn_idx = check_source(
            r#"fun main() {
                val i = 0
                val p = nextInt(Random_new(1), 0, 5)
                println(p[i])
            }"#,
        );
        assert!(
            dyn_idx
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E005)),
            "non-literal tuple index should E005: {:?}",
            dyn_idx
        );

        let ok_parts = check_source(
            r#"fun main() {
                val nums = List[1, 2, 3]
                val parts = partition(nums) { x -> x % 2 == 0 }
                println(parts[0].len())
                println(parts[1].len())
            }"#,
        );
        assert!(
            !ok_parts.iter().any(|e| e.code == Some(crate::error::DiagnosticCode::E005)),
            "in-bounds pair index must stay valid: {:?}",
            ok_parts
        );
    }

    #[test]
    fn test_m65_unknown_struct_field_e013() {
        let bad = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                val p = { x = 1, y = 2 }
                println(p.z)
            }
            "#,
        );
        assert!(
            bad.iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E013)),
            "unknown named field should E013: {:?}",
            bad
        );

        let ok = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                val p = { x = 1, y = 2 }
                println(p.x + p.y)
            }
            "#,
        );
        assert!(
            !ok.iter().any(|e| e.code == Some(crate::error::DiagnosticCode::E013)),
            "known fields must stay valid: {:?}",
            ok
        );

        // UFCS on a named struct must not be misdiagnosed as missing field.
        let ufcs = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun doubleX(p: Point) -> Int { p.x * 2 }
            fun main() {
                val p = { x = 3, y = 4 }
                println(p.doubleX())
            }
            "#,
        );
        assert!(
            !ufcs.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E013)
                    | Some(crate::error::DiagnosticCode::E004)
            )),
            "struct UFCS method call should typecheck: {:?}",
            ufcs
        );
    }

    #[test]
    fn test_m67_struct_literal_and_assign_hygiene() {
        let missing = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                val p: Point = { x = 1 }
            }
            "#,
        );
        assert!(
            missing
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E015)),
            "missing field under Point ann should E015: {:?}",
            missing
        );

        let extra = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                val p: Point = { x = 1, y = 2, z = 3 }
            }
            "#,
        );
        assert!(
            extra
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E013)),
            "extra field under Point ann should E013: {:?}",
            extra
        );

        let ok = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                val p: Point = { y = 2, x = 1 }
                println(p.x + p.y)
            }
            "#,
        );
        assert!(
            !ok.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E013)
                    | Some(crate::error::DiagnosticCode::E015)
            )),
            "order-independent complete literal should be OK: {:?}",
            ok
        );

        let assign = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                var p: Point = { x = 1, y = 2 }
                p.z = 3
            }
            "#,
        );
        assert!(
            assign
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E013)),
            "assign to unknown field should E013: {:?}",
            assign
        );
    }

    #[test]
    fn test_m68_struct_literal_return_and_args() {
        let ret_missing = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun make() -> Point { { x = 1 } }
            fun main() { }
            "#,
        );
        assert!(
            ret_missing
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E015)),
            "return body missing field should E015: {:?}",
            ret_missing
        );

        let ret_stmt = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun make() -> Point { return { x = 1 } }
            fun main() { }
            "#,
        );
        assert!(
            ret_stmt
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E015)),
            "explicit return missing field should E015: {:?}",
            ret_stmt
        );

        let arg_missing = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun take(p: Point) -> Int { p.x }
            fun main() { println(take({ x = 1 })) }
            "#,
        );
        assert!(
            arg_missing
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E015)),
            "call arg missing field should E015: {:?}",
            arg_missing
        );

        let arg_extra = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun take(p: Point) -> Int { p.x }
            fun main() { println(take({ x = 1, y = 2, z = 3 })) }
            "#,
        );
        assert!(
            arg_extra
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E013)),
            "call arg extra field should E013: {:?}",
            arg_extra
        );

        let ok = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun make() -> Point { { y = 2, x = 1 } }
            fun take(p: Point) -> Int { p.x + p.y }
            fun main() {
                println(take(make()))
                var p: Point = { x = 0, y = 0 }
                p = { y = 9, x = 8 }
                println(p.x)
            }
            "#,
        );
        assert!(
            !ok.iter().any(|e| matches!(
                e.code,
                Some(crate::error::DiagnosticCode::E013)
                    | Some(crate::error::DiagnosticCode::E015)
            )),
            "complete reorder/return/assign should stay clean: {:?}",
            ok
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
        let errors = check_source("val x = 1 + true");
        assert!(
            !errors.is_empty(),
            "Expected type error for Bool Add"
        );
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("bool") && (msg.contains("arithmetic") || msg.contains("+")),
            "Expected Add-on-bool error, got: {}",
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
    fn test_unary_not_non_bool() {
        let errors = check_source("val x = not 1");
        assert!(!errors.is_empty(), "Expected type error for not Int");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("bool") && (msg.contains("!") || msg.contains("not") || msg.contains("unary")),
            "Expected unary-not Bool error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_unary_neg_rejects_bool_and_string() {
        let errors = check_source("val x = -true");
        assert!(!errors.is_empty(), "Expected type error for -Bool");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("bool") && (msg.contains("-") || msg.contains("unary") || msg.contains("supported")),
            "Expected unary-neg Bool error, got: {}",
            errors[0].message
        );
        let errors = check_source("val x = -\"hi\"");
        assert!(!errors.is_empty(), "Expected type error for -String");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("string") && (msg.contains("-") || msg.contains("unary") || msg.contains("supported")),
            "Expected unary-neg String error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_unary_pos_rejects_bool_and_string() {
        let errors = check_source("val x = +true");
        assert!(!errors.is_empty(), "Expected type error for +Bool");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("bool") && (msg.contains("+") || msg.contains("unary") || msg.contains("supported")),
            "Expected unary-pos Bool error, got: {}",
            errors[0].message
        );
        let errors = check_source("val x = +\"hi\"");
        assert!(!errors.is_empty(), "Expected type error for +String");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("string") && (msg.contains("+") || msg.contains("unary") || msg.contains("supported")),
            "Expected unary-pos String error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_field_index_assign_rejects_immutable_root() {
        let errors = check_source(
            r#"
            type Point = {x: Int, y: Int}
            fun main() -> Int {
                val p: Point = {x = 0, y = 0}
                p.x = 1
                return p.x
            }
            "#,
        );
        assert!(
            !errors.is_empty(),
            "Expected immutable field-assign error"
        );
        assert!(
            errors[0].message.to_lowercase().contains("immutable"),
            "Expected immutable message, got: {}",
            errors[0].message
        );
        let errors = check_source(
            r#"
            fun main() -> Int {
                val xs: List = List[1, 2, 3]
                xs[0] = 9
                return 0
            }
            "#,
        );
        assert!(
            !errors.is_empty(),
            "Expected immutable index-assign error"
        );
        assert!(
            errors[0].message.to_lowercase().contains("immutable"),
            "Expected immutable message, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_range_bounds_must_be_int() {
        let errors = check_source(
            "fun main() -> Int {\n\
                 var s: Int = 0\n\
                 for i in 1..true { s = s + 1 }\n\
                 return s\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for Bool range end");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("range") && msg.contains("int"),
            "Expected range Int-bound error, got: {}",
            errors[0].message
        );
        let errors = check_source(
            "fun main() -> Int {\n\
                 var s: Int = 0\n\
                 for i in \"a\"..5 { s = s + 1 }\n\
                 return s\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for String range start");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("range") && msg.contains("int"),
            "Expected range Int-bound error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_when_condition_must_be_bool() {
        let errors = check_source("fun main() -> Int { return if 1 { 0 } else { 1 } }");
        assert!(!errors.is_empty(), "Expected type error for Int if condition");
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E017)),
            "Expected E017 for non-Bool if condition, got: {:?}",
            errors
        );
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("if") && msg.contains("bool") && msg.contains("condition"),
            "Expected if-condition Bool error, got: {}",
            errors[0].message
        );
        let errors = check_source(
            "fun main() -> Int {\n\
                 return when {\n\
                     1 -> 0\n\
                     else -> 1\n\
                 }\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for Int when-chain condition");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("when") && msg.contains("bool") && msg.contains("condition"),
            "Expected when-condition Bool error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_for_condition_must_be_bool() {
        let errors = check_source(
            "fun main() -> Int {\n\
                 var x: Int = 1\n\
                 var s: Int = 0\n\
                 for x { s = s + 1; break }\n\
                 return s\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for Int for-condition");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("for") && msg.contains("bool") && msg.contains("condition"),
            "Expected for-condition Bool error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_when_guard_non_bool() {
        let errors = check_source(
            "enum Color { Red, Green, Blue }\n\
             fun main() -> Int {\n\
                 val c: Color = Red\n\
                 val code: Int = when c { Red and 1 -> 0 else -> 1 }\n\
                 return code\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for non-Bool when guard");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("guard") && msg.contains("bool"),
            "Expected when-guard Bool error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_let_named_init_mismatch() {
        let errors = check_source(
            "type Point = {x: Int, y: Int}\n\
             fun main() -> Int {\n\
                 val p: Point = 1\n\
                 return 0\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for Point = Int");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("point") && msg.contains("int"),
            "Expected let ann↔init mismatch, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_assign_named_rhs_mismatch() {
        let errors = check_source(
            "type Point = {x: Int, y: Int}\n\
             fun main() -> Int {\n\
                 var p: Point = {x = 0, y = 0}\n\
                 p = 1\n\
                 return 0\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for Point = Int assign");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("assign") || (msg.contains("point") && msg.contains("int")),
            "Expected assign type mismatch, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_field_assign_type_mismatch() {
        let errors = check_source(
            "type Point = {x: Int, y: Int}\n\
             fun main() -> Int {\n\
                 var p: Point = {x = 0, y = 0}\n\
                 p.x = true\n\
                 return 0\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for p.x = Bool");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("assign") || (msg.contains("int") && msg.contains("bool")),
            "Expected field-assign type mismatch, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_field_assign_unknown() {
        let errors = check_source(
            "type Point = {x: Int, y: Int}\n\
             fun main() -> Int {\n\
                 var p: Point = {x = 0, y = 0}\n\
                 p.z = 1\n\
                 return 0\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for unknown field z");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("z") || msg.contains("unknown") || msg.contains("field"),
            "Expected unknown-field error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_list_mixed_element_types() {
        let errors = check_source("fun main() -> Int { val xs: List = List[1, \"a\"]; return 0 }");
        assert!(!errors.is_empty(), "Expected type error for mixed List elems");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("list") || msg.contains("element") || (msg.contains("int") && msg.contains("string")),
            "Expected list homogeneity error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_map_mixed_value_types() {
        let errors = check_source(
            "fun main() -> Int { val m: Map = Map[\"a\": 1, \"b\": true]; return 0 }",
        );
        assert!(!errors.is_empty(), "Expected type error for mixed Map values");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("map") || msg.contains("entry") || (msg.contains("int") && msg.contains("bool")),
            "Expected map homogeneity error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_list_index_assign_type_mismatch() {
        let errors = check_source(
            "fun main() -> Int {\n\
                 var xs: List = List[1, 2, 3]\n\
                 xs[0] = true\n\
                 return 0\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for xs[0] = Bool");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("assign") || (msg.contains("int") && msg.contains("bool")),
            "Expected index-assign type mismatch, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_map_index_assign_type_mismatch() {
        let errors = check_source(
            "fun main() -> Int {\n\
                 var m: Map = Map[\"a\": 1]\n\
                 m[\"a\"] = true\n\
                 return 0\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for m[k] = Bool");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("assign") || (msg.contains("int") && msg.contains("bool")),
            "Expected map index-assign type mismatch, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_list_index_key_must_be_int() {
        let errors = check_source(
            "fun main() -> Int {\n\
                 var xs: List = List[1, 2, 3]\n\
                 xs[true] = 1\n\
                 return 0\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for xs[Bool]");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("index") || (msg.contains("int") && msg.contains("bool")),
            "Expected list index key error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_string_index_key_must_be_int() {
        let errors = check_source(
            "fun main() -> Int {\n\
                 val s: String = \"ab\"\n\
                 return s[true]\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for s[Bool]");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("index") || (msg.contains("int") && msg.contains("bool")),
            "Expected string index key error, got: {}",
            errors[0].message
        );
        let errors = check_source(
            "fun main() -> Int {\n\
                 val s: String = \"ab\"\n\
                 return s[\"x\"]\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for s[String]");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("index") || (msg.contains("int") && msg.contains("string")),
            "Expected string index key error, got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_map_index_key_must_be_string() {
        let errors = check_source(
            "fun main() -> Int {\n\
                 var m: Map = Map[\"a\": 1]\n\
                 m[1] = 2\n\
                 return 0\n\
             }",
        );
        assert!(!errors.is_empty(), "Expected type error for m[Int]");
        let msg = errors[0].message.to_lowercase();
        assert!(
            msg.contains("index") || (msg.contains("string") && msg.contains("int")),
            "Expected map index key error, got: {}",
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
    fn test_m66_unknown_enum_constructor_e014() {
        let alone = check_source(
            "enum Color { Red, Blue } fun f(c: Color) -> Int { when c { Fake -> 1 } }",
        );
        assert!(
            alone
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E014)),
            "unknown-only constructor should E014: {:?}",
            alone
        );

        let with_real = check_source(
            "enum Color { Red, Blue } fun f(c: Color) -> Int { when c { Red -> 1; Blue -> 2; Fake -> 3 } }",
        );
        assert!(
            with_real
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E014)),
            "unknown constructor among real arms should E014: {:?}",
            with_real
        );
        assert!(
            !with_real.iter().any(|e| e.message.to_lowercase().contains("non-exhaustive")),
            "Red+Blue+Fake must not look incomplete: {:?}",
            with_real
        );

        let ok = check_source(
            "enum Color { Red, Blue } fun f(c: Color) -> Int { when c { Red -> 1; Blue -> 2 } }",
        );
        assert!(
            !ok.iter().any(|e| e.code == Some(crate::error::DiagnosticCode::E014)),
            "exhaustive known variants must stay clean: {:?}",
            ok
        );

        let call = check_source("enum Color { Red, Blue } fun main() { Fake() }");
        assert!(
            call.iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E004)),
            "unknown constructor call stays E004: {:?}",
            call
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
    fn test_if_continue_is_not_e018() {
        let errors = check_source(
            "fun main() {\n\
                 for x in List[1, 2, 3] {\n\
                     if x % 2 == 0 { x } else { continue }\n\
                 }\n\
             }",
        );
        assert!(
            !errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E018)),
            "continue arm must not trigger E018, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_if_branch_type_mismatch_e018() {
        let errors = check_source("fun main() { val x = if true { 1 } else { \"a\" } }");
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E018)),
            "Expected E018 for mismatched if branches, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_if_multi_stmt_block_types() {
        let errors = check_source(
            "fun main() -> Int {\n\
                 return if true { val x = 1; x } else { 0 }\n\
             }",
        );
        assert!(
            errors.is_empty(),
            "multi-stmt if block should typecheck, got: {:?}",
            errors
        );
        let errors = check_source(
            "fun main() -> Int {\n\
                 return if true { val x = 1; x } else { \"no\" }\n\
             }",
        );
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E018)),
            "Expected E018 for multi-stmt if vs string else, got: {:?}",
            errors
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
        // M70: brace literal under Named annotation (not the stale `Person { … }` form).
        let src = r#"
            type Person = { name: String, age: Int }
            fun main() {
                val p: Person = { name = "Alice", age = "twenty" }
            }
            "#;
        let errors = check_source(src);
        assert!(
            errors
                .iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E016)),
            "expected E016 for field type mismatch: {:?}",
            errors
        );
    }

    #[test]
    fn test_m70_struct_literal_field_value_types() {
        let bad = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                val p: Point = { y = 1, x = "hi" }
            }
            "#,
        );
        assert!(
            bad.iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E016)),
            "String in Int field should E016: {:?}",
            bad
        );

        let ret = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun make() -> Point { { x = true, y = 2 } }
            fun main() { }
            "#,
        );
        assert!(
            ret.iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E016)),
            "Bool in Int field on return should E016: {:?}",
            ret
        );

        let ok = check_source(
            r#"
            type Person = { name: String, age: Int }
            fun take(p: Person) -> Int { p.age }
            fun main() {
                val p: Person = { age = 20, name = "Alice" }
                println(take({ name = "Bob", age = 21 }))
            }
            "#,
        );
        assert!(
            !ok.iter().any(|e| e.code == Some(crate::error::DiagnosticCode::E016)),
            "matching field value types must stay clean: {:?}",
            ok
        );
    }

    #[test]
    fn test_m71_untyped_unique_shape_field_value_types() {
        let bad = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                val p = { x = "a", y = 1 }
                println(p.x)
            }
            "#,
        );
        assert!(
            bad.iter()
                .any(|e| e.code == Some(crate::error::DiagnosticCode::E016)),
            "untyped unique-shape literal must E016 on field values: {:?}",
            bad
        );

        let ok = check_source(
            r#"
            type Point = { x: Int, y: Int }
            fun main() {
                val p = { y = 2, x = 1 }
                println(p.x + p.y)
            }
            "#,
        );
        assert!(
            !ok.iter().any(|e| e.code == Some(crate::error::DiagnosticCode::E016)),
            "untyped complete matching literal must stay clean: {:?}",
            ok
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
