#!/usr/bin/env python3
"""Apply P2 task patches (idempotent)."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def patch_hir():
    p = ROOT / "crates/action-frontend/src/hir/mod.rs"
    t = p.read_text()
    old = '''        for rel in [
            "examples/bench_cow.ac",
            "examples/map_filter.ac",
            "examples/hello.ac",
        ] {'''
    new = '''        for rel in [
            "examples/bench_cow.ac",
            "examples/map_filter.ac",
            "examples/hello.ac",
            "examples/bench_all.ac",
            "examples/type_ann.ac",
            "examples/when_match.ac",
            "examples/lambda.ac",
            "examples/struct.ac",
        ] {'''
    if old in t:
        p.write_text(t.replace(old, new))

def patch_lib():
    p = ROOT / "crates/action-codegen/src/lib.rs"
    t = p.read_text()
    if "mod list_get_cache;" not in t:
        p.write_text(t.replace("mod lambda_mono;\n", "mod lambda_mono;\nmod list_get_cache;\n"))

def patch_for_loop():
    p = ROOT / "crates/action-codegen/src/for_loop.rs"
    t = p.read_text()
    i = t.find("    /// Cache alloca for action_list_get_cached")
    if i >= 0:
        j = t.find("    /// `for idx < end", i)
        p.write_text(t[:i] + t[j:])

def patch_ht():
    p = ROOT / "crates/action-codegen/src/runtime_decl/define_hash_table.rs"
    t = p.read_text()
    if "HT_MIGRATE_BATCH" not in t:
        p.write_text(t.replace(
            "    const HT_ENTRY_BYTES: u64 = 40;\n",
            "    const HT_ENTRY_BYTES: u64 = 40;\n    const HT_MIGRATE_BATCH: u64 = 16;\n",
        ))

def patch_lambda_mono():
    p = ROOT / "crates/action-codegen/src/lambda_mono.rs"
    t = p.read_text()
    if "ensure_direct_map_filter_walk" in t and "map_first: Option" in t:
        return
    t = t.replace(
        "        self.define_direct_filter_walk_fn(&cache_key, &target)\n    }\n\n    /// Monomorphized fold:",
        """        self.define_direct_filter_walk_fn(&cache_key, &target, None)
    }

    pub(super) fn ensure_direct_map_filter_walk(
        &mut self,
        filter_target: DirectLambdaTarget<'ctx>,
        map_target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let fk = self.direct_lambda_cache_key("f", &filter_target);
        let mk = self.direct_lambda_cache_key("m", &map_target);
        let cache_key = format!(".mono_map_filter_{fk}_{mk}");
        if !self.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct map+filter walk '{cache_key}'"));
        }
        self.define_direct_filter_walk_fn(&cache_key, &filter_target, Some(&map_target))
    }

    /// Monomorphized fold:""",
    )
    t = t.replace(
        "    fn define_direct_filter_walk_fn(\n        &mut self,\n        name: &str,\n        target: &DirectLambdaTarget<'ctx>,\n    ) -> Result<FunctionValue<'ctx>, String> {",
        "    fn define_direct_filter_walk_fn(\n        &mut self,\n        name: &str,\n        target: &DirectLambdaTarget<'ctx>,\n        map_first: Option<&DirectLambdaTarget<'ctx>>,\n    ) -> Result<FunctionValue<'ctx>, String> {",
        1,
    )
    old_leaf = """        let elem_tag = self
            .builder
            .build_extract_value(elem, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let pred_bv = self.emit_direct_lambda_call(target, elem_tag, "pred")?;"""
    new_leaf = """        let elem_tag = self
            .builder
            .build_extract_value(elem, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let (pred_arg, push_elem) = if let Some(map_t) = map_first {
            let mapped_bv = self.emit_direct_lambda_call(map_t, elem_tag, "mapped")?;
            let mapped_tag = self.fat_tag_from_call_result(mapped_bv)?;
            let mapped_fat = self.fat_struct_from_call_result(mapped_bv)?;
            (mapped_tag, mapped_fat)
        } else {
            (elem_tag, elem)
        };
        let pred_bv = self.emit_direct_lambda_call(target, pred_arg, "pred")?;"""
    if old_leaf in t:
        t = t.replace(old_leaf, new_leaf, 1)
        t = t.replace(
            "self.builder.build_store(buf_ep, elem).map_err(llvm_err)?;",
            "self.builder.build_store(buf_ep, push_elem).map_err(llvm_err)?;",
            1,
        )
    if "try_builtin_map_filter_direct" not in t:
        t = t.replace(
            "    /// Run fold via monomorphized direct-call walk when eligible; otherwise None.",
            """    pub(super) fn try_builtin_map_filter_direct(
        &mut self,
        filter_fn: TypedValue<'ctx>,
        map_fn: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let filter_target = match self.try_direct_lambda(filter_fn) {
            Some(t) => t,
            None => return Ok(None),
        };
        let map_target = match self.try_direct_lambda(map_fn) {
            Some(t) => t,
            None => return Ok(None),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Ok(None),
        };
        let list_struct = self.load_list(list_ptr)?;
        let walk_fn = self.ensure_direct_map_filter_walk(filter_target, map_target)?;
        let cc = self
            .builder
            .build_call(walk_fn, &[list_struct.into()], "mono_map_filter")
            .map_err(llvm_err)?;
        let result_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("mono map+filter call failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "map_filter_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, result_bv)
            .map_err(llvm_err)?;
        Ok(Some(TypedValue::List(result_alloca)))
    }

    /// Run fold via monomorphized direct-call walk when eligible; otherwise None.""",
        )
    p.write_text(t)

