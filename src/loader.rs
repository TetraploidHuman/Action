use crate::ast::*;
use crate::error::CompilerError;
use crate::lexer::Span;
use crate::typecheck::{TypeChecker, TypeRegistry};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Register builtin struct types (Date, DateTime, Random)
pub fn builtin_types(program: &Program) -> Vec<Stmt> {
    let has_date = program
        .stmts
        .iter()
        .any(|s| matches!(s, Stmt::TypeAlias { name, .. } if name == "Date"));
    let has_datetime = program
        .stmts
        .iter()
        .any(|s| matches!(s, Stmt::TypeAlias { name, .. } if name == "DateTime"));
    let has_random = program
        .stmts
        .iter()
        .any(|s| matches!(s, Stmt::TypeAlias { name, .. } if name == "Random"));

    let mut builtins = Vec::new();
    if !has_date {
        builtins.push(Stmt::TypeAlias {
            name: "Date".into(),
            type_params: vec![],
            definition: Type::Struct(vec![
                ("year".into(), Type::Named("Int".into())),
                ("month".into(), Type::Named("Int".into())),
                ("day".into(), Type::Named("Int".into())),
            ]),
            span: Span::default(),
        });
    }
    if !has_datetime {
        builtins.push(Stmt::TypeAlias {
            name: "DateTime".into(),
            type_params: vec![],
            definition: Type::Struct(vec![
                ("year".into(), Type::Named("Int".into())),
                ("month".into(), Type::Named("Int".into())),
                ("day".into(), Type::Named("Int".into())),
                ("hour".into(), Type::Named("Int".into())),
                ("minute".into(), Type::Named("Int".into())),
                ("second".into(), Type::Named("Int".into())),
            ]),
            span: Span::default(),
        });
    }
    if !has_random {
        builtins.push(Stmt::TypeAlias {
            name: "Random".into(),
            type_params: vec![],
            definition: Type::Struct(vec![("seed".into(), Type::Named("Int".into()))]),
            span: Span::default(),
        });
    }
    builtins
}

/// Validate that a module name does not contain directory traversal or other
/// path-injection characters (defense-in-depth: module names come from parsed source).
fn validate_module_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Module name is empty".to_string());
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(format!(
            "Invalid module name '{}': must not contain path separators or traversal",
            name
        ));
    }
    Ok(())
}

/// Load a single module file and return its statements
fn load_module(module_name: &str, search_dirs: &[PathBuf]) -> Result<Vec<Stmt>, String> {
    validate_module_name(module_name)?;
    for ext in &["atom", "at"] {
        let file_name = format!("{}.{}", module_name, ext);
        for dir in search_dirs {
            let path = dir.join(&file_name);
            if let Ok(canon) = path.canonicalize() {
                if let Ok(dir_canon) = dir.canonicalize() {
                    if !canon.starts_with(&dir_canon) {
                        continue;
                    }
                }
            }
            if path.exists() {
                let source = fs::read_to_string(&path)
                    .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;
                let mut lexer = crate::lexer::Lexer::new(&source);
                let tokens = lexer.tokenize();
                let lexer_errors = lexer.take_errors();
                if !lexer_errors.is_empty() {
                    let first = &lexer_errors[0];
                    return Err(format!("Lexer error in {}: {}", file_name, first));
                }
                let mut parser = crate::parser::Parser::new(tokens);
                let program = parser.parse_program().map_err(|e| {
                    format!(
                        "Parse error in {} at line {}, col {}: {}",
                        file_name, e.line, e.col, e.message
                    )
                })?;
                return Ok(program.stmts);
            }
        }
    }
    Err(format!(
        "Module '{}' not found (looked for {}.atom or {}.at)",
        module_name, module_name, module_name
    ))
}

