# Archived migration scripts (Expr `{kind, span}` migration, 2026)

These Python scripts were used for the one-time AST expression migration.
**Do not run again** — migration is complete. Kept for historical reference only.

Scripts:
- `apply_ast_expr_migration.py`
- `finalize_expr_migration.py`
- `fix_codegen_expr.py`
- `fix_expr_match_kind.py`
- `fix_parser_expr_legacy.py`
- `fix_parser_expr_v3.py`
- `fix_parser_spans.py`
- `migrate_expr_kind.py`

To regenerate list runtime fragments after editing `*.inc.rs`:

```bash
python3 scripts/resplit_list_include.py   # restores from git if needed
python3 scripts/concat_list_body.py       # writes core/body.inc.rs and tree/body.inc.rs
```
