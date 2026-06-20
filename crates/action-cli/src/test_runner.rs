use action::ast::*;
use action_driver as driver;
use inkwell::context::Context;
use inkwell::targets::{InitializationConfig, Target};
use std::path::PathBuf;

/// Run test functions from a source file
pub fn run_test_file(path: &PathBuf, opt: u8, profile: bool, target: &str) -> Result<(), String> {
    let opt = driver::effective_opt_level(path, opt);

    let checked = driver::load_checked(path, false)?;

    let test_names: Vec<String> = checked
        .program
        .stmts
        .iter()
        .filter_map(|stmt| {
            if let Stmt::Fun {
                name,
                params,
                is_test: true,
                ..
            } = stmt
            {
                if params.is_empty() {
                    Some(name.clone())
                } else {
                    eprintln!("Warning: @test function '{}' has parameters (tests must be parameterless), skipping", name);
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if test_names.is_empty() {
        return Err("No @test functions found in the source file".to_string());
    }

    Target::initialize_x86(&InitializationConfig::default());
    Target::initialize_aarch64(&InitializationConfig::default());

    let context = Context::create();
    let cg = driver::compile_checked(&context, "test_runner", &checked, opt, target)?;

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

    let results = cg.run_tests(&test_names)?;

    let total = results.len();
    let passed = results.iter().filter(|(_, p, _)| *p).count();
    let failed = total - passed;

    println!("\nTest results:");
    println!("{}", "-".repeat(40));
    for (name, pass, output) in &results {
        let status = if *pass { "PASS" } else { "FAIL" };
        println!("  {}: {}", status, name);
        if !output.is_empty() {
            println!("    {}", output);
        }
    }
    println!("{}", "-".repeat(40));
    println!("{} passed, {} failed, {} total", passed, failed, total);

    if failed > 0 {
        Err(format!("{} test(s) failed", failed))
    } else {
        Ok(())
    }
}