/// Resolve import statements by loading module files and adding their statements.
/// Performs recursive resolution to handle transitive imports and detects cycles.
pub fn resolve_imports(program: &Program, search_dirs: &[PathBuf]) -> Result<Vec<Stmt>, String> {
    let mut extra_stmts = Vec::new();
    let mut loaded: HashSet<String> = HashSet::new();
    let mut visiting: HashSet<String> = HashSet::new();

    fn resolve_module(
        module: &str,
        items: &Option<Vec<String>>,
        alias: &Option<String>,
        search_dirs: &[PathBuf],
        loaded: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        extra_stmts: &mut Vec<Stmt>,
    ) -> Result<(), String> {
        if loaded.contains(module) {
            return Ok(());
        }
        if visiting.contains(module) {
            return Err(format!(
                "Circular import detected: module '{}' is part of an import cycle",
                module
            ));
        }
        visiting.insert(module.to_string());

        let module_stmts = load_module(module, search_dirs)?;
        let prefix = alias.as_deref().unwrap_or(module);

        let exported: Option<HashSet<String>> =
            module_stmts.iter().find_map(|s| {
                if let Stmt::Module { exports, .. } = s {
                    Some(
                        exports
                            .iter()
                            .filter_map(|e| match e {
                                ExportItem::Function(name)
                                | ExportItem::Constant(name)
                                | ExportItem::Type(name) => Some(name.clone()),
                            })
                            .collect(),
                    )
                } else {
                    None
                }
            });

        let mut stmts_to_check: Vec<&Stmt> = Vec::new();
        for m_stmt in &module_stmts {
            match m_stmt {
                Stmt::Module { body, .. } => {
                    for b in body {
                        stmts_to_check.push(b);
                    }
                }
                _ => stmts_to_check.push(m_stmt),
            }
        }

        for m_stmt in &stmts_to_check {
            match m_stmt {
                Stmt::Fun { name, params, return_type, body, is_single_expr, type_params, .. } => {
                    let imported_name = if items.is_some() { name.clone() } else { format!("{}_{}", prefix, name) };
                    let should_import = if let Some(ref its) = items { its.contains(name) }
                        else if let Some(ref exported_set) = exported { exported_set.contains(name) }
                        else { true };
                    if should_import {
                        extra_stmts.push(Stmt::Fun {
                            name: imported_name,
                            params: params.clone(),
                            return_type: return_type.clone(),
                            body: body.clone(),
                            type_params: type_params.clone(),
                            is_single_expr: *is_single_expr,
                            is_test: false,
                            span: Span::default(),
                        });
                    }
                }
                Stmt::Const { name, type_ann, value, .. } => {
                    let should_import = if let Some(ref its) = items { its.contains(name) }
                        else if let Some(ref exported_set) = exported { exported_set.contains(name) }
                        else { true };
                    if should_import {
                        let imported_name = if items.is_some() { name.clone() } else { format!("{}_{}", prefix, name) };
                        extra_stmts.push(Stmt::Const {
                            name: imported_name,
                            type_ann: type_ann.clone(),
                            value: value.clone(),
                            span: Span::default(),
                        });
                    }
                }
                Stmt::TypeAlias { name, type_params, definition, .. } => {
                    let should_import = if let Some(ref its) = items { its.contains(name) }
                        else if let Some(ref exported_set) = exported { exported_set.contains(name) }
                        else { true };
                    if should_import {
                        let imported_name = if items.is_some() { name.clone() } else { format!("{}_{}", prefix, name) };
                        extra_stmts.push(Stmt::TypeAlias {
                            name: imported_name,
                            type_params: type_params.clone(),
                            definition: definition.clone(),
                            span: Span::default(),
                        });
                    }
                }
                Stmt::Enum { name, type_params, variants, .. } => {
                    let should_import = if let Some(ref its) = items { its.contains(name) }
                        else if let Some(ref exported_set) = exported { exported_set.contains(name) }
                        else { true };
                    if should_import {
                        let imported_name = if items.is_some() { name.clone() } else { format!("{}_{}", prefix, name) };
                        extra_stmts.push(Stmt::Enum {
                            name: imported_name,
                            type_params: type_params.clone(),
                            variants: variants.clone(),
                            span: Span::default(),
                        });
                    }
                }
                Stmt::Extension { type_name, methods, .. } => {
                    for method in methods {
                        if let Stmt::Fun { name, params, return_type, body, is_single_expr, type_params, .. } = method {
                            let fn_name = format!("{}_{}", type_name, name);
                            extra_stmts.push(Stmt::Fun {
                                name: fn_name,
                                params: params.clone(),
                                return_type: return_type.clone(),
                                body: body.clone(),
                                type_params: type_params.clone(),
                                is_single_expr: *is_single_expr,
                                is_test: false,
                                span: Span::default(),
                            });
                        }
                    }
                }
                Stmt::External { name, params, return_type, .. } => {
                    extra_stmts.push(Stmt::External {
                        name: name.clone(),
                        params: params.clone(),
                        return_type: return_type.clone(),
                        span: Span::default(),
                    });
                }
                Stmt::ExternalType { name, .. } => {
                    extra_stmts.push(Stmt::ExternalType {
                        name: name.clone(),
                        span: Span::default(),
                    });
                }
                _ => {}
            }
        }

        loaded.insert(module.to_string());
        visiting.remove(module);
        Ok(())
    }

    for stmt in &program.stmts {
        if let Stmt::Import { module, items, alias, .. } = stmt {
            resolve_module(module, items, alias, search_dirs, &mut loaded, &mut visiting, &mut extra_stmts)?;
        }
    }

    Ok(extra_stmts)
}

