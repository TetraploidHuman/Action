//! HIR: typed intermediate representation between frontend and codegen.

mod lower;
mod nodes;
mod to_ast;

pub use lower::lower_program;
pub use nodes::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Program;
    use crate::lexer::Lexer;
    use crate::loader::build_type_registry;
    use crate::parser::Parser;
    use crate::registry::TypeRegistry;
    use crate::typecheck::TypeChecker;

    fn check_and_lower(source: &str) -> (Program, HirModule) {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let program = Parser::new(tokens)
            .parse_program()
            .expect("parse should succeed");
        let mut registry = TypeRegistry::new();
        for stmt in &program.stmts {
            let _ = registry.register(stmt);
        }
        registry = build_type_registry(&program).expect("register types");
        let mut checker = TypeChecker::new(registry);
        let errors = checker.check(&program);
        assert!(errors.is_empty(), "type errors: {:?}", errors);
        let hir = lower_program(&program, &checker);
        (program, hir)
    }

    #[test]
    fn hir_round_trip_hello() {
        let (program, hir) = check_and_lower("fun main() { println(\"hi\") }");
        assert_eq!(hir.to_program(), program);
    }

    #[test]
    fn hir_round_trip_arith() {
        let (program, hir) = check_and_lower("fun main() {\n  val x = 1 + 2\n  println(x * 3)\n}");
        assert_eq!(hir.to_program(), program);
    }

    #[test]
    fn hir_json_contains_types() {
        let (_, hir) = check_and_lower("fun main() { println(42) }");
        let json = hir.to_json_pretty().unwrap();
        assert!(json.contains("Int"));
        assert!(json.contains("main"));
    }

    #[test]
    fn hir_when_round_trip() {
        let (program, hir) = check_and_lower(
            "fun main() {\n  println(when 1 {\n    0 -> 10\n    else -> 20\n  })\n}",
        );
        assert_eq!(hir.to_program(), program);
    }

    /// Fun params must be re-seeded at lower so `if b { wrap(s) } else { … }` keeps String.
    #[test]
    fn hir_fun_params_seed_when_call_string_ty() {
        let src = r#"
fun wrap(s: String) -> String { return s }
fun pick(s: String, b: Bool) -> String {
  return if b { wrap(s) } else { "x" }
}
"#;
        let (_, hir) = check_and_lower(src);
        let pick = hir
            .stmts
            .iter()
            .find_map(|s| match s {
                HirStmt::Fun { name, body, .. } if name == "pick" => Some(body),
                _ => None,
            })
            .expect("pick");
        let HirExprKind::Block(stmts) = &pick.kind else {
            panic!("pick body block");
        };
        let HirStmt::Return {
            value: Some(ret), ..
        } = &stmts[0]
        else {
            panic!("return if");
        };
        let HirExprKind::When(w) = &ret.kind else {
            panic!("if");
        };
        let HirWhenKind::OneLine { then_expr, .. } = &w.kind else {
            panic!("one-line");
        };
        assert_eq!(
            then_expr.ty,
            crate::ast::Type::Named("String".into()),
            "Call in when arm must not collapse to Unit"
        );
    }

    #[test]
    fn hir_round_trip_examples() {
        let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        for rel in [
            "examples/bench_cow.ac",
            "examples/map_filter.ac",
            "examples/hello.ac",
        ] {
            let path = workspace.join(rel);
            let checked = crate::loader::check_file(&path, false)
                .unwrap_or_else(|e| panic!("load {} failed: {:?}", rel, e));
            assert!(
                checked.verify_hir_round_trip(),
                "HIR round-trip failed for {}",
                rel
            );
            let json = checked.hir_json_pretty().expect("hir json");
            assert!(json.contains("main"), "missing main in HIR for {}", rel);
        }
    }
}
