//! Shared frontend compilation session for loader, LSP, and REPL.
//!
//! Encapsulates stdlib injection, import resolution, and type-checking so
//! strict (fail-fast) and recover (LSP) paths share the same pipeline.
//!
//! ## Unified entry points
//!
//! - [`FrontendSession::compile_source_strict`] — CLI / loader (fail-fast parse)
//! - [`FrontendSession::compile_source_recover`] — LSP / REPL buffers (recovering parse)
//! - [`FrontendSession::for_repl`] — REPL session with stdlib type context

use crate::ast::*;
use crate::checked::CheckedProgram;
use crate::error::CompilerError;
use crate::loader::{
    build_type_registry, builtin_types, load_path_dependencies, load_stdlib, resolve_imports,
    transform_module_access,
};
use crate::parser::{ParseError, Parser};
use crate::registry::TypeRegistry;
use crate::typecheck::TypeChecker;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Holds stdlib context and module search paths for repeated compilations.
#[derive(Clone)]
pub struct FrontendSession {
    pub stdlib_stmts: Vec<Stmt>,
    pub search_dirs: Vec<PathBuf>,
    pub base_registry: TypeRegistry,
    pub base_type_env: HashMap<String, Type>,
}

impl FrontendSession {
    /// Build search directories for a source file (same strategy as deprecated `load_program`).
    pub fn search_dirs_for_file(path: &Path) -> Result<Vec<PathBuf>, String> {
        let mod_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let cwd_lib = std::env::current_dir()
            .map_err(|e| format!("Cannot get current dir: {}", e))?
            .join("lib");
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_default();
        let exe_lib = exe_dir.join("..").join("lib");
        let exe_stdlib = exe_dir.join("..").join("stdlib");
        Ok(vec![mod_dir, cwd_lib, exe_lib, exe_stdlib])
    }