/// Transform `math.add(...)` and `math.PI` into `math_add(...)` and `math_PI`
pub fn transform_module_access(program: &mut Program) {
    use std::collections::HashSet;
    let mut module_prefixes: HashSet<String> = HashSet::new();
    for stmt in &program.stmts {
        if let Stmt::Import { module, items, alias, .. } = stmt {
            if items.is_none() {
                let prefix = alias.as_ref().unwrap_or(module);
                module_prefixes.insert(prefix.clone());
            }
        }
    }
    if module_prefixes.is_empty() {
        return;
    }

    fn transform_expr(expr: &mut Expr, prefixes: &HashSet<String>) {
        if let Expr::FieldAccess(ref base, ref field) = expr {
            if let Expr::Ident(ref ident) = **base {
                if prefixes.contains(ident) {
                    *expr = Expr::Ident(format!("{}_{}", ident, field));
                    return;
                }
            }
        }
        match expr {
            Expr::Call { func, args, trailing_lambda } => {
                transform_expr(func, prefixes);
                for arg in args.iter_mut() { transform_expr(arg, prefixes); }
                if let Some(ref mut lambda) = trailing_lambda { transform_expr(lambda, prefixes); }
            }
            Expr::Binary(lhs, _, rhs) => { transform_expr(lhs, prefixes); transform_expr(rhs, prefixes); }
            Expr::Unary(_, operand) => { transform_expr(operand, prefixes); }
            Expr::FieldAccess(base, _) => { transform_expr(base, prefixes); }
            Expr::Index(base, idx) => { transform_expr(base, prefixes); transform_expr(idx, prefixes); }
            Expr::Range(start, end) => { transform_expr(start, prefixes); transform_expr(end, prefixes); }
            Expr::Tuple(elements) => { for (_, e) in elements.iter_mut() { transform_expr(e, prefixes); } }
            Expr::StructLiteral(fields) => { for (_, e) in fields.iter_mut() { transform_expr(e, prefixes); } }
            Expr::MapLiteral(entries) => { for (k, v) in entries.iter_mut() { transform_expr(k, prefixes); transform_expr(v, prefixes); } }
            Expr::SetLiteral(items) => { for item in items.iter_mut() { transform_expr(item, prefixes); } }
            Expr::Block(stmts) => { for s in stmts.iter_mut() { transform_stmt(s, prefixes); } }
            Expr::Lambda { body, .. } => { transform_expr(body, prefixes); }
            Expr::When(w) => {
                match &mut w.kind {
                    WhenKind::OneLine { condition, then_expr, else_expr } => {
                        transform_expr(condition, prefixes); transform_expr(then_expr, prefixes); transform_expr(else_expr, prefixes);
                    }
                    WhenKind::ValueMatch { value, arms } => {
                        transform_expr(value, prefixes);
                        for arm in arms.iter_mut() {
                            if let Some(ref mut g) = arm.guard { transform_expr(g, prefixes); }
                            transform_expr(&mut arm.body, prefixes);
                        }
                    }
                    WhenKind::ConditionChain { arms } => {
                        for arm in arms.iter_mut() {
                            if let Some(ref mut g) = arm.guard { transform_expr(g, prefixes); }
                            transform_expr(&mut arm.body, prefixes);
                        }
                    }
                }
            }
            Expr::For(fr) => {
                match &mut fr.kind {
                    ForKind::Iterate { iterable, body, .. } | ForKind::IterateWithIndex { iterable, body, .. } => {
                        transform_expr(iterable, prefixes); transform_expr(body, prefixes);
                    }
                    ForKind::Condition { condition, body } => {
                        transform_expr(condition, prefixes); transform_expr(body, prefixes);
                    }
                    ForKind::Infinite { body } => { transform_expr(body, prefixes); }
                    ForKind::NestedIterate { bindings, body, .. } => {
                        for (_, e) in bindings.iter_mut() { transform_expr(e, prefixes); }
                        transform_expr(body, prefixes);
                    }
                }
            }
            Expr::Assign { target, value, .. } => { transform_expr(target, prefixes); transform_expr(value, prefixes); }
            Expr::Unsafe(inner) => { transform_expr(inner, prefixes); }
            Expr::Copy(inner) => { transform_expr(inner, prefixes); }
            Expr::Null => {}
            Expr::OrBlock { nullable, fallback } => { transform_expr(nullable, prefixes); transform_expr(fallback, prefixes); }
            Expr::StringInterpolate(parts) => {
                for part in parts.iter_mut() {
                    if let StringPart::Expr(ref mut e) = part { transform_expr(e, prefixes); }
                }
            }
            _ => {}
        }
    }

    fn transform_stmt(stmt: &mut Stmt, prefixes: &HashSet<String>) {
        match stmt {
            Stmt::Fun { body, .. } => { transform_expr(body, prefixes); }
            Stmt::Let { value, .. } => { transform_expr(value, prefixes); }
            Stmt::Const { value, .. } => { transform_expr(value, prefixes); }
            Stmt::Expr { expr, .. } => { transform_expr(expr, prefixes); }
            Stmt::Return { value, .. } => { if let Some(ref mut e) = value { transform_expr(e, prefixes); } }
            Stmt::Module { body, .. } => { for s in body.iter_mut() { transform_stmt(s, prefixes); } }
            Stmt::Export { stmt, .. } => { transform_stmt(stmt, prefixes); }
            Stmt::Destructure { value, .. } => { transform_expr(value, prefixes); }
            Stmt::Extension { methods, .. } => { for m in methods.iter_mut() { transform_stmt(m, prefixes); } }
            Stmt::External { .. } | Stmt::ExternalType { .. } | Stmt::Enum { .. }
            | Stmt::TypeAlias { .. } | Stmt::Import { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
        }
    }

    for stmt in &mut program.stmts {
        transform_stmt(stmt, &module_prefixes);
    }
}

/// Load stdlib source files
pub fn load_stdlib() -> Result<Vec<Stmt>, String> {
    let mut stmts = Vec::new();
    let exe_lib = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("..").join("lib")))
        .unwrap_or_default();
    let _cwd_lib = std::env::current_dir()
        .map_err(|e| format!("Cannot get current dir: {}", e))?
        .join("lib");
    for file_name in &[] as &[&str] {
        let path = [&exe_lib, &_cwd_lib]
            .iter()
            .map(|d| d.join(file_name))
            .find(|p| p.exists());
        if let Some(path) = path {
            let source = fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;
            let mut lexer = crate::lexer::Lexer::new(&source);
            let tokens = lexer.tokenize();
            let lexer_errors = lexer.take_errors();
            if !lexer_errors.is_empty() {
                return Err(format!("Lexer error in {}: {}", file_name, lexer_errors[0]));
            }
            let mut parser = crate::parser::Parser::new(tokens);
            let program = parser.parse_program().map_err(|e| {
                format!("Parse error in {} at line {}, col {}: {}", file_name, e.line, e.col, e.message)
            })?;
            stmts.extend(program.stmts);
        }
    }
    Ok(stmts)
}

