mod ast;
mod codegen;
mod config;
mod error;
mod http_runtime;
mod lexer;
mod lsp;
mod parser;
mod runtime_json;
mod runtime_threading;
mod typecheck;

use ariadne::{Color, Label, Report, ReportKind, Source};
use ast::*;
use clap::{Parser as ClapParser, Subcommand};
use config::ProjectConfig;
use error::CompilerError;
use inkwell::context::Context;
use lexer::Span;
use std::fs;
use std::path::{Path, PathBuf};
use typecheck::{TypeChecker, TypeRegistry};

#[derive(ClapParser)]
#[command(name = "action", about = "Action Language Compiler", version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile and run an Atomic source file
    Run {
        /// Source file path (.at or .atom)
        file: PathBuf,
        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "0")]
        opt: u8,
        /// Type-check only (don't run)
        #[arg(long)]
        check: bool,
        /// Emit format: ir, bc, asm, obj (writes to file; ir prints to stdout)
        #[arg(long, value_name = "FORMAT")]
        emit: Option<String>,
        /// Enable verbose error messages with suggestions
        #[arg(long)]
        explain: bool,
        /// Target platform: native, linux-x64, linux-arm64, windows-x64, wasm
        #[arg(long, default_value = "native")]
        target: String,
    },
    /// Compile an Atomic source file
    Build {
        /// Source file path (.at or .atom)
        file: PathBuf,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "0")]
        opt: u8,
        /// Emit format: ir, bc, asm, obj
        #[arg(long, value_name = "FORMAT")]
        emit: Option<String>,
        /// Target platform: native, linux-x64, linux-arm64, windows-x64, wasm
        #[arg(long, default_value = "native")]
        target: String,
    },
    /// Type-check an Atomic source file without compilation
    Check {
        /// Source file path (.at or .atom)
        file: PathBuf,
        /// Enable verbose error messages
        #[arg(long)]
        explain: bool,
    },
    /// Start the Action Language LSP server
    Lsp,
}

#[cfg(windows)]
extern "system" {
    fn SetConsoleOutputCP(code_page: u32) -> i32;
}

