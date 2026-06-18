//! HIR: typed intermediate representation between frontend and codegen.

mod lower;
mod nodes;
mod to_ast;

pub use lower::lower_program;
pub use nodes::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::Program;
    use crate::frontend::loader::register_types;
    use crate::frontend::registry::TypeRegistry;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
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
        registry = register_types(&program);
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
}