    /// Build search dirs from workspace roots (LSP).
    pub fn search_dirs_for_workspace(extra: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = extra.into_iter().collect();
        if let Ok(cwd) = std::env::current_dir() {
            dirs.push(cwd.join("lib"));
        }
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                dirs.push(exe_dir.join("..").join("lib"));
                dirs.push(exe_dir.join("..").join("stdlib"));
            }
        }
        dirs
    }

    /// Session for compiling a file on disk (loader / CLI).
    pub fn for_source_file(path: &Path) -> Result<Self, String> {
        let search_dirs = Self::search_dirs_for_file(path)?;
        let stdlib_stmts = load_stdlib()?;
        Ok(Self {
            stdlib_stmts,
            search_dirs,
            base_registry: TypeRegistry::new(),
            base_type_env: HashMap::new(),
        })
    }

    /// Session pre-loaded with stdlib type context (LSP).
    pub fn with_context(
        search_dirs: Vec<PathBuf>,
        base_registry: TypeRegistry,
        base_type_env: HashMap<String, Type>,
    ) -> Result<Self, String> {
        Ok(Self {
            stdlib_stmts: load_stdlib()?,
            search_dirs,
            base_registry,
            base_type_env,
        })
    }

    fn assemble_program(
        &self,
        path: Option<&Path>,
        mut user_stmts: Vec<Stmt>,
    ) -> Result<Program, String> {
        let builtins_types = builtin_types(&Program {
            stmts: user_stmts.clone(),
        });
        let path_deps = if let Some(p) = path {
            load_path_dependencies(p)?
        } else {
            Vec::new()
        };

        let program_shell = Program {
            stmts: user_stmts.clone(),
        };
        let imported = resolve_imports(&program_shell, &self.search_dirs)?;

        let mut all_stmts: Vec<Stmt> = Vec::new();
        all_stmts.extend(builtins_types);
        all_stmts.extend(self.stdlib_stmts.clone());
        all_stmts.extend(path_deps);
        all_stmts.extend(imported);
        all_stmts.append(&mut user_stmts);

        let mut program = Program { stmts: all_stmts };
        transform_module_access(&mut program);
        Ok(program)
    }

    fn typecheck_with_checker(
        &self,
        program: &Program,
        explain: bool,
    ) -> Result<(TypeRegistry, TypeChecker), Vec<CompilerError>> {
        let registry = build_type_registry(program)?;

        let mut checker = TypeChecker::new(registry.clone());
        checker.seed_type_env(&self.base_type_env);
        let errors = checker.check(program);
        if !errors.is_empty() {
            if explain {
                return Err(errors
                    .into_iter()
                    .map(crate::error::enrich_with_explain)
                    .collect());
            }
            return Err(errors);
        }
        Ok((registry, checker))
    }

    fn typecheck(
        &self,
        program: &Program,
        explain: bool,
    ) -> Result<TypeRegistry, Vec<CompilerError>> {
        self.typecheck_with_checker(program, explain)
            .map(|(registry, _)| registry)
    }

    /// REPL session with stdlib type context (mirrors LSP startup).
    pub fn for_repl() -> Result<Self, String> {
        let search_dirs = Self::search_dirs_for_workspace([]);
        let (base_registry, base_type_env) = Self::load_stdlib_context(&search_dirs);
        Self::with_context(search_dirs, base_registry, base_type_env)
    }

    /// Unified strict compile: lex → parse (fail-fast) → assemble → typecheck → HIR.
    pub fn compile_source_strict(
        &self,
        source: &str,
        path: &Path,
        explain: bool,
    ) -> Result<CheckedProgram, Vec<CompilerError>> {
        self.check_source(source, path, explain)
    }

    /// Unified recovering compile for editor / REPL buffers.
    pub fn compile_source_recover(&self, source: &str, path: Option<&Path>) -> RecoverResult {
        self.compile_recover_for_path(source, path)
    }

    /// Strict compile with HIR lowering (read source from `path`).
    pub fn check_file(
        &self,
        path: &Path,
        explain: bool,
    ) -> Result<CheckedProgram, Vec<CompilerError>> {
        let source = std::fs::read_to_string(path).map_err(|e| {
            vec![CompilerError::new(format!(
                "Cannot read '{}': {}",
                path.display(),
                e
            ))]
        })?;
        self.check_source(&source, path, explain)
    }

    /// Strict compile with HIR lowering.
    #[deprecated(since = "0.5.5", note = "use `check_file`")]
    pub fn compile_checked(
        &self,
        path: &Path,
        explain: bool,
    ) -> Result<CheckedProgram, Vec<CompilerError>> {
        self.check_file(path, explain)
    }

    pub fn check_source(
        &self,
        source: &str,
        path: &Path,
        explain: bool,
    ) -> Result<CheckedProgram, Vec<CompilerError>> {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let lexer_errors = lexer.take_errors();
        if !lexer_errors.is_empty() {
            return Err(lexer_errors);
        }

        let mut parser = Parser::new(tokens);
        let program_shell = parser
            .parse_program()
            .map_err(|e| vec![e.to_compiler_error()])?;

        let program = self
            .assemble_program(Some(path), program_shell.stmts)
            .map_err(|e| vec![CompilerError::new(e)])?;

        let (registry, checker) = self.typecheck_with_checker(&program, explain)?;
        Ok(CheckedProgram::new(program, registry, &checker))
    }

    /// Strict compile from a source file (fail-fast parse).
    pub fn compile_file(
        &self,
        path: &Path,
        explain: bool,
    ) -> Result<(Program, TypeRegistry), Vec<CompilerError>> {
        let source = std::fs::read_to_string(path).map_err(|e| {
            vec![CompilerError::new(format!(
                "Cannot read '{}': {}",
                path.display(),
                e
            ))]
        })?;
        self.compile_strict_source(&source, path, explain)
    }

    /// Strict compile from source text.
    pub fn compile_strict_source(
        &self,
        source: &str,
        path: &Path,
        explain: bool,
    ) -> Result<(Program, TypeRegistry), Vec<CompilerError>> {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let lexer_errors = lexer.take_errors();
        if !lexer_errors.is_empty() {
            return Err(lexer_errors);
        }

        let mut parser = Parser::new(tokens);
        let program_shell = parser
            .parse_program()
            .map_err(|e| vec![e.to_compiler_error()])?;

        let program = self
            .assemble_program(Some(path), program_shell.stmts)
            .map_err(|e| vec![CompilerError::new(e)])?;

        let registry = self.typecheck(&program, explain)?;
        Ok((program, registry))
    }

    /// Recovering parse + typecheck with full import/stdlib assembly.
    pub fn compile_recover(&self, source: &str) -> RecoverResult {
        self.compile_recover_for_path(source, None)
    }

    /// Recovering parse + typecheck for LSP / REPL buffers with optional file path.
    pub fn compile_recover_for_path(&self, source: &str, path: Option<&Path>) -> RecoverResult {
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let lexer_errors = lexer.take_errors();

        let mut parser = Parser::new(tokens);
        let (user_stmts, parse_errors) = parser.parse_program_recover();

        if user_stmts.is_empty() {
            return RecoverResult {
                stmts: Vec::new(),
                registry: self.base_registry.clone(),
                type_env: self.base_type_env.clone(),
                type_errors: lexer_errors,
                parse_errors,
                hir: None,
            };
        }

        let program = match self.assemble_program(path, user_stmts.clone()) {
            Ok(p) => p,
            Err(e) => {
                return RecoverResult {
                    stmts: user_stmts,
                    registry: self.base_registry.clone(),
                    type_env: self.base_type_env.clone(),
                    type_errors: vec![CompilerError::new(e)],
                    parse_errors,
                    hir: None,
                };
            }
        };

        let registry = match build_type_registry(&program) {
            Ok(r) => r,
            Err(mut reg_errors) => {
                reg_errors.extend(lexer_errors);
                return RecoverResult {
                    stmts: user_stmts,
                    registry: self.base_registry.clone(),
                    type_env: self.base_type_env.clone(),
                    type_errors: reg_errors,
                    parse_errors,
                    hir: None,
                };
            }
        };
        let mut checker = TypeChecker::new(registry.clone());
        checker.seed_type_env(&self.base_type_env);
        let mut type_errors = lexer_errors;
        type_errors.extend(checker.check(&program));
        let type_env = checker.type_env().clone();
        let hir = if type_errors.is_empty() {
            Some(crate::hir::lower_program(&program, &checker))
        } else {
            None
        };

        RecoverResult {
            stmts: user_stmts,
            registry,
            type_env,
            type_errors,
            parse_errors,
            hir,
        }
    }

    /// Type-check assembled user statements (REPL / programmatic compile).
    pub fn check_from_stmts(
        &self,
        user_stmts: Vec<Stmt>,
        path: &Path,
        explain: bool,
    ) -> Result<CheckedProgram, Vec<CompilerError>> {
        let program = self
            .assemble_program(Some(path), user_stmts)
            .map_err(|e| vec![CompilerError::new(e)])?;
        let (registry, checker) = self.typecheck_with_checker(&program, explain)?;
        Ok(CheckedProgram::new(program, registry, &checker))
    }

    /// Type-check assembled user statements (REPL / programmatic compile).
    #[deprecated(since = "0.5.5", note = "use `check_from_stmts`")]
    pub fn compile_checked_from_stmts(
        &self,
        user_stmts: Vec<Stmt>,
        path: &Path,
        explain: bool,
    ) -> Result<CheckedProgram, Vec<CompilerError>> {
        self.check_from_stmts(user_stmts, path, explain)
    }

    /// Strict compile with HIR lowering from source text.
    #[deprecated(since = "0.5.5", note = "use `check_source`")]
    pub fn compile_checked_source(
        &self,
        source: &str,
        path: &Path,
        explain: bool,
    ) -> Result<CheckedProgram, Vec<CompilerError>> {
        self.check_source(source, path, explain)
    }

    /// Recovering parse + typecheck for a single buffer (LSP legacy).
    ///
    /// Prefer [`compile_recover_for_path`] with the document path for import/path_deps parity.
    pub fn compile_recover_buffer(&self, source: &str) -> RecoverResult {
        self.compile_recover_for_path(source, None)
    }

    /// Load stdlib `.ac` files into registry + type_env (for LSP / REPL startup).
    pub fn load_stdlib_context(search_dirs: &[PathBuf]) -> (TypeRegistry, HashMap<String, Type>) {
        match Self::try_load_stdlib_context(search_dirs) {
            Ok(ctx) => ctx,
            Err(e) => {
                eprintln!("warning: stdlib context load failed: {e}");
                (TypeRegistry::new(), HashMap::new())
            }
        }
    }

    fn try_load_stdlib_context(
        search_dirs: &[PathBuf],
    ) -> Result<(TypeRegistry, HashMap<String, Type>), String> {
        let mut registry = TypeRegistry::new();
        let mut type_env: HashMap<String, Type> = HashMap::new();

        for filename in &["math.ac", "json.ac"] {
            let path = search_dirs
                .iter()
                .flat_map(|d| [d.join("lib").join(filename), d.join(filename)])
                .find(|p| p.is_file());
            let Some(path) = path else {
                continue;
            };
            let stmts = crate::loader::parse_ac_file(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?;

            for stmt in &stmts {
                registry
                    .register(stmt)
                    .map_err(|e| format!("register {}: {e}", path.display()))?;
            }

            let program = Program { stmts };
            let mut checker = TypeChecker::new(registry.clone());
            checker.seed_type_env(&type_env);
            let errors = checker.check(&program);
            if !errors.is_empty() {
                let msg = errors
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(format!("typecheck {}: {msg}", path.display()));
            }
            for (k, v) in checker.type_env() {
                type_env.entry(k.clone()).or_insert_with(|| v.clone());
            }
            registry = checker.registry_ref().clone();
        }

        Ok((registry, type_env))
    }
}

/// Result of `FrontendSession::compile_recover`.
pub struct RecoverResult {
    pub stmts: Vec<Stmt>,
    pub registry: TypeRegistry,
    pub type_env: HashMap<String, Type>,
    pub type_errors: Vec<CompilerError>,
    pub parse_errors: Vec<ParseError>,
    /// Lowered HIR when type-check succeeded (LSP hover / codegen alignment).
    pub hir: Option<crate::hir::HirModule>,
}

#[cfg(test)]
mod session_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn load_stdlib_context_loads_math_and_json() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let (_registry, type_env) = FrontendSession::load_stdlib_context(&[root]);
        assert!(
            type_env.contains_key("add"),
            "math.ac should register add in type_env"
        );
        assert!(
            type_env.contains_key("jsonParse"),
            "json.ac should register jsonParse in type_env"
        );
    }
}
