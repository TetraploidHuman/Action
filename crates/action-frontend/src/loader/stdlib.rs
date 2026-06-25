use crate::ast::*;
use crate::config::ProjectConfig;
use action_span::Span;
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
    let has_http_response = program.stmts.iter().any(|s| {
        matches!(s, Stmt::TypeAlias { name, .. } if name == "HttpResponse")
    });
    if !has_http_response {
        builtins.push(Stmt::TypeAlias {
            name: "HttpResponse".into(),
            type_params: vec![],
            definition: Type::Struct(vec![
                ("status".into(), Type::Named("Int".into())),
                ("body".into(), Type::Named("String".into())),
            ]),
            span: Span::default(),
        });
    }
    builtins
}

/// Parse a single `.ac` / `.atom` source file into statements.
fn parse_source_file(path: &Path) -> Result<Vec<Stmt>, String> {
    let source =
        fs::read_to_string(path).map_err(|e| format!("Cannot read '{}': {}", path.display(), e))?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<source>");
    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    let lexer_errors = lexer.take_errors();
    if !lexer_errors.is_empty() {
        return Err(format!("Lexer error in {}: {}", file_name, lexer_errors[0]));
    }
    let mut parser = crate::parser::Parser::new(tokens);
    let program = parser
        .parse_program()
        .map_err(|e| format!("Parse error in {}: {}", file_name, e))?;
    Ok(program.stmts)
}

/// Load all `.ac` files from a dependency path (file, directory, or `name.ac` sibling).
fn load_ac_sources(dep_path: &Path) -> Result<Vec<Stmt>, String> {
    if dep_path.is_file() {
        return parse_source_file(dep_path);
    }
    if dep_path.is_dir() {
        let mut files: Vec<PathBuf> = fs::read_dir(dep_path)
            .map_err(|e| format!("Cannot read directory '{}': {}", dep_path.display(), e))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|ext| ext == "ac"))
            .collect();
        files.sort();
        if files.is_empty() {
            return Err(format!(
                "Dependency directory '{}' contains no .ac files",
                dep_path.display()
            ));
        }
        let mut stmts = Vec::new();
        for file in files {
            stmts.extend(parse_source_file(&file)?);
        }
        return Ok(stmts);
    }
    let at_file = dep_path.with_extension("ac");
    if at_file.is_file() {
        return parse_source_file(&at_file);
    }
    Err(format!("Dependency path not found: {}", dep_path.display()))
}

/// Load local path dependencies declared in atom.toml (before the main program).
pub fn load_path_dependencies(source_path: &Path) -> Result<Vec<Stmt>, String> {
    let Some((project_root, config)) = ProjectConfig::find_and_load_with_root(source_path) else {
        return Ok(Vec::new());
    };

    let mut stmts = Vec::new();
    for dep_path in config.path_dependencies() {
        let resolved = if dep_path.is_absolute() {
            dep_path.clone()
        } else {
            project_root.join(dep_path)
        };
        stmts.extend(load_ac_sources(&resolved)?);
    }
    Ok(stmts)
}

/// Load stdlib source files as modules (math, json) so top-level names do not clash with user code.
pub fn load_stdlib() -> Result<Vec<Stmt>, String> {
    let mut stmts = Vec::new();
    let exe_lib = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("..").join("lib")))
        .unwrap_or_default();
    let cwd_lib = std::env::current_dir()
        .map_err(|e| format!("Cannot get current dir: {}", e))?
        .join("lib");
    for file_name in &["math.ac", "json.ac"] as &[&str] {
        let path = [&exe_lib, &cwd_lib]
            .iter()
            .map(|d| d.join(file_name))
            .find(|p| p.exists());
        if let Some(path) = path {
            let body = parse_source_file(&path)?;
            let module_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("stdlib")
                .to_string();
            let exports: Vec<ExportItem> = body
                .iter()
                .filter_map(|s| match s {
                    Stmt::Fun { name, .. } => Some(ExportItem::Function(name.clone())),
                    Stmt::Const { name, .. } => Some(ExportItem::Constant(name.clone())),
                    Stmt::TypeAlias { name, .. } | Stmt::ExternalType { name, .. } => {
                        Some(ExportItem::Type(name.clone()))
                    }
                    _ => None,
                })
                .collect();
            let span = body.first().map(|s| s.span()).unwrap_or(Span {
                start: 0,
                end: 0,
                line: 1,
                col: 1,
            });
            stmts.push(Stmt::Module {
                name: module_name,
                exports,
                body,
                span,
            });
        }
    }
    Ok(stmts)
}
