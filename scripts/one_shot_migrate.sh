#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

python3 scripts/apply_ast_expr_migration.py
python3 scripts/migrate_expr_kind.py

python3 << 'PY'
from pathlib import Path
import scripts.fix_expr_match_kind as f
for p in Path('crates/action-codegen/src').rglob('*.rs'):
    t = f.fix_match_kind(p.read_text())
    p.write_text(t)
PY

python3 << 'PY'
from pathlib import Path
p = Path('crates/action-frontend/src/parser/mod.rs')
t = p.read_text()
if 'fn make_expr' not in t:
    ins = '''    pub(crate) fn merge_spans(a: Span, b: Span) -> Span {
        Span { start: a.start.min(b.start), end: a.end.max(b.end), line: a.line, col: a.col }
    }
    pub(crate) fn merge_expr_spans(left: &Expr, right: &Expr) -> Span {
        Self::merge_spans(left.span, right.span)
    }
    pub(crate) fn make_expr(&self, kind: ExprKind) -> Expr {
        Expr::new(kind, self.current_span())
    }
    pub(crate) fn make_expr_from(&self, left: &Expr, kind: ExprKind) -> Expr {
        Expr::new(kind, left.span)
    }
    pub(crate) fn make_expr_merge(&self, left: &Expr, right: &Expr, kind: ExprKind) -> Expr {
        Expr::new(kind, Self::merge_expr_spans(left, right))
    }

'''
    t = t.replace('    // ---- Parse Program ----', ins + '    // ---- Parse Program ----', 1)
    p.write_text(t)
PY

