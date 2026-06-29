use action::driver;
use action::error;
use action::*;
use clap::{Parser as ClapParser, Subcommand};
use inkwell::context::Context;
use std::fs;
use std::path::{Path, PathBuf};

mod repl;
mod test_runner;

/// CLI run/build failure: structured type-check vs plain message (codegen / IO).
pub(crate) enum RunFailure {
    Check(Vec<action_frontend::error::CompilerError>),
    Message(String),
}

impl RunFailure {
    pub(crate) fn report(&self, path: &Path) {
        match self {
            RunFailure::Check(errors) => driver::report_check_errors(path, errors),
            RunFailure::Message(msg) => {
                if let Ok(source) = fs::read_to_string(path) {
                    error::report_error_message(&source, &path.to_string_lossy(), msg);
                } else {
                    eprintln!("Error: {}", msg);
                }
            }
        }
    }
}

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
        /// Source file path (.ac or .atom)
        file: PathBuf,
        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "0")]
        opt: u8,
        /// Type-check only (don't run)
        #[arg(long)]
        check: bool,
        /// Emit format: ir, bc, asm, obj, hir (hir writes JSON; ir prints to stdout)
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
        /// Source file path (.ac or .atom)
        file: PathBuf,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "0")]
        opt: u8,
        /// Emit format: ir, bc, asm, obj, hir
        #[arg(long, value_name = "FORMAT")]
        emit: Option<String>,
        /// Target platform: native, linux-x64, linux-arm64, windows-x64, wasm
        #[arg(long, default_value = "native")]
        target: String,
    },
    /// Type-check an Atomic source file without compilation
    Check {
        /// Source file path (.ac or .atom)
        file: PathBuf,
        /// Enable verbose error messages with suggestions
        #[arg(long)]
        explain: bool,
        /// Output format: human (default) or json
        #[arg(long, default_value = "human", value_name = "FORMAT")]
        format: String,
        /// Emit HIR JSON to `<file>.hir.json`
        #[arg(long, value_name = "FORMAT")]
        emit: Option<String>,
    },
    /// Format an Action source file (indentation)
    Fmt {
        /// Source file path (.ac or .atom)
        file: PathBuf,
        /// Check formatting without writing (exit 1 if changes needed)
        #[arg(long)]
        check: bool,
        /// Number of spaces per indent level
        #[arg(long, default_value_t = 4)]
        tab_size: u8,
        /// Indent with spaces instead of tabs
        #[arg(long, default_value_t = true)]
        insert_spaces: bool,
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
        /// Source file path (.ac or .atom)
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
                e.report(&file);
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
                e.report(&file);
                std::process::exit(1);
            }
        }
        Commands::Check {
            file,
            explain,
            format,
            emit,
        } => match driver::check_file(&file, explain) {
            Ok(checked) => {
                if let Some(ref fmt) = emit {
                    if fmt == "hir" {
                        if let Err(e) = driver::emit_hir(&checked, &file, false) {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    } else if fmt == "diagnostics" {
                        if let Err(e) = driver::emit_diagnostics_json(&[], &file, explain, false) {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    } else {
                        eprintln!(
                            "Unknown emit format: {}. Supported for check: hir, diagnostics",
                            fmt
                        );
                        std::process::exit(1);
                    }
                }
                if format == "json" {
                    println!(
                        "{}",
                        action_frontend::error::diagnostics_to_json_pretty(
                            &[],
                            &file.to_string_lossy(),
                            explain
                        )
                        .unwrap_or_else(|_| "{\"version\":1,\"diagnostics\":[]}".to_string())
                    );
                } else {
                    println!("Type checking passed. No errors found.");
                }
            }
            Err(errors) => {
                if format == "json" {
                    match action_frontend::error::diagnostics_to_json_pretty(
                        &errors,
                        &file.to_string_lossy(),
                        explain,
                    ) {
                        Ok(json) => print!("{}", json),
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else if let Ok(source) = fs::read_to_string(&file) {
                    error::report_compiler_errors(&source, &file.to_string_lossy(), &errors);
                } else {
                    for e in &errors {
                        eprintln!("Error: {}", e);
                    }
                }
                std::process::exit(1);
            }
        },
        Commands::Fmt {
            file,
            check,
            tab_size,
            insert_spaces,
        } => {
            if let Err(code) = fmt_file(&file, check, tab_size, insert_spaces) {
                std::process::exit(code);
            }
        }
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
                e.report(&file);
                std::process::exit(1);
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
) -> Result<(), RunFailure> {
    let opt = driver::effective_opt_level(path, opt);

    let checked = driver::check_file(path, explain).map_err(RunFailure::Check)?;

    if emit.as_deref() == Some("hir") {
        driver::emit_hir(&checked, path, false).map_err(RunFailure::Message)?;
        if check {
            println!(
                "Type checking passed for '{}'. No errors found.",
                path.display()
            );
        }
        return Ok(());
    }

    if check {
        println!(
            "Type checking passed for '{}'. No errors found.",
            path.display()
        );
        return Ok(());
    }

    let context = Context::create();
    let cg = driver::codegen_checked(&context, "main", &checked, opt, target)
        .map_err(RunFailure::Message)?;

    let is_cross = target != "native";
    let is_exe = emit.as_deref() == Some("exe");
    if let Some(ref fmt) = emit {
        emit_output(&cg, path, fmt, target).map_err(RunFailure::Message)?;
    }

    if is_cross {
        if !is_exe && emit.is_none() {
            emit_output(&cg, path, "obj", target).map_err(RunFailure::Message)?;
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
        let exit_code = cg.run_jit().map_err(RunFailure::Message)?;
        if exit_code != 0 {
            std::process::exit(exit_code as i32);
        }
    } else {
        let exe_path = aot_exe_path(path, target);
        let status = std::process::Command::new(&exe_path)
            .status()
            .map_err(|e| {
                RunFailure::Message(format!("Failed to run {}: {}", exe_path.display(), e))
            })?;
        if !status.success() {
            return Err(RunFailure::Message(format!(
                "Process exited with status: {}",
                status
            )));
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
) -> Result<(), RunFailure> {
    let opt = driver::effective_opt_level(path, opt);

    let checked = driver::check_file(path, false).map_err(RunFailure::Check)?;

    if emit.as_deref() == Some("hir") {
        return driver::emit_hir(&checked, path, false).map_err(RunFailure::Message);
    }

    let context = Context::create();
    let cg = driver::codegen_checked(&context, "main", &checked, opt, target)
        .map_err(RunFailure::Message)?;

    if let Some(ref fmt) = emit {
        emit_output(&cg, path, fmt, target).map_err(RunFailure::Message)?;
    } else {
        let ir = cg.print_ir();
        let out_path = output.unwrap_or_else(|| path.with_extension("ll"));
        fs::write(&out_path, ir).map_err(|e| {
            RunFailure::Message(format!("Cannot write to '{}': {}", out_path.display(), e))
        })?;
        println!("Compiled to: {}", out_path.display());
    }
    Ok(())
}

fn is_windows_aot_target(target: &str) -> bool {
    if target == "windows-x64"
        || target == "x86_64-pc-windows-gnu"
        || target == "x86_64-pc-windows-msvc"
    {
        return true;
    }
    target == "native" && cfg!(windows)
}

fn aot_exe_path(src_path: &Path, target: &str) -> PathBuf {
    if is_windows_aot_target(target) {
        src_path.with_extension("exe")
    } else {
        src_path.with_extension("")
    }
}

fn aot_object_path(src_path: &Path, target: &str) -> PathBuf {
    if cfg!(windows) && (target == "native" || target == "x86_64-pc-windows-msvc") {
        src_path.with_extension("obj")
    } else {
        src_path.with_extension("o")
    }
}

#[cfg(windows)]
fn find_lld_link_exe() -> Option<PathBuf> {
    if let Ok(prefix) = std::env::var("LLVM_SYS_211_PREFIX") {
        let lld = PathBuf::from(prefix).join("bin/lld-link.exe");
        if lld.is_file() {
            return Some(lld);
        }
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let lld = dir.join("lld-link.exe");
            if lld.is_file() {
                return Some(lld);
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn find_lld_link_exe() -> Option<PathBuf> {
    None
}

#[cfg(windows)]
fn find_msvc_link_exe() -> Option<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let link = dir.join("link.exe");
            if link.is_file() {
                return Some(link);
            }
        }
    }

    let mut search_roots: Vec<PathBuf> = Vec::new();
    if let Ok(pf) = std::env::var("ProgramFiles") {
        search_roots.push(PathBuf::from(pf).join("Microsoft Visual Studio"));
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        search_roots.push(PathBuf::from(pf86).join("Microsoft Visual Studio"));
    }

    for root in search_roots {
        if !root.is_dir() {
            continue;
        }
        let pattern = root.join("VC/Tools/MSVC");
        if let Ok(entries) = std::fs::read_dir(&pattern) {
            for ver in entries.flatten() {
                let link = ver.path().join("bin/Hostx64/x64/link.exe");
                if link.is_file() {
                    return Some(link);
                }
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn find_msvc_link_exe() -> Option<PathBuf> {
    None
}

/// System libraries required when linking `action_host_rt.lib` (Rust std + HTTP/JSON).
fn windows_aot_system_libs() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        return &[
            "kernel32.lib",
            "msvcrt.lib",
            "ucrt.lib",
            "vcruntime.lib",
            "legacy_stdio_definitions.lib",
            "ws2_32.lib",
            "ntdll.lib",
            "advapi32.lib",
            "userenv.lib",
            "bcrypt.lib",
        ];
    }
    #[cfg(not(windows))]
    {
        &[]
    }
}

fn link_aot_executable(
    obj_path: &Path,
    exe_path: &Path,
    src_path: &Path,
    target: &str,
) -> Result<(), String> {
    if cfg!(windows) && (target == "native" || target == "x86_64-pc-windows-msvc") {
        let link_exe = find_lld_link_exe()
            .or_else(find_msvc_link_exe)
            .ok_or_else(|| {
                "Failed to locate lld-link.exe or link.exe (install LLVM or Visual Studio Build Tools)"
                    .to_string()
            })?;
        let mut cmd = std::process::Command::new(link_exe);
        cmd.arg("/NOLOGO")
            .arg("/SUBSYSTEM:CONSOLE")
            .arg(format!("/OUT:{}", exe_path.display()))
            .arg(obj_path);
        if let Some(host_lib) = find_aot_host_staticlib() {
            cmd.arg(host_lib);
        } else {
            eprintln!(
                "warning: action_host_rt static library not found; AOT link may fail if program uses JSON/HTTP"
            );
        }
        cmd.args(windows_aot_system_libs());
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to invoke link.exe: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!(
                "link.exe failed (status {:?}): {}{}",
                output.status.code(),
                stderr,
                stdout
            ));
        }
        if !exe_path.is_file() {
            return Err(format!(
                "link.exe reported success but executable is missing: {}",
                exe_path.display()
            ));
        }
        return Ok(());
    }

    let linker = match target {
        "windows-x64" | "x86_64-pc-windows-gnu" => "x86_64-w64-mingw32-gcc",
        "linux-arm64" | "aarch64-unknown-linux-gnu" => "aarch64-linux-gnu-gcc",
        _ => "cc",
    };
    let mut link_cmd = std::process::Command::new(linker);
    link_cmd.arg("-o").arg(exe_path).arg(obj_path);
    if let Some(host_lib) = find_aot_host_staticlib() {
        link_cmd.arg(host_lib);
    }
    if !matches!(
        target,
        "windows-x64" | "x86_64-pc-windows-gnu" | "wasm" | "wasm32-unknown-unknown"
    ) {
        link_cmd.args(["-lm", "-lpthread", "-ldl"]);
    }
    if let Some(cfg) = config::ProjectConfig::find_and_load(src_path) {
        if cfg.lto {
            link_cmd.arg("-flto");
        }
    }
    let status = link_cmd
        .status()
        .map_err(|e| format!("Failed to invoke linker '{}': {}", linker, e))?;
    if !status.success() {
        return Err(format!("Linker '{}' failed", linker));
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
            let out = aot_object_path(src_path, target);
            cg.emit_object(&out)?;
            println!("Object file written to: {}", out.display());
        }
        "exe" => {
            if target == "wasm" || target == "wasm32-unknown-unknown" {
                return Err("--emit exe is not supported for wasm target. Use --emit obj to produce a .wasm file.".to_string());
            }
            let obj_path = aot_object_path(src_path, target);
            cg.emit_object(&obj_path)?;
            let exe_path = aot_exe_path(src_path, target);
            link_aot_executable(&obj_path, &exe_path, src_path, target)?;
            let _ = std::fs::remove_file(&obj_path);
            println!("Executable written to: {}", exe_path.display());
        }
        other => {
            return Err(format!(
                "Unknown emit format: {}. Supported: ir, bc, asm, obj, exe, hir",
                other
            ))
        }
    }
    Ok(())
}

fn find_aot_host_staticlib() -> Option<String> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profiles = ["release", "debug"];
    let lib_names: &[&str] = if cfg!(windows) {
        &["action_host_rt.lib", "libaction_host_rt.a"]
    } else {
        &["libaction_host_rt.a"]
    };
    let mut candidates = Vec::new();

    let mut push_candidates = |root: &Path| {
        for profile in profiles {
            for name in lib_names {
                candidates.push(root.join(format!("host_rt_build/{profile}/{name}")));
                candidates.push(root.join(format!("{profile}/{name}")));
            }
        }
    };

    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        push_candidates(&PathBuf::from(target_dir));
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..5 {
            if let Some(ref d) = dir {
                push_candidates(d);
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
    }

    push_candidates(&manifest.join("target"));

    for path in candidates {
        if path.exists() {
            return Some(path.to_string_lossy().into_owned());
        }
    }
    None
}

fn fmt_file(path: &PathBuf, check: bool, tab_size: u8, insert_spaces: bool) -> Result<(), i32> {
    let source = fs::read_to_string(path).map_err(|e| {
        eprintln!("Cannot read '{}': {}", path.display(), e);
        1
    })?;

    let mut lexer = lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    let lexer_errors = lexer.take_errors();
    if !lexer_errors.is_empty() {
        error::report_compiler_errors(&source, &path.to_string_lossy(), &lexer_errors);
        return Err(1);
    }

    let options = fmt::FormatOptions {
        tab_size: tab_size as usize,
        insert_spaces,
    };
    let formatted = fmt::format_source(&source, &tokens, &options);

    if formatted == source {
        if check {
            println!("{}: formatted", path.display());
        }
        return Ok(());
    }

    if check {
        eprintln!("{}: would reformat", path.display());
        return Err(1);
    }

    fs::write(path, &formatted).map_err(|e| {
        eprintln!("Cannot write '{}': {}", path.display(), e);
        1
    })?;
    println!("Formatted {}", path.display());
    Ok(())
}
