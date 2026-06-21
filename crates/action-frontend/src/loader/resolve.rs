use crate::ast::*;
use action_span::Span;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

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
    for ext in &["atom", "ac", "at"] {
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
                let program = parser
                    .parse_program()
                    .map_err(|e| format!("Parse error in {}: {}", file_name, e))?;
                return Ok(program.stmts);
            }
        }
    }
    Err(format!(
        "Module '{}' not found (looked for {}.atom, {}.ac, or {}.at)",
        module_name, module_name, module_name, module_name
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

        let exported: Option<HashSet<String>> = module_stmts.iter().find_map(|s| {
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

        for m_stmt in &module_stmts {
            if let Stmt::Import {
                module: sub,
                items: sub_items,
                alias: sub_alias,
                ..
            } = m_stmt
            {
                resolve_module(
                    sub,
                    sub_items,
                    sub_alias,
                    search_dirs,
                    loaded,
                    visiting,
                    extra_stmts,
                )?;
            }
        }

        for m_stmt in &stmts_to_check {
            match m_stmt {
                Stmt::Fun {
                    name,
                    params,
                    return_type,
                    body,
                    is_single_expr,
                    type_params,
                    ..
                } => {
                    let imported_name = if items.is_some() {
                        name.clone()
                    } else {
                        format!("{}_{}", prefix, name)
                    };
                    let should_import = if let Some(ref its) = items {
                        its.contains(name)
                    } else if let Some(ref exported_set) = exported {
                        exported_set.contains(name)
                    } else {
                        true
                    };
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
                Stmt::Const {
                    name,
                    type_ann,
                    value,
                    ..
                } => {
                    let should_import = if let Some(ref its) = items {
                        its.contains(name)
                    } else if let Some(ref exported_set) = exported {
                        exported_set.contains(name)
                    } else {
                        true
                    };
                    if should_import {
                        let imported_name = if items.is_some() {
                            name.clone()
                        } else {
                            format!("{}_{}", prefix, name)
                        };
                        extra_stmts.push(Stmt::Const {
                            name: imported_name,
                            type_ann: type_ann.clone(),
                            value: value.clone(),
                            span: Span::default(),
                        });
                    }
                }
                Stmt::TypeAlias {
                    name,
                    type_params,
                    definition,
                    ..
                } => {
                    let should_import = if let Some(ref its) = items {
                        its.contains(name)
                    } else if let Some(ref exported_set) = exported {
                        exported_set.contains(name)
                    } else {
                        true
                    };
                    if should_import {
                        let imported_name = if items.is_some() {
                            name.clone()
                        } else {
                            format!("{}_{}", prefix, name)
                        };
                        extra_stmts.push(Stmt::TypeAlias {
                            name: imported_name,
                            type_params: type_params.clone(),
                            definition: definition.clone(),
                            span: Span::default(),
                        });
                    }
                }
                Stmt::Enum {
                    name,
                    type_params,
                    variants,
                    ..
                } => {
                    let should_import = if let Some(ref its) = items {
                        its.contains(name)
                    } else if let Some(ref exported_set) = exported {
                        exported_set.contains(name)
                    } else {
                        true
                    };
                    if should_import {
                        let imported_name = if items.is_some() {
                            name.clone()
                        } else {
                            format!("{}_{}", prefix, name)
                        };
                        extra_stmts.push(Stmt::Enum {
                            name: imported_name,
                            type_params: type_params.clone(),
                            variants: variants.clone(),
                            span: Span::default(),
                        });
                    }
                }
                Stmt::Extension {
                    type_name, methods, ..
                } => {
                    for method in methods {
                        if let Stmt::Fun {
                            name,
                            params,
                            return_type,
                            body,
                            is_single_expr,
                            type_params,
                            ..
                        } = method
                        {
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
                Stmt::External {
                    name,
                    params,
                    return_type,
                    ..
                } => {
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
        if let Stmt::Import {
            module,
            items,
            alias,
            ..
        } = stmt
        {
            resolve_module(
                module,
                items,
                alias,
                search_dirs,
                &mut loaded,
                &mut visiting,
                &mut extra_stmts,
            )?;
        }
    }

    Ok(extra_stmts)
}

/// Transform `math.add(...)` and `math.PI` into `math_add(...)` and `math_PI`
pub fn transform_module_access(program: &mut Program) {
    use std::collections::HashSet;
    let mut module_prefixes: HashSet<String> = HashSet::new();
    for stmt in &program.stmts {
        if let Stmt::Import {
            module,
            items,
            alias,
            ..
        } = stmt
        {
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
        // Handle module prefix transformation: Module.func -> Module_func
        // Check separately to avoid borrow conflicts with the mutable match below
        let replacement = match &expr.kind {
            ExprKind::FieldAccess(base, field) => match &base.kind {
                ExprKind::Ident(ident) if prefixes.contains(ident) => {
                    Some(ExprKind::Ident(format!("{}_{}", ident, field)).into())
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(new_expr) = replacement {
            *expr = new_expr;
            return;
        }

        match &mut expr.kind {
            ExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => {
                transform_expr(func, prefixes);
                for arg in args.iter_mut() {
                    transform_expr(arg, prefixes);
                }
                if let Some(ref mut lambda) = trailing_lambda {
                    transform_expr(lambda, prefixes);
                }
            }
            ExprKind::Binary(lhs, _, rhs) => {
                transform_expr(lhs, prefixes);
                transform_expr(rhs, prefixes);
            }
            ExprKind::Unary(_, operand) => {
                transform_expr(operand, prefixes);
            }
            ExprKind::FieldAccess(base, _) => {
                transform_expr(base, prefixes);
            }
            ExprKind::Index(base, idx) => {
                transform_expr(base, prefixes);
                transform_expr(idx, prefixes);
            }
            ExprKind::Range(start, end) => {
                transform_expr(start, prefixes);
                transform_expr(end, prefixes);
            }
            ExprKind::Tuple(elements) => {
                for (_, e) in elements.iter_mut() {
                    transform_expr(e, prefixes);
                }
            }
            ExprKind::StructLiteral(fields) => {
                for (_, e) in fields.iter_mut() {
                    transform_expr(e, prefixes);
                }
            }
            ExprKind::MapLiteral(entries) => {
                for (k, v) in entries.iter_mut() {
                    transform_expr(k, prefixes);
                    transform_expr(v, prefixes);
                }
            }
            ExprKind::SetLiteral(items) => {
                for item in items.iter_mut() {
                    transform_expr(item, prefixes);
                }
            }
            ExprKind::Block(stmts) => {
                for s in stmts.iter_mut() {
                    transform_stmt(s, prefixes);
                }
            }
            ExprKind::Lambda { body, .. } => {
                transform_expr(body, prefixes);
            }
            ExprKind::When(w) => match &mut w.kind {
                WhenKind::OneLine {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    transform_expr(condition, prefixes);
                    transform_expr(then_expr, prefixes);
                    transform_expr(else_expr, prefixes);
                }
                WhenKind::ValueMatch { value, arms } => {
                    transform_expr(value, prefixes);
                    for arm in arms.iter_mut() {
                        if let Some(ref mut g) = arm.guard {
                            transform_expr(g, prefixes);
                        }
                        transform_expr(&mut arm.body, prefixes);
                    }
                }
                WhenKind::ConditionChain { arms } => {
                    for arm in arms.iter_mut() {
                        if let Some(ref mut g) = arm.guard {
                            transform_expr(g, prefixes);
                        }
                        transform_expr(&mut arm.body, prefixes);
                    }
                }
            },
            ExprKind::For(fr) => match &mut fr.kind {
                ForKind::Iterate { iterable, body, .. }
                | ForKind::IterateWithIndex { iterable, body, .. } => {
                    transform_expr(iterable, prefixes);
                    transform_expr(body, prefixes);
                }
                ForKind::Condition { condition, body } => {
                    transform_expr(condition, prefixes);
                    transform_expr(body, prefixes);
                }
                ForKind::Infinite { body } => {
                    transform_expr(body, prefixes);
                }
                ForKind::NestedIterate { bindings, body, .. } => {
                    for (_, e) in bindings.iter_mut() {
                        transform_expr(e, prefixes);
                    }
                    transform_expr(body, prefixes);
                }
            },
            ExprKind::Assign { target, value, .. } => {
                transform_expr(target, prefixes);
                transform_expr(value, prefixes);
            }
            ExprKind::Unsafe(inner) => {
                transform_expr(inner, prefixes);
            }
            ExprKind::Copy(inner) => {
                transform_expr(inner, prefixes);
            }
            ExprKind::Null => {}
            ExprKind::OrBlock { nullable, fallback } => {
                transform_expr(nullable, prefixes);
                transform_expr(fallback, prefixes);
            }
            ExprKind::StringInterpolate(parts) => {
                for part in parts.iter_mut() {
                    if let StringPart::Expr(ref mut e) = part {
                        transform_expr(e, prefixes);
                    }
                }
            }
            _ => {}
        }
    }

    fn transform_stmt(stmt: &mut Stmt, prefixes: &HashSet<String>) {
        match stmt {
            Stmt::Fun { body, .. } => {
                transform_expr(body, prefixes);
            }
            Stmt::Let { value, .. } => {
                transform_expr(value, prefixes);
            }
            Stmt::Const { value, .. } => {
                transform_expr(value, prefixes);
            }
            Stmt::Expr { expr, .. } => {
                transform_expr(expr, prefixes);
            }
            Stmt::Return { value, .. } => {
                if let Some(ref mut e) = value {
                    transform_expr(e, prefixes);
                }
            }
            Stmt::Module { body, .. } => {
                for s in body.iter_mut() {
                    transform_stmt(s, prefixes);
                }
            }
            Stmt::Export { stmt, .. } => {
                transform_stmt(stmt, prefixes);
            }
            Stmt::Destructure { value, .. } => {
                transform_expr(value, prefixes);
            }
            Stmt::Extension { methods, .. } => {
                for m in methods.iter_mut() {
                    transform_stmt(m, prefixes);
                }
            }
            Stmt::External { .. }
            | Stmt::ExternalType { .. }
            | Stmt::Enum { .. }
            | Stmt::TypeAlias { .. }
            | Stmt::Import { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. } => {}
        }
    }

    for stmt in &mut program.stmts {
        transform_stmt(stmt, &module_prefixes);
    }
}
