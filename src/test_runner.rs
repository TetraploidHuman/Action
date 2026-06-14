use crate::ast::*;
use crate::loader;
use crate::typecheck::TypeRegistry;
use inkwell::context::Context;
use inkwell::targets::{InitializationConfig, Target};
use std::path::PathBuf;

/// Run test functions from a source file
pub fn run_test_file(path: &PathBuf, opt: u8, profile: bool, target: &str) -> Result<(), String> {
    let config = crate::config::ProjectConfig::find_and_load(path);
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

    let test_names: Vec<String> = program
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
    let target_opt = if target == "native" {
        None
    } else {
        Some(target.to_string())
    };
    let mut cg = crate::codegen::CodeGen::new(&context, "test_runner", registry, target_opt);
    cg.set_opt_level(opt);
    cg.compile(&program)?;
    cg.verify()?;

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