/// Register all type definitions from the program
pub fn register_types(program: &Program) -> TypeRegistry {
    let mut registry = TypeRegistry::new();
    for stmt in &program.stmts {
        let _ = registry.register(stmt);
    }
    registry
}

/// Load, resolve imports, register types, and type-check a program.
pub fn load_program(path: &PathBuf, explain: bool) -> Result<(Program, TypeRegistry), Vec<CompilerError>> {
    let source = fs::read_to_string(path).map_err(|e| {
        vec![CompilerError::new(format!("Cannot read '{}': {}", path.display(), e))]
    })?;

    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    let lexer_errors = lexer.take_errors();
    if !lexer_errors.is_empty() {
        return Err(lexer_errors);
    }

    let mut parser = crate::parser::Parser::new(tokens);
    let mut program = parser.parse_program().map_err(|e| {
        vec![CompilerError::new(format!("Parse error at line {}, col {}: {}", e.line, e.col, e.message))]
    })?;

    let builtins_types = builtin_types(&program);
    let stdlib = load_stdlib().map_err(|e| vec![CompilerError::new(e)])?;

    let mod_dir = path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
    let cwd_lib = std::env::current_dir()
        .map_err(|e| vec![CompilerError::new(format!("Cannot get current dir: {}", e))])?
        .join("lib");
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_default();
    let exe_lib = exe_dir.join("..").join("lib");
    let exe_stdlib = exe_dir.join("..").join("stdlib");
    let search_dirs = vec![mod_dir, cwd_lib, exe_lib, exe_stdlib];

    let imported = resolve_imports(&program, &search_dirs)
        .map_err(|e| vec![CompilerError::new(format!("Import error: {}", e))])?;

    let mut all_stmts: Vec<Stmt> = Vec::new();
    all_stmts.extend(builtins_types);
    all_stmts.extend(stdlib);
    all_stmts.extend(imported);
    all_stmts.append(&mut program.stmts);
    program.stmts = all_stmts;

    transform_module_access(&mut program);

    let registry = register_types(&program);

    let mut checker = TypeChecker::new(registry.clone());
    let errors = checker.check(&program);
    if !errors.is_empty() {
        if explain {
            let mut explained = Vec::new();
            for e in errors {
                let msg = e.to_string();
                let help = if msg.contains("Undefined variable") {
                    Some("Check that the variable is defined in the current scope. Variable names are case-sensitive.".to_string())
                } else if msg.contains("type") && msg.contains("expected") {
                    Some("Type annotations and inferred types must match. Consider adding an explicit type annotation.".to_string())
                } else if msg.contains("Undefined function") {
                    Some("Functions must be defined before they are called. Check for typos in the function name.".to_string())
                } else if msg.contains("not exhaustive") {
                    Some("When expressions must cover all possible cases. Add an 'else' arm or cover all enum variants.".to_string())
                } else { None };
                let mut new_e = CompilerError::new(msg);
                if let Some(h) = help { new_e = new_e.with_help(h); }
                explained.push(new_e);
            }
            return Err(explained);
        }
        return Err(errors);
    }

    Ok((program, registry))
}