def patch_builtins_iter():
    p = ROOT / "crates/action-codegen/src/builtins_iter.rs"
    t = p.read_text()
    if "try_builtin_filter_map_fusion" not in t:
        t = t.replace(
            "    pub(super) fn builtin_filter(\n        &mut self,\n        args: &[Expr],\n        trailing: &Option<Box<Expr>>,\n    ) -> Result<TypedValue<'ctx>, String> {\n        let (fn_ptr, list_val)",
            "    pub(super) fn builtin_filter(\n        &mut self,\n        args: &[Expr],\n        trailing: &Option<Box<Expr>>,\n    ) -> Result<TypedValue<'ctx>, String> {\n        if let Some(result) = self.try_builtin_filter_map_fusion(args, trailing)? {\n            return Ok(result);\n        }\n\n        let (fn_ptr, list_val)",
        )
        fusion = '''
    fn try_builtin_filter_map_fusion(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        if let Some(filter_lam) = trailing {
            if args.len() != 1 {
                return Ok(None);
            }
            if let Expr::Call {
                func,
                args: map_args,
                trailing_lambda: Some(map_lam),
                ..
            } = &args[0]
            {
                if matches!(func.as_ref(), Expr::Ident(n) if n == "map") && map_args.len() == 1 {
                    let fv = self.compile_expr(filter_lam)?;
                    let mv = self.compile_expr(map_lam)?;
                    let lv = self.compile_expr(&map_args[0])?;
                    return self.try_builtin_map_filter_direct(fv, mv, lv);
                }
            }
            return Ok(None);
        }
        if args.len() == 2 {
            if let Expr::Call {
                func,
                args: map_args,
                trailing_lambda: None,
                ..
            } = &args[1]
            {
                if matches!(func.as_ref(), Expr::Ident(n) if n == "map") && map_args.len() == 2 {
                    let fv = self.compile_expr(&args[0])?;
                    let mv = self.compile_expr(&map_args[0])?;
                    let lv = self.compile_expr(&map_args[1])?;
                    return self.try_builtin_map_filter_direct(fv, mv, lv);
                }
            }
            if let Expr::Call {
                func,
                args: map_args,
                trailing_lambda: Some(map_lam),
                ..
            } = &args[1]
            {
                if matches!(func.as_ref(), Expr::Ident(n) if n == "map") && map_args.len() == 1 {
                    let fv = self.compile_expr(&args[0])?;
                    let mv = self.compile_expr(map_lam)?;
                    let lv = self.compile_expr(&map_args[0])?;
                    return self.try_builtin_map_filter_direct(fv, mv, lv);
                }
            }
        }
        Ok(None)
    }
'''
        t = t[:-2] + fusion + "\n}\n"
    if "list_get_cached_fat" not in t:
        for hdr in ["find_hdr", "fi_hdr", "tw_hdr", "dw_hdr"]:
            t = t.replace(
                f'        self.builder\n            .build_store(i_a, i64.const_int(0, false))\n            .map_err(llvm_err)?;\n        let hdr = self.context.append_basic_block(current_fn, "{hdr}");',
                f'        self.builder\n            .build_store(i_a, i64.const_int(0, false))\n            .map_err(llvm_err)?;\n        let get_cache = self.alloc_list_get_cache()?;\n        let hdr = self.context.append_basic_block(current_fn, "{hdr}");',
                1,
            )
        t = t.replace(
            "            .build_store(right_a, right_bv)\n            .map_err(llvm_err)?;\n        let hdr = self.context.append_basic_block(current_fn, \"part_hdr\");",
            "            .build_store(right_a, right_bv)\n            .map_err(llvm_err)?;\n        let get_cache = self.alloc_list_get_cache()?;\n        let hdr = self.context.append_basic_block(current_fn, \"part_hdr\");",
            1,
        )
        t = t.replace(
            "            .build_store(cnt_a, i64.const_int(0, false))\n            .map_err(llvm_err)?;\n        let hdr = self.context.append_basic_block(current_fn, \"cnt_hdr\");",
            "            .build_store(cnt_a, i64.const_int(0, false))\n            .map_err(llvm_err)?;\n        let get_cache = self.alloc_list_get_cache()?;\n        let hdr = self.context.append_basic_block(current_fn, \"cnt_hdr\");",
            1,
        )
        t = t.replace(
            "        self.builder.build_store(i_a, one).map_err(llvm_err)?;\n        // Init: load first element into acc\n        let init_bb",
            "        self.builder.build_store(i_a, one).map_err(llvm_err)?;\n        let get_cache = self.alloc_list_get_cache()?;\n        // Init: load first element into acc\n        let init_bb",
            1,
        )
        t = t.replace(
            "        self.builder.build_store(i_a, start_i).map_err(llvm_err)?;\n        let hdr = self.context.append_basic_block(current_fn, \"fr_hdr\");",
            "        self.builder.build_store(i_a, start_i).map_err(llvm_err)?;\n        let get_cache = self.alloc_list_get_cache()?;\n        let hdr = self.context.append_basic_block(current_fn, \"fr_hdr\");",
            1,
        )
        t = t.replace(
            "        let input_list = self.load_list(list_ptr)?;\n        let elem = self.call_rt(\"action_list_get\", &[input_list.into(), iv.into()])?;\n        let elem_val = elem.try_as_basic_value().basic().ok_or(\"list_get failed\")?;",
            "        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;",
        )
        t = t.replace(
            "        let input_list0 = self.load_list(list_ptr)?;\n        let first = self.call_rt(\"action_list_get\", &[input_list0.into(), zero.into()])?;\n        let first_val = first\n            .try_as_basic_value()\n            .basic()\n            .ok_or(\"list_get failed\")?;",
            "        let first_val = self.list_get_cached_fat(list_ptr, zero, get_cache)?;",
            1,
        )
    p.write_text(t)

def main():
    patch_hir()
    patch_lib()
    patch_for_loop()
    patch_ht()
    patch_lambda_mono()
    patch_builtins_iter()
    print("task patches applied")

if __name__ == "__main__":
    main()
