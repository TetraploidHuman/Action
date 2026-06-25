use action::ast::*;
use action::error;
use action::session::FrontendSession;
use action_span::Span;
use inkwell::context::Context;
use inkwell::targets::{InitializationConfig, Target};
use std::io::{self, Write};
use std::path::Path;
use std::sync::OnceLock;

static REPL_SESSION: OnceLock<FrontendSession> = OnceLock::new();

fn repl_session() -> Result<&'static FrontendSession, String> {
    if let Some(s) = REPL_SESSION.get() {
        return Ok(s);
    }
    let session = FrontendSession::for_repl()?;
    let _ = REPL_SESSION.set(session);
    Ok(REPL_SESSION.get().unwrap())
}

/// Interactive REPL: read, compile, execute, print
pub fn run_repl(opt: u8, profile: bool, target: &str) -> Result<(), String> {
    Target::initialize_x86(&InitializationConfig::default());
    Target::initialize_aarch64(&InitializationConfig::default());

    let context = Context::create();
    eprintln!(
        "Action REPL v{} (type :quit to exit)",
        env!("CARGO_PKG_VERSION")
    );

    let stdin = io::stdin();
    let mut line_buf = String::new();
    let mut multiline = String::new();

    loop {
        let prompt = if multiline.is_empty() { "> " } else { "... " };
        print!("{}", prompt);
        io::stdout().flush().map_err(|e| e.to_string())?;

        line_buf.clear();
        if stdin.read_line(&mut line_buf).map_err(|e| e.to_string())? == 0 {
            if !multiline.is_empty() {
                eval_repl_line(&context, &multiline, opt, profile, target)?;
            }
            break;
        }

        let trimmed = line_buf.trim_end();
        if trimmed == ":quit" || trimmed == ":q" {
            if !multiline.is_empty() {
                eval_repl_line(&context, &multiline, opt, profile, target)?;
            }
            break;
        }

        if trimmed.is_empty() && !multiline.is_empty() {
            let input = std::mem::take(&mut multiline);
            eval_repl_line(&context, &input, opt, profile, target)?;
            continue;
        }

        let needs_continuation = trimmed.ends_with('{')
            || trimmed.ends_with('\\')
            || (trimmed.ends_with(",") && !multiline.is_empty());

        if needs_continuation || !multiline.is_empty() {
            multiline.push_str(trimmed);
            multiline.push('\n');
        } else {
            let _ = eval_repl_line(&context, trimmed, opt, profile, target);
        }
    }
    Ok(())
}

/// Evaluate a single REPL line via CheckedProgram
pub fn eval_repl_line(
    context: &Context,
    input: &str,
    opt: u8,
    profile: bool,
    target: &str,
) -> Result<(), String> {
    let input = input.trim();
    if input.is_empty() || input.starts_with("//") {
        return Ok(());
    }

    let mut lexer = action::lexer::Lexer::new(input);
    let tokens = lexer.tokenize();
    let lexer_errors = lexer.take_errors();
    if !lexer_errors.is_empty() {
        error::report_compiler_errors(input, "<repl>", &lexer_errors);
        return Ok(());
    }

    let mut parser = action::parser::Parser::new(tokens);
    let program: Program;
    if let Ok(expr) = parser.parse_expr() {
        let print_call: Expr = ExprKind::Call {
            func: Box::new(ExprKind::Ident("println".to_string()).into()),
            args: vec![expr],
            trailing_lambda: None,
        }
        .into();
        program = Program {
            stmts: vec![Stmt::Fun {
                name: "main".to_string(),
                params: vec![],
                return_type: None,
                body: print_call,
                type_params: vec![],
                is_single_expr: true,
                is_test: false,
                fn_or_fallback: None,
                span: Span::default(),
            }],
        };
    } else {
        let mut parser2 = action::parser::Parser::new(action::lexer::Lexer::new(input).tokenize());
        match parser2.parse_statement() {
            Ok(stmt) => {
                program = Program { stmts: vec![stmt] };
            }
            Err(e) => {
                error::report_compiler_errors(input, "<repl>", &[e.to_compiler_error()]);
                return Ok(());
            }
        }
    }

    let repl_path = Path::new("<repl>");
    let session = repl_session()?;

    let checked = match session.compile_checked_from_stmts(program.stmts, repl_path, false) {
        Ok(c) => c,
        Err(errors) => {
            error::report_compiler_errors(input, "<repl>", &errors);
            return Ok(());
        }
    };

    let target_opt = if target == "native" {
        None
    } else {
        Some(target.to_string())
    };
    let mut cg =
        action::codegen::CodeGen::new(context, "repl", checked.registry.clone(), target_opt);
    cg.set_opt_level(opt);
    if let Err(e) = cg.compile_checked(&checked) {
        eprintln!("Compile error: {}", e);
        return Ok(());
    }
    if let Err(e) = cg.verify() {
        eprintln!("Verify error: {}", e);
        return Ok(());
    }

    if profile {
        let ir = cg.print_ir();
        eprintln!(
            "[profile] malloc_rc:{} rc_inc:{} rc_dec:{}",
            ir.matches("call ptr @action_malloc_rc").count(),
            ir.matches("call void @action_rc_inc").count(),
            ir.matches("call void @action_rc_dec").count()
        );
    }

    match cg.run_jit() {
        Ok(exit_code) => {
            if exit_code != 0 {
                eprintln!("exit code: {}", exit_code);
            }
        }
        Err(e) => {
            eprintln!("JIT error: {}", e);
        }
    }
    Ok(())
}
