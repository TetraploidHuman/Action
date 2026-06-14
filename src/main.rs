mod ast;
mod codegen;
mod config;
mod error;
mod http_runtime;
mod lexer;
mod loader;
mod lsp;
mod parser;
mod repl;
mod runtime_json;
mod runtime_threading;
mod test_runner;
mod typecheck;

use ariadne::{Color, Label, Report, ReportKind, Source};
use clap::{Parser as ClapParser, Subcommand};
use config::ProjectConfig;
use error::CompilerError;
use inkwell::context::Context;
use std::fs;
use std::path::{Path, PathBuf};

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
        /// Enable memory profiling (print RC operation counts)
        #[arg(long)]
        profile: bool,
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
    /// Start an interactive REPL session
    Repl {
        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "0")]
        opt: u8,
        /// Enable memory profiling
        #[arg(long)]
        profile: bool,
        /// Target platform
        #[arg(long, default_value = "native")]
        target: String,
    },
    /// Discover and run @test functions in a source file
    Test {
        /// Source file path (.at or .atom)
        file: PathBuf,
        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "0")]
        opt: u8,
        /// Enable memory profiling
        #[arg(long)]
        profile: bool,
        /// Target platform
        #[arg(long, default_value = "native")]
        target: String,
    },
}

#[cfg(windows)]
extern "system" {
    fn SetConsoleOutputCP(code_page: u32) -> i32;
}

fn main() {
    #[cfg(windows)]
    unsafe {
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
            profile,
            target,
        } => {
            if let Err(e) = run_file(&file, opt, check, emit, explain, profile, &target) {
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
        Commands::Repl {
            opt,
            target,
            profile,
        } => {
            if let Err(e) = repl::run_repl(opt, profile, &target) {
                eprintln!("REPL error: {}", e);
            }
        }
        Commands::Lsp => {
            if let Err(e) = lsp::start_lsp() {
                eprintln!("LSP error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Test {
            file,
            opt,
            profile,
            target,
        } => {
            if let Err(e) = test_runner::run_test_file(&file, opt, profile, &target) {
                if let Ok(source) = fs::read_to_string(&file) {
                    report_error(&source, &file.to_string_lossy(), &e);
                } else {
                    eprintln!("Error: {}", e);
                }
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
fn report_error(source: &str, path: &str, error: &str) {
    fn parse_error_line(line: &str) -> Option<(usize, usize, String, Option<String>)> {
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

    let lines: Vec<&str> = error.lines().collect();
    let mut i = 0;
    let mut has_ariadne_output = false;

    while i < lines.len() {
        let line = lines[i];
        let mut help_text: Option<String> = None;

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
            if !has_ariadne_output {
                eprintln!("\x1b[1;31merror:\x1b[0m {}", line);
                if let Some(ref help) = help_text {
                    eprintln!("  \x1b[1;36mhelp:\x1b[0m {}", help);
                }
            }
        }
        i += 1;
    }

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

fn run_file(
    path: &PathBuf,
    opt: u8,
    check: bool,
    emit: Option<String>,
    explain: bool,
    profile: bool,
    target: &str,
) -> Result<(), String> {
    let config = ProjectConfig::find_and_load(path);
    let opt = config
        .as_ref()
        .map(|c| c.effective_opt_level(opt))
        .unwrap_or(opt);

    let (program, registry) = loader::load_program(path, explain).map_err(|errors| {
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
        if !is_exe && emit.is_none() {
            emit_output(&cg, path, "obj", target)?;
        }
    } else if !is_exe {
        if profile {
            let ir = cg.print_ir();
            let malloc_count = ir.matches("call ptr @action_malloc_rc").count();
            let inc_count = ir.matches("call void @action_rc_inc").count();
            let dec_count = ir.matches("call void @action_rc_dec").count();
            let total = malloc_count + inc_count + dec_count;
            if total > 0 {
                eprintln!(
                    "[profile] operations: {} (malloc_rc: {} rc_inc: {} rc_dec: {})",
                    total, malloc_count, inc_count, dec_count
                );
            }
        }
        let exit_code = cg.run_jit()?;
        if exit_code != 0 {
            std::process::exit(exit_code as i32);
        }
    } else {
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

    let (program, registry) = loader::load_program(path, false).map_err(|errors| {
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
    loader::load_program(path, explain).map(|_| ())
}