fn main() {
    #[cfg(windows)]
    unsafe {
        // Set console output to UTF-8 so AI responses display correctly
        // (Windows defaults to the system code page, e.g. GBK, which garbles UTF-8)
        SetConsoleOutputCP(65001);
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            opt,
            check,
            emit,
            explain,
            target,
        } => {
            if let Err(e) = run_file(&file, opt, check, emit, explain, &target) {
                if let Ok(source) = fs::read_to_string(&file) {
                    report_error(&source, &file.to_string_lossy(), &e);
                } else {
                    eprintln!("Error: {}", e);
                }
                std::process::exit(1);
            }
        }
        Commands::Build {
            file,
            output,
            opt,
            emit,
            target,
        } => {
            if let Err(e) = build_file(&file, output, opt, emit, &target) {
                if let Ok(source) = fs::read_to_string(&file) {
                    report_error(&source, &file.to_string_lossy(), &e);
                } else {
                    eprintln!("Error: {}", e);
                }
                std::process::exit(1);
            }
        }
        Commands::Check { file, explain } => match check_file(&file, explain) {
            Ok(()) => {
                println!("Type checking passed. No errors found.");
            }
            Err(errors) => {
                if let Ok(source) = fs::read_to_string(&file) {
                    let msg = errors
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<_>>()
                        .join("\n");
                    report_error(&source, &file.to_string_lossy(), &msg);
                } else {
                    for e in &errors {
                        eprintln!("Error: {}", e);
                    }
                }
                std::process::exit(1);
            }
        },
        Commands::Lsp => {
            if let Err(e) = lsp::start_lsp() {
                eprintln!("LSP error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Convert line (1-indexed) and col (1-indexed) to byte offset in source
fn line_col_to_offset(source: &str, line: usize, col: usize) -> usize {
    let mut cur_line = 1;
    let mut cur_col = 1;
    for (i, ch) in source.char_indices() {
        if cur_line == line && cur_col == col {
            return i;
        }
        if ch == '\n' {
            cur_line += 1;
            cur_col = 1;
        } else {
            cur_col += 1;
        }
    }
    source.len()
}

/// Report errors with ariadne for pretty source-context output.
/// Handles both single errors and multi-line typecheck/codegen errors.
/// Format: "Error at line X, col Y: message" or "Error: message" with optional "\n  help: ..."
fn report_error(source: &str, path: &str, error: &str) {
    // Parse function to extract (line, col, message, help) from an error line
    fn parse_error_line(line: &str) -> Option<(usize, usize, String, Option<String>)> {
        // Try "Error at line X, col Y: message" format
        if let Some(rest) = line.strip_prefix("Error at line ") {
            let parts: Vec<&str> = rest.splitn(2, ", col ").collect();
            if parts.len() == 2 {
                let line_num: usize = parts[0].parse().ok()?;
                let col_parts: Vec<&str> = parts[1].splitn(2, ": ").collect();
                let col: usize = col_parts[0].parse().ok()?;
                let msg = col_parts.get(1).unwrap_or(&"error").to_string();
                return Some((line_num, col, msg, None));
            }
        }
        // Try "Parse error at line X, col Y: message" format
        if let Some(rest) = line.strip_prefix("Parse error at line ") {
            let parts: Vec<&str> = rest.splitn(2, ", col ").collect();
            if parts.len() == 2 {
                let line_num: usize = parts[0].parse().ok()?;
                let col_parts: Vec<&str> = parts[1].splitn(2, ": ").collect();
                let col: usize = col_parts[0].parse().ok()?;
                let msg = col_parts.get(1).unwrap_or(&"parse error").to_string();
                return Some((line_num, col, msg, None));
            }
        }
        None
    }

    // Split into individual error messages (separated by newlines)
    let lines: Vec<&str> = error.lines().collect();
    let mut i = 0;
    let mut has_ariadne_output = false;

    while i < lines.len() {
        let line = lines[i];
        let mut help_text: Option<String> = None;

        // Check if the next line is a help line
        if i + 1 < lines.len() && lines[i + 1].trim().starts_with("help: ") {
            help_text = Some(
                lines[i + 1]
                    .trim()
                    .strip_prefix("help: ")
                    .unwrap_or("")
                    .to_string(),
            );
            i += 1;
        }

        if let Some((line_num, col, msg, _)) = parse_error_line(line) {
            let offset = line_col_to_offset(source, line_num, col);
            let mut report = Report::build(ReportKind::Error, path, offset)
                .with_message(&msg)
                .with_label(
                    Label::new((path, offset..offset + 1))
                        .with_message("here")
                        .with_color(Color::Red),
                );

            if let Some(ref help) = help_text {
                report = report.with_help(help.clone());
            }

            report
                .finish()
                .eprint((path, Source::from(source)))
                .unwrap_or_else(|_| eprintln!("Error: {}", line));
            has_ariadne_output = true;
        } else {
            // For errors without location info, show with simple formatting
            if !has_ariadne_output {
                eprintln!("\x1b[1;31merror:\x1b[0m {}", line);
                if let Some(ref help) = help_text {
                    eprintln!("  \x1b[1;36mhelp:\x1b[0m {}", help);
                }
            }
        }
        i += 1;
    }

    // If no ariadne output and no formatted output, fallback
    if !has_ariadne_output
        && error
            .lines()
            .all(|l| !l.starts_with("Error at line") && !l.starts_with("Parse error at line"))
    {
        for line in error.lines() {
            if !line.trim().starts_with("help: ") {
                eprintln!("\x1b[1;31merror:\x1b[0m {}", line);
            }
        }
    }
}

/// Register builtin struct types (Date, DateTime, Random)
fn builtin_types(program: &Program) -> Vec<Stmt> {
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
            span: lexer::Span::default(),
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
            span: lexer::Span::default(),
        });
    }
    if !has_random {
        builtins.push(Stmt::TypeAlias {
            name: "Random".into(),
            type_params: vec![],
            definition: Type::Struct(vec![("seed".into(), Type::Named("Int".into()))]),
            span: lexer::Span::default(),
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
            // Canonicalize to ensure the resolved path stays within the search directory
            if let Ok(canon) = path.canonicalize() {
                if let Ok(dir_canon) = dir.canonicalize() {
                    if !canon.starts_with(&dir_canon) {
                        continue; // path traversal attempt, skip this directory
                    }
                }
            }
            if path.exists() {
                let source = fs::read_to_string(&path)
                    .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;
                let mut lexer = lexer::Lexer::new(&source);
                let tokens = lexer.tokenize();
                let lexer_errors = lexer.take_errors();
                if !lexer_errors.is_empty() {
                    return Err(format!(
                        "Lexer errors in {}:\n{}",
                        file_name,
                        lexer_errors.join("\n")
                    ));
                }
                let mut parser = parser::Parser::new(tokens);
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
fn resolve_imports(program: &Program, search_dirs: &[PathBuf]) -> Result<Vec<Stmt>, String> {
    let mut extra_stmts = Vec::new();
    let mut loaded: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Track modules currently being resolved for cycle detection
    let mut visiting: std::collections::HashSet<String> = std::collections::HashSet::new();

    fn resolve_module(
        module: &str,
        items: &Option<Vec<String>>,
        alias: &Option<String>,
        search_dirs: &[PathBuf],
        loaded: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
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

        // Check if the module has an explicit Module statement with export list
        let exported: Option<std::collections::HashSet<String>> =
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

        // Collect statements to import (from module body or top-level)
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
                Stmt::Fun {
                    name,
                    params,
                    return_type,
                    body,
                    is_single_expr,
                    type_params,
                    ..
                } => {
                    // Selective import: bare names. Wildcard: prefixed.
                    let imported_name = if items.is_some() {
                        name.clone()
                    } else {
                        format!("{}_{}", prefix, name)
                    };
                    // Filter by: import items list (if specified) AND module exports (if specified)
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
                                span: Span::default(),
                            });
                        }
                    }
                }
                // External declarations are infrastructure — always import as-is
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
/// for full-module imports (not selective imports).
fn transform_module_access(program: &mut Program) {
    use std::collections::HashSet;

    // Collect module prefixes from Import statements
    let mut module_prefixes: HashSet<String> = HashSet::new();
    for stmt in &program.stmts {
        if let Stmt::Import {
            module,
            items,
            alias,
            ..
        } = stmt
        {
            // Only transform for full-module imports (no items, no alias)
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
        // Check if this is a FieldAccess on a module name
        if let Expr::FieldAccess(ref base, ref field) = expr {
            if let Expr::Ident(ref ident) = **base {
                if prefixes.contains(ident) {
                    let new_name = format!("{}_{}", ident, field);
                    *expr = Expr::Ident(new_name);
                    return; // Already transformed
                }
            }
        }

        // Recurse into sub-expressions
        match expr {
            Expr::Call {
                ref mut func,
                ref mut args,
                ref mut trailing_lambda,
            } => {
                transform_expr(func, prefixes);
                for arg in args.iter_mut() {
                    transform_expr(arg, prefixes);
                }
                if let Some(ref mut lambda) = trailing_lambda {
                    transform_expr(lambda, prefixes);
                }
            }
            Expr::Binary(ref mut lhs, _, ref mut rhs) => {
                transform_expr(lhs, prefixes);
                transform_expr(rhs, prefixes);
            }
            Expr::Unary(_, ref mut operand) => {
                transform_expr(operand, prefixes);
            }
            Expr::FieldAccess(ref mut base, _) => {
                transform_expr(base, prefixes);
            }
            Expr::Index(ref mut base, ref mut idx) => {
                transform_expr(base, prefixes);
                transform_expr(idx, prefixes);
            }
            Expr::Range(ref mut start, ref mut end) => {
                transform_expr(start, prefixes);
                transform_expr(end, prefixes);
            }
            Expr::Tuple(ref mut elements) => {
                for (_, e) in elements.iter_mut() {
                    transform_expr(e, prefixes);
                }
            }
            Expr::StructLiteral(ref mut fields) => {
                for (_, e) in fields.iter_mut() {
                    transform_expr(e, prefixes);
                }
            }
            Expr::MapLiteral(ref mut entries) => {
                for (k, v) in entries.iter_mut() {
                    transform_expr(k, prefixes);
                    transform_expr(v, prefixes);
                }
            }
            Expr::SetLiteral(ref mut items) => {
                for item in items.iter_mut() {
                    transform_expr(item, prefixes);
                }
            }
            Expr::Block(ref mut stmts) => {
                for s in stmts.iter_mut() {
                    transform_stmt(s, prefixes);
                }
            }
            Expr::Lambda { ref mut body, .. } => {
                transform_expr(body, prefixes);
            }
            Expr::When(ref mut w) => {
                // w is Box<When>
                match &mut w.kind {
                    WhenKind::OneLine {
                        ref mut condition,
                        ref mut then_expr,
                        ref mut else_expr,
                    } => {
                        transform_expr(condition, prefixes);
                        transform_expr(then_expr, prefixes);
                        transform_expr(else_expr, prefixes);
                    }
                    WhenKind::ValueMatch {
                        ref mut value,
                        ref mut arms,
                    } => {
                        transform_expr(value, prefixes);
                        for arm in arms.iter_mut() {
                            if let Some(ref mut g) = arm.guard {
                                transform_expr(g, prefixes);
                            }
                            transform_expr(&mut arm.body, prefixes);
                        }
                    }
                    WhenKind::ConditionChain { ref mut arms } => {
                        for arm in arms.iter_mut() {
                            if let Some(ref mut g) = arm.guard {
                                transform_expr(g, prefixes);
                            }
                            transform_expr(&mut arm.body, prefixes);
                        }
                    }
                }
            }
            Expr::For(ref mut fr) => {
                // fr is Box<For>
                match &mut fr.kind {
                    ForKind::Iterate {
                        ref mut iterable,
                        ref mut body,
                        ..
                    } => {
                        transform_expr(iterable, prefixes);
                        transform_expr(body, prefixes);
                    }
                    ForKind::IterateWithIndex {
                        ref mut iterable,
                        ref mut body,
                        ..
                    } => {
                        transform_expr(iterable, prefixes);
                        transform_expr(body, prefixes);
                    }
                    ForKind::Condition {
                        ref mut condition,
                        ref mut body,
                    } => {
                        transform_expr(condition, prefixes);
                        transform_expr(body, prefixes);
                    }
                    ForKind::Infinite { ref mut body } => {
                        transform_expr(body, prefixes);
                    }
                    ForKind::NestedIterate {
                        ref mut bindings,
                        ref mut body,
                        ..
                    } => {
                        for (_, e) in bindings.iter_mut() {
                            transform_expr(e, prefixes);
                        }
                        transform_expr(body, prefixes);
                    }
                }
            }
            Expr::Assign {
                ref mut target,
                ref mut value,
                ..
            } => {
                transform_expr(target, prefixes);
                transform_expr(value, prefixes);
            }
            Expr::Unsafe(ref mut inner) => {
                transform_expr(inner, prefixes);
            }
            Expr::Copy(ref mut inner) => {
                transform_expr(inner, prefixes);
            }
            Expr::Null => {}
            Expr::OrBlock {
                ref mut nullable,
                ref mut fallback,
            } => {
                transform_expr(nullable, prefixes);
                transform_expr(fallback, prefixes);
            }
            Expr::StringInterpolate(ref mut parts) => {
                for part in parts.iter_mut() {
                    if let StringPart::Expr(ref mut e) = part {
                        transform_expr(e, prefixes);
                    }
                }
            }
            _ => {} // Literal, Ident, Break, Continue, FunctionRef — no sub-expressions
        }
    }

    fn transform_stmt(stmt: &mut Stmt, prefixes: &HashSet<String>) {
        match stmt {
            Stmt::Fun { ref mut body, .. } => {
                transform_expr(body, prefixes);
            }
            Stmt::Let { ref mut value, .. } => {
                transform_expr(value, prefixes);
            }
            Stmt::Const { ref mut value, .. } => {
                transform_expr(value, prefixes);
            }
            Stmt::Expr { ref mut expr, .. } => {
                transform_expr(expr, prefixes);
            }
            Stmt::Return { ref mut value, .. } => {
                if let Some(ref mut e) = value {
                    transform_expr(e, prefixes);
                }
            }
            Stmt::Module { ref mut body, .. } => {
                for s in body.iter_mut() {
                    transform_stmt(s, prefixes);
                }
            }
            Stmt::Export { ref mut stmt, .. } => {
                transform_stmt(stmt, prefixes);
            }
            Stmt::Destructure { ref mut value, .. } => {
                transform_expr(value, prefixes);
            }
            Stmt::Extension {
                ref mut methods, ..
            } => {
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

/// Load stdlib source files
fn load_stdlib() -> Result<Vec<Stmt>, String> {
    let mut stmts = Vec::new();
    // Search lib/ relative to the executable (release packages) and cwd
    let exe_lib = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("..").join("lib")))
        .unwrap_or_default();
    let cwd_lib = std::env::current_dir()
        .map_err(|e| format!("Cannot get current dir: {}", e))?
        .join("lib");

    // Option/Result have been replaced by built-in nullable types.
    // No built-in enum stdlib files to load.
    let _ = &exe_lib; // silence unused warning
    for file_name in &[] as &[&str] {
        let path = [&exe_lib, &cwd_lib]
            .iter()
            .map(|d| d.join(file_name))
            .find(|p| p.exists());
        if let Some(path) = path {
            let source = fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;
            let mut lexer = lexer::Lexer::new(&source);
            let tokens = lexer.tokenize();
            let lexer_errors = lexer.take_errors();
            if !lexer_errors.is_empty() {
                return Err(format!("Lexer errors in {}:\n{}", file_name, lexer_errors.join("\n")));
            }
            let mut parser = parser::Parser::new(tokens);
            let program = parser.parse_program().map_err(|e| {
                format!(
                    "Parse error in {} at line {}, col {}: {}",
                    file_name, e.line, e.col, e.message
                )
            })?;
            stmts.extend(program.stmts);
        }
    }
    Ok(stmts)
}

/// Register all type definitions (enums, structs, type aliases) from the program
fn register_types(program: &Program) -> TypeRegistry {
    let mut registry = TypeRegistry::new();
    for stmt in &program.stmts {
        let _ = registry.register(stmt);
    }
    registry
}

/// Load, resolve imports, register types, and type-check a program.
/// Returns the fully-resolved Program and TypeRegistry on success.
fn load_program(
    path: &PathBuf,
    explain: bool,
) -> Result<(Program, TypeRegistry), Vec<CompilerError>> {
    let source = fs::read_to_string(path).map_err(|e| {
        vec![CompilerError::new(format!(
            "Cannot read '{}': {}",
            path.display(),
            e
        ))]
    })?;

    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    let lexer_errors = lexer.take_errors();
    if !lexer_errors.is_empty() {
        return Err(lexer_errors
            .into_iter()
            .map(|e| CompilerError::new(e))
            .collect());
    }

    let mut parser = parser::Parser::new(tokens);
    let mut program = parser.parse_program().map_err(|e| {
        vec![CompilerError::new(format!(
            "Parse error at line {}, col {}: {}",
            e.line, e.col, e.message
        ))]
    })?;

    let builtins_types = builtin_types(&program);
    let stdlib = load_stdlib().map_err(|e| vec![CompilerError::new(e)])?;

    let mod_dir = path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    let cwd_lib = std::env::current_dir()
        .map_err(|e| vec![CompilerError::new(format!("Cannot get current dir: {}", e))])?
        .join("lib");
    // Also search relative to the executable, so release packages work out-of-the-box
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
                } else {
                    None
                };
                let mut new_e = CompilerError::new(msg);
                if let Some(h) = help {
                    new_e = new_e.with_help(h);
                }
                explained.push(new_e);
            }
            return Err(explained);
        }
        return Err(errors);
    }

    Ok((program, registry))
}

fn run_file(
    path: &PathBuf,
    opt: u8,
    check: bool,
    emit: Option<String>,
    explain: bool,
    target: &str,
) -> Result<(), String> {
    let config = ProjectConfig::find_and_load(path);
    let opt = config
        .as_ref()
        .map(|c| c.effective_opt_level(opt))
        .unwrap_or(opt);

    let (program, registry) = load_program(path, explain).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    if check {
        println!(
            "Type checking passed for '{}'. No errors found.",
            path.display()
        );
        return Ok(());
    }

    let context = Context::create();
    let target_opt = if target == "native" {
        None
    } else {
        Some(target.to_string())
    };
    let mut cg = codegen::CodeGen::new(&context, "main", registry, target_opt);
    cg.set_opt_level(opt);
    cg.compile(&program)?;
    cg.verify()?;

    let is_cross = target != "native";
    let is_exe = emit.as_deref() == Some("exe");
    if let Some(ref fmt) = emit {
        emit_output(&cg, path, fmt, target)?;
    }

    if is_cross {
        // Cross-compilation: can't JIT or run non-native executables locally
        if !is_exe && emit.is_none() {
            // No --emit specified with cross target: default to --emit obj
            emit_output(&cg, path, "obj", target)?;
        }
    } else if !is_exe {
        let exit_code = cg.run_jit()?;
        if exit_code != 0 {
            std::process::exit(exit_code as i32);
        }
    } else {
        // Run the compiled executable
        let exe_path = path.with_extension("");
        let status = std::process::Command::new(&exe_path)
            .status()
            .map_err(|e| format!("Failed to run {}: {}", exe_path.display(), e))?;
        if !status.success() {
            return Err(format!("Process exited with status: {}", status));
        }
    }
    Ok(())
}

fn build_file(
    path: &PathBuf,
    output: Option<PathBuf>,
    opt: u8,
    emit: Option<String>,
    target: &str,
) -> Result<(), String> {
    let config = ProjectConfig::find_and_load(path);
    let opt = config
        .as_ref()
        .map(|c| c.effective_opt_level(opt))
        .unwrap_or(opt);

    let (program, registry) = load_program(path, false).map_err(|errors| {
        errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let context = Context::create();
    let target_opt = if target == "native" {
        None
    } else {
        Some(target.to_string())
    };
    let mut cg = codegen::CodeGen::new(&context, "main", registry, target_opt);
    cg.set_opt_level(opt);
    cg.compile(&program)?;
    cg.verify()?;

    if let Some(ref fmt) = emit {
        emit_output(&cg, path, fmt, target)?;
    } else {
        let ir = cg.print_ir();
        let out_path = output.unwrap_or_else(|| path.with_extension("ll"));
        fs::write(&out_path, ir)
            .map_err(|e| format!("Cannot write to '{}': {}", out_path.display(), e))?;
        println!("Compiled to: {}", out_path.display());
    }
    Ok(())
}

fn emit_output(
    cg: &codegen::CodeGen,
    src_path: &Path,
    fmt: &str,
    target: &str,
) -> Result<(), String> {
    match fmt {
        "ir" => {
            println!("=== LLVM IR ===");
            println!("{}", cg.print_ir());
        }
        "bc" => {
            let out = src_path.with_extension("bc");
            cg.emit_bitcode(&out)?;
            println!("Bitcode written to: {}", out.display());
        }
        "asm" | "s" => {
            let out = src_path.with_extension("s");
            cg.emit_assembly(&out)?;
            println!("Assembly written to: {}", out.display());
        }
        "obj" | "o" => {
            let out = src_path.with_extension("o");
            cg.emit_object(&out)?;
            println!("Object file written to: {}", out.display());
        }
        "exe" => {
            if target == "wasm" || target == "wasm32-unknown-unknown" {
                return Err("--emit exe is not supported for wasm target. Use --emit obj to produce a .wasm file.".to_string());
            }
            let obj_path = src_path.with_extension("o");
            cg.emit_object(&obj_path)?;
            let exe_path = if target == "windows-x64" || target == "x86_64-pc-windows-gnu" {
                src_path.with_extension("exe")
            } else {
                src_path.with_extension("")
            };
            let linker = match target {
                "windows-x64" | "x86_64-pc-windows-gnu" => "x86_64-w64-mingw32-gcc",
                "linux-arm64" | "aarch64-unknown-linux-gnu" => "aarch64-linux-gnu-gcc",
                _ => "cc",
            };
            let status = std::process::Command::new(linker)
                .arg("-o")
                .arg(&exe_path)
                .arg(&obj_path)
                .status()
                .map_err(|e| format!("Failed to invoke linker '{}': {}", linker, e))?;
            if !status.success() {
                return Err(format!("Linker '{}' failed", linker));
            }
            let _ = std::fs::remove_file(&obj_path);
            println!("Executable written to: {}", exe_path.display());
        }
        other => {
            return Err(format!(
                "Unknown emit format: {}. Supported: ir, bc, asm, obj, exe",
                other
            ))
        }
    }
    Ok(())
}

fn check_file(path: &PathBuf, explain: bool) -> Result<(), Vec<CompilerError>> {
    load_program(path, explain).map(|_| ())
}
