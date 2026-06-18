use crate::ast::*;
use crate::lexer::Span;
use crate::loader::register_types;
use crate::typecheck::TypeChecker;
use inkwell::context::Context;
use inkwell::targets::{InitializationConfig, Target};
use std::io::{self, Write};

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

/// Evaluate a single REPL line: parse, compile, JIT execute
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

    let mut lexer = crate::lexer::Lexer::new(input);
    let tokens = lexer.tokenize();
    let lexer_errors = lexer.take_errors();
    if !lexer_errors.is_empty() {
        for e in &lexer_errors {
            eprintln!("{}", e);
        }
        return Ok(());
    }

    let mut parser = crate::parser::Parser::new(tokens);
    let program: Program;

    if let Ok(expr) = parser.parse_expr() {
        let print_call = Expr::Call {
            func: Box::new(Expr::Ident("println".to_string())),
            args: vec![expr],
            trailing_lambda: None,
        };
        program = Program {
            stmts: vec![Stmt::Fun {
                name: "main".to_string(),
                params: vec![],
                return_type: None,
                body: print_call,
                type_params: vec![],
                is_single_expr: true,
                is_test: false,
                span: Span::default(),
            }],
        };
    } else {
        let mut parser2 = crate::parser::Parser::new(crate::lexer::Lexer::new(input).tokenize());
        match parser2.parse_statement() {
            Ok(stmt) => {
                program = Program { stmts: vec![stmt] };
            }
            Err(e) => {
                eprintln!("{}", e);
                return Ok(());
            }
        }
    }

    let registry = register_types(&program);
    let mut checker = TypeChecker::new(registry.clone());
    let errors = checker.check(&program);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("Type error: {}", e.message);
        }
        return Ok(());
    }

    let target_opt = if target == "native" {
        None
    } else {
        Some(target.to_string())
    };
    let mut cg = crate::codegen::CodeGen::new(context, "repl", registry, target_opt);
    cg.set_opt_level(opt);
    if let Err(e) = cg.compile(&program) {
        eprintln!("Compile error: {}", e);
        return Ok(());
    }
    if let Err(e) = cg.verify() {
        eprintln!("Verify error: {}", e);
        return Ok(());
    }

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
