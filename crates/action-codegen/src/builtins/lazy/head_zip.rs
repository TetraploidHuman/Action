use crate::{llvm_err, CodeGen, TypedValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_lazy_head_value(
        &mut self,
        lazy_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (head_val, is_empty) = match &lazy_val {
            TypedValue::LazyList(ptr) => {
                let ll_sv = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "head_ll")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let h = self
                    .builder
                    .build_extract_value(ll_sv, 0, "head_h")
                    .map_err(llvm_err)?;
                let take_count = self
                    .builder
                    .build_extract_value(ll_sv, 3, "head_tc")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_empty = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        take_count,
                        self.i64_ty().const_int(0, false),
                        "ll_is_empty",
                    )
                    .map_err(llvm_err)?;
                (h, is_empty)
            }
            TypedValue::List(ptr) => {
                let list = self.load_list(*ptr)?;
                let len = self
                    .builder
                    .build_extract_value(list, 1, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let data = self
                    .builder
                    .build_extract_value(list, 0, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let zero = self.i64_ty().const_int(0, false);
                let is_empty_cond = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "is_empty")
                    .map_err(llvm_err)?;
                let first_ptr = unsafe {
                    self.builder
                        .build_gep(self.fat_return_type, data, &[zero], "head_gep")
                        .map_err(llvm_err)
                }?;
                let first_fat = self
                    .builder
                    .build_load(self.fat_return_type, first_ptr, "head_fat")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let h = self
                    .builder
                    .build_extract_value(first_fat, 0, "head_h")
                    .map_err(llvm_err)?;
                (h, is_empty_cond)
            }
            _ => return Err("lazyHead: argument must be a LazyList or List".to_string()),
        };

        let head_i64 = head_val.into_int_value();
        let not_empty = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                is_empty,
                self.bool_ty().const_zero(),
                "not_empty",
            )
            .map_err(llvm_err)?;
        self.build_fallible_int_from_ok(head_i64, not_empty)
    }

    pub(crate) fn builtin_lazy_zip_values(
        &mut self,
        v1: TypedValue<'ctx>,
        v2: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let p1 = self.ensure_list_ptr(&v1, "lz1")?;
        let p2 = self.ensure_list_ptr(&v2, "lz2")?;
        let l1 = self.load_list(p1)?;
        let l2 = self.load_list(p2)?;
        let len1 = self
            .builder
            .build_extract_value(l1, 1, "lz_len1")
            .map_err(llvm_err)?
            .into_int_value();
        let len2 = self
            .builder
            .build_extract_value(l2, 1, "lz_len2")
            .map_err(llvm_err)?
            .into_int_value();
        let d1 = self
            .builder
            .build_extract_value(l1, 0, "lz_d1")
            .map_err(llvm_err)?
            .into_pointer_value();
        let d2 = self
            .builder
            .build_extract_value(l2, 0, "lz_d2")
            .map_err(llvm_err)?
            .into_pointer_value();

        let i64 = self.i64_ty();
        let is_len1_lt_len2 = self
            .builder
            .build_int_compare(IntPredicate::SLT, len1, len2, "is_len1_lt_len2")
            .map_err(llvm_err)?;
        let min_len = self
            .builder
            .build_select(is_len1_lt_len2, len1, len2, "lz_min")
            .map_err(llvm_err)?
            .into_int_value();

        let cc = self.call_rt("action_list_create", &[min_len.into()])?;
        let new_list = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "lz_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_list)
            .map_err(llvm_err)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let i_alloca = self.builder.build_alloca(i64, "lz_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_hdr = self.context.append_basic_block(current_fn, "lz_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "lz_bdy");
        let loop_ext = self.context.append_basic_block(current_fn, "lz_ext");

        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let i = self
            .builder
            .build_load(i64, i_alloca, "lz_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i, min_len, "lz_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);

        self.builder.position_at_end(loop_bdy);
        let sp1 = unsafe {
            self.builder
                .build_gep(self.string_type, d1, &[i], "lz_sp1")
                .map_err(llvm_err)
        }?;
        let e1 = self
            .builder
            .build_load(self.string_type, sp1, "lz_e1")
            .map_err(llvm_err)?;
        let sp2 = unsafe {
            self.builder
                .build_gep(self.string_type, d2, &[i], "lz_sp2")
                .map_err(llvm_err)
        }?;
        let e2 = self
            .builder
            .build_load(self.string_type, sp2, "lz_e2")
            .map_err(llvm_err)?;

        let cur = self.load_list(result_alloca)?;
        let cc = self.call_rt("action_list_push", &[cur.into(), e1.into()])?;
        let nl = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_push e1 failed")?;
        self.builder
            .build_store(result_alloca, nl)
            .map_err(llvm_err)?;
        let cur2 = self.load_list(result_alloca)?;
        let cc2 = self.call_rt("action_list_push", &[cur2.into(), e2.into()])?;
        let nl2 = cc2
            .try_as_basic_value()
            .basic()
            .ok_or("list_push e2 failed")?;
        self.builder
            .build_store(result_alloca, nl2)
            .map_err(llvm_err)?;

        let ni = self
            .builder
            .build_int_add(i, i64.const_int(1, false), "lz_ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_ext);
        Ok(TypedValue::List(result_alloca))
    }
}