python3 scripts/fix_parser_expr_v3.py
python3 scripts/fix_parser_expr_legacy.py
python3 scripts/fix_parser_spans.py
python3 scripts/finalize_expr_migration.py
sed -i 's/= &\([a-z_.]*\)\.kind { {/= \&\1.kind {/g' crates/action-frontend/src/parser/expr.rs crates/action-frontend/src/typecheck/*.rs crates/action-frontend/src/loader/resolve.rs 2>/dev/null || true

python3 << 'PY'
from pathlib import Path
p = Path('crates/action-frontend/src/loader/resolve.rs')
t = p.read_text()
old = """    fn transform_expr(expr: &mut Expr, prefixes: &HashSet<String>) {
        if let Expr::FieldAccess(ref base, ref field) = expr {
            if let Expr::Ident(ref ident) = **base {
                if prefixes.contains(ident) {
                    *expr = Expr::Ident(format!("{}_{}", ident, field));
                    return;
                }
            }
        }
        match expr {"""
new = """    fn transform_expr(expr: &mut Expr, prefixes: &HashSet<String>) {
        if let ExprKind::FieldAccess(ref base, ref field) = expr.kind {
            if let ExprKind::Ident(ref ident) = base.kind {
                if prefixes.contains(ident) {
                    expr.kind = ExprKind::Ident(format!("{}_{}", ident, field));
                    return;
                }
            }
        }
        match &mut expr.kind {"""
if old in t:
    t = t.replace(old, new)
t = t.replace('= &expr.kind { {', '= &expr.kind {')
p.write_text(t)
PY

python3 << 'PY'
from pathlib import Path
import re
p = Path('crates/action-frontend/src/parser/pattern.rs')
t = p.read_text().replace('Expr::When','ExprKind::When').replace('Expr::For','ExprKind::For')
t = re.sub(r'return Ok\(ExprKind::(When|For)\(', r'return Ok(self.make_expr(ExprKind::\1(', t)
lines = []
for line in t.splitlines():
    if line.strip().endswith('})));') and any('make_expr(ExprKind::' in l for l in lines[-8:]):
        line = line.replace('})));', '}))));')
    lines.append(line)
t = '\n'.join(lines) + '\n'
t = t.replace('ExprKind::Literal(Literal::Unit)\n                };', 'self.make_expr(ExprKind::Literal(Literal::Unit))\n                };')
p.write_text(t)
PY

python3 << 'PY'
from pathlib import Path
import scripts.migrate_expr_kind as m
import scripts.fix_expr_match_kind as f

def fix(path):
    m.migrate_file(path)
    t = path.read_text()
    t = t.replace('match expr {', 'match &expr.kind {')
    reps = [
        ('if let ExprKind::Ident(name) = func.as_ref()', 'if let ExprKind::Ident(name) = &func.kind'),
        ('} else if let ExprKind::FieldAccess(receiver, method) = func.as_ref()', '} else if let ExprKind::FieldAccess(receiver, method) = &func.kind'),
        ('if let ExprKind::Ident(name) = value.as_ref()', 'if let ExprKind::Ident(name) = &value.kind'),
        ('if let ExprKind::Ident(name) = target.as_ref()', 'if let ExprKind::Ident(name) = &target.kind'),
        ('if let ExprKind::Ident(name) = func {', 'if let ExprKind::Ident(name) = &func.kind {'),
        ('} else if let ExprKind::FieldAccess(receiver, method) = func {', '} else if let ExprKind::FieldAccess(receiver, method) = &func.kind {'),
        ('matches!(condition.as_ref(), ExprKind::Binary', 'matches!(&condition.kind, ExprKind::Binary'),
        ('matches!(func.as_ref(), ExprKind::Ident', 'matches!(&func.kind, ExprKind::Ident'),
        ('match (lhs.as_ref(), rhs.as_ref())', 'match (&lhs.kind, &rhs.kind)'),
        ('match condition.as_ref().kind {', 'match &condition.as_ref().kind {'),
        ('infer_expr_type(lhs)\n                                        .unwrap_or', 'infer_expr_type(lhs.as_ref())\n                                        .unwrap_or'),
        ('infer_expr_type(lhs.as_ref())?', 'infer_expr_type(lhs)?'),
        ('matches!(arg, ExprKind::Lambda', 'matches!(&arg.kind, ExprKind::Lambda'),
        ('.filter(|a| !matches!(a, ExprKind::Lambda', '.filter(|a| !matches!(&a.kind, ExprKind::Lambda'),
        ('infer_expr_type_with_locals(e, locals)', 'infer_expr_type_with_locals(&e, locals)'),
    ]
    for a, b in reps:
        t = t.replace(a, b)
    for old in [
        '&ExprKind::Call {\n                                    func: Box::new(ExprKind::Ident(method.clone())),\n                                    args: all_args,\n                                    trailing_lambda: None,\n                                }',
        '&Expr::Call {\n                                    func: Box::new(Expr::Ident(method.clone())),\n                                    args: all_args,\n                                    trailing_lambda: None,\n                                }',
    ]:
        t = t.replace(old, '&Expr::call(Expr::ident(method), all_args)')
    path.write_text(t)

for p in Path('crates/action-frontend/src/typecheck').glob('*.rs'):
    fix(p)
PY

python3 << 'PY'
from pathlib import Path
p = Path('crates/action-frontend/src/session.rs')
t = p.read_text()
if 'compile_source_buffer' in t and 'Expr::call(Expr::ident("println")' not in t:
    import re
    block = '''pub fn compile_source_buffer(
        &self,
        source: &str,
        path: &Path,
        explain: bool,
    ) -> Result<CheckedProgram, Vec<CompilerError>> {
        let source = source.trim();
        if source.is_empty() || source.starts_with("//") {
            return Err(vec![]);
        }
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize();
        let lexer_errors = lexer.take_errors();
        if !lexer_errors.is_empty() {
            return Err(lexer_errors);
        }
        let mut parser = Parser::new(tokens);
        let user_stmts = if let Ok(expr) = parser.parse_expr() {
            let print_call = Expr::call(Expr::ident("println"), vec![expr]);
            vec![Stmt::Fun {
                name: "main".to_string(),
                params: vec![],
                return_type: None,
                body: print_call,
                type_params: vec![],
                is_single_expr: true,
                is_test: false,
                span: Span::default(),
            }]
        } else {
            let mut parser2 = Parser::new(crate::lexer::Lexer::new(source).tokenize());
            match parser2.parse_statement() {
                Ok(stmt) => vec![stmt],
                Err(e) => return Err(vec![e.to_compiler_error()]),
            }
        };
        let program = self
            .assemble_program(Some(path), user_stmts)
            .map_err(|e| vec![CompilerError::new(e)])?;
        let (registry, checker) = self.typecheck_with_checker(&program, explain)?;
        Ok(CheckedProgram::new(program, registry, &checker))
    }'''
    if re.search(r'pub fn compile_source_buffer', t):
        t = re.sub(r'pub fn compile_source_buffer\([\s\S]*?\n    \}', block, t, count=1)
    else:
        t = t.replace('    /// Recovering parse + typecheck for a single buffer (LSP).', block + '\n\n    /// Recovering parse + typecheck for a single buffer (LSP).', 1)
    p.write_text(t)
PY

python3 -c "
from pathlib import Path
import re
for path in Path('crates/action-codegen/src').rglob('*.rs'):
    t=path.read_text(); n=re.sub(r'= ExprKind::Ident\(', '= Expr::ident(', t)
    if n!=t: path.write_text(n)
"

test -n "$(rg 'pub struct Expr' crates/action-frontend/src/ast.rs)"
python3 scripts/apply_ast_expr_migration.py
sync
nix-shell --run 'cargo build --release && cargo test --release --test integration -- --test-threads=1'
