// Submodule: runtime_decl/define_list_extra
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_extra(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        // ---- action_list_tail({ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Returns a new list without the first element (empty list if input is empty)
        let lt_fn = self.module.add_function(
            "action_list_tail",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(lt_fn, "entry");
        self.builder.position_at_end(entry);
        let lt_list = lt_fn.get_first_param().unwrap().into_struct_value();
        let lt_len = self
            .builder
            .build_extract_value(lt_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let _lt_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, lt_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let lt_empty_or_one = self
            .builder
            .build_int_compare(
                IntPredicate::SLE,
                lt_len,
                i64.const_int(1, false),
                "empty_or_one",
            )
            .map_err(llvm_err)?;
        let lt_do = self.context.append_basic_block(lt_fn, "do");
        let lt_empty_bb = self.context.append_basic_block(lt_fn, "empty_ret");
        let _ = self
            .builder
            .build_conditional_branch(lt_empty_or_one, lt_empty_bb, lt_do);
        self.builder.position_at_end(lt_empty_bb);
        // Return empty list
        let cc0 = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let lte_r = cc0.try_as_basic_value().unwrap_basic();
        let _ = self.builder.build_return(Some(&lte_r));
        // Copy elements [1..len) via drop(1) range walk
        self.builder.position_at_end(lt_do);
        let lt_drop_fn = self.module.get_function("action_list_drop").unwrap();
        let lt_drop_r = self
            .builder
            .build_call(
                lt_drop_fn,
                &[lt_list.into(), i64.const_int(1, false).into()],
                "drop1",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lt_drop_r));

        // ---- action_list_zip({ptr,i64,i64}, {ptr,i64,i64}) -> {ptr,i64,i64} ----
        let lz_fn = self.module.add_function(
            "action_list_zip",
            self.list_type
                .fn_type(&[self.list_type.into(), self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(lz_fn, "entry");
        self.builder.position_at_end(entry);
        let lz_a = lz_fn.get_first_param().unwrap().into_struct_value();
        let lz_b = lz_fn.get_nth_param(1).unwrap().into_struct_value();
        let lz_alen = self
            .builder
            .build_extract_value(lz_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let lz_blen = self
            .builder
            .build_extract_value(lz_b, 1, "blen")
            .map_err(llvm_err)?
            .into_int_value();
        let lz_altb = self
            .builder
            .build_int_compare(IntPredicate::SLT, lz_alen, lz_blen, "altb")
            .map_err(llvm_err)?;
        let lz_min = self
            .builder
            .build_select(lz_altb, lz_alen, lz_blen, "min")
            .map_err(llvm_err)?
            .into_int_value();
        let cc3 = self.call_rt("action_list_create", &[lz_min.into()])?;
        let lz_new = cc3.try_as_basic_value().unwrap_basic().into_struct_value();
        let lz_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_new_alloc, lz_new)
            .map_err(llvm_err)?;
        let lz_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(lz_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let lz_cache_a = self
            .builder
            .build_alloca(i8.array_type(32), "cache_a")
            .map_err(llvm_err)?;
        let lz_cache_b = self
            .builder
            .build_alloca(i8.array_type(32), "cache_b")
            .map_err(llvm_err)?;
        let lz_cache_a_i8 = self
            .builder
            .build_pointer_cast(lz_cache_a, ptr, "cache_a_i8")
            .map_err(llvm_err)?;
        let lz_cache_b_i8 = self
            .builder
            .build_pointer_cast(lz_cache_b, ptr, "cache_b_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_cache_a_i8, i8.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_cache_b_i8, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let lz_loop = self.context.append_basic_block(lz_fn, "loop");
        let lz_body = self.context.append_basic_block(lz_fn, "body");
        let lz_done = self.context.append_basic_block(lz_fn, "done");
        let _ = self.builder.build_unconditional_branch(lz_loop);
        self.builder.position_at_end(lz_loop);
        let lz_i = self
            .builder
            .build_load(i64, lz_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let lz_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lz_i, lz_min, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lz_cond, lz_body, lz_done);
        self.builder.position_at_end(lz_body);
        let lz_get_cached_fn = self.module.get_function("action_list_get_cached").unwrap();
        let lz_av = self
            .builder
            .build_call(
                lz_get_cached_fn,
                &[lz_a.into(), lz_i.into(), lz_cache_a.into()],
                "av",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get a failed")?;
        let lz_bv = self
            .builder
            .build_call(
                lz_get_cached_fn,
                &[lz_b.into(), lz_i.into(), lz_cache_b.into()],
                "bv",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get b failed")?;
        // Allocate tuple struct {fat_a, fat_b}
        let lz_tup_ty = self
            .context
            .struct_type(&[self.string_type.into(), self.string_type.into()], false);
        let lz_tup_size = i64.const_int(32, false);
        let lz_tup = self
            .builder
            .build_call(malloc_rc_fn, &[lz_tup_size.into()], "tup")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated tuple
        let lz_tup_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(lz_tup, i64, "lz_tup_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "lz_tup_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(lz_tup_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        let lz_tup_a = self
            .builder
            .build_struct_gep(lz_tup_ty, lz_tup, 0, "ta")
            .map_err(llvm_err)?;
        let lz_tup_b = self
            .builder
            .build_struct_gep(lz_tup_ty, lz_tup, 1, "tb")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_tup_a, lz_av)
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_tup_b, lz_bv)
            .map_err(llvm_err)?;
        // Fat struct: tag=5 (Struct), data=ptr to tuple
        let lz_fat_und = self.string_type.get_undef();
        let lz_fat1 = self
            .builder
            .build_insert_value(lz_fat_und, self.i64_ty().const_int(5, false), 0, "tag")
            .map_err(llvm_err)?;
        let lz_fat2 = self
            .builder
            .build_insert_value(lz_fat1, lz_tup, 1, "data")
            .map_err(llvm_err)?;
        // Push into result list
        let lz_cur = self
            .builder
            .build_load(self.list_type, lz_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let lz_push_cc = self.call_rt(
            "action_list_push",
            &[lz_cur.into(), lz_fat2.as_basic_value_enum().into()],
        )?;
        let lz_nv = lz_push_cc.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(lz_new_alloc, lz_nv)
            .map_err(llvm_err)?;
        let lz_ni = self
            .builder
            .build_int_add(lz_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_i_alloc, lz_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lz_loop);
        self.builder.position_at_end(lz_done);
        let lz_rv = self
            .builder
            .build_load(self.list_type, lz_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lz_rv));

        // ---- action_list_init({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let li_fn = self.module.add_function(
            "action_list_init",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(li_fn, "entry");
        self.builder.position_at_end(entry);
        let li_list = li_fn.get_first_param().unwrap().into_struct_value();
        let li_len = self
            .builder
            .build_extract_value(li_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let li_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let li_do = self.context.append_basic_block(li_fn, "do");
        let li_empty_bb = self.context.append_basic_block(li_fn, "empty_ret");
        let _ = self
            .builder
            .build_conditional_branch(li_empty, li_empty_bb, li_do);
        self.builder.position_at_end(li_empty_bb);
        let cce = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let li_er = cce.try_as_basic_value().unwrap_basic();
        let _ = self.builder.build_return(Some(&li_er));
        self.builder.position_at_end(li_do);
        let li_nlen = self
            .builder
            .build_int_sub(li_len, i64.const_int(1, false), "nlen")
            .map_err(llvm_err)?;
        let li_take_fn = self.module.get_function("action_list_take").unwrap();
        let li_take_r = self
            .builder
            .build_call(li_take_fn, &[li_list.into(), li_nlen.into()], "take")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_take_r));

        // ---- action_list_last({ptr, i64, i64}) -> {i64, ptr} ----
        let llast_fn = self.module.add_function(
            "action_list_last",
            self.string_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(llast_fn, "entry");
        self.builder.position_at_end(entry);
        let ll_list = llast_fn.get_first_param().unwrap().into_struct_value();
        let ll_len = self
            .builder
            .build_extract_value(ll_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ll_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, ll_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let ll_has = self.context.append_basic_block(llast_fn, "has");
        let ll_none = self.context.append_basic_block(llast_fn, "none");
        let _ = self
            .builder
            .build_conditional_branch(ll_empty, ll_none, ll_has);
        self.builder.position_at_end(ll_none);
        let ll_none_val = self.string_type.const_zero();
        let _ = self.builder.build_return(Some(&ll_none_val));
        self.builder.position_at_end(ll_has);
        let ll_last_idx = self
            .builder
            .build_int_sub(ll_len, i64.const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        let ll_get_fn = self.module.get_function("action_list_get").unwrap();
        let ll_val = self
            .builder
            .build_call(ll_get_fn, &[ll_list.into(), ll_last_idx.into()], "val")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?;
        let _ = self.builder.build_return(Some(&ll_val));

        // ---- action_string_chars({i64, ptr}) -> {ptr, i64, i64} ----
        let ch_fn = self.module.add_function(
            "action_string_chars",
            self.list_type.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(ch_fn, "entry");
        self.builder.position_at_end(entry);
        let ch_s = ch_fn.get_first_param().unwrap().into_struct_value();
        let ch_len = self
            .builder
            .build_extract_value(ch_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_ptr = self
            .builder
            .build_extract_value(ch_s, 1, "sptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc0 = self.call_rt("action_list_create", &[ch_len.into()])?;
        let ch_list_init = cc0.try_as_basic_value().unwrap_basic().into_struct_value();
        let ch_list_alloc = self
            .builder
            .build_alloca(self.list_type, "list_acc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_list_alloc, ch_list_init)
            .map_err(llvm_err)?;
        let ch_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(ch_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let ch_loop = self.context.append_basic_block(ch_fn, "loop");
        let ch_body = self.context.append_basic_block(ch_fn, "body");
        let ch_done = self.context.append_basic_block(ch_fn, "done");
        let _ = self.builder.build_unconditional_branch(ch_loop);
        self.builder.position_at_end(ch_loop);
        let ch_i = self
            .builder
            .build_load(i64, ch_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ch_i, ch_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ch_cond, ch_body, ch_done);
        self.builder.position_at_end(ch_body);
        let ch_cp = unsafe {
            self.builder
                .build_gep(i8, ch_ptr, &[ch_i], "cp")
                .map_err(llvm_err)
        }?;
        let ch_c = self
            .builder
            .build_load(i8, ch_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        // Create a 1-byte string from this character
        let ch_salloc = self
            .builder
            .build_call(malloc_rc_fn, &[i64.const_int(1, false).into()], "salloc")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated buffer
        let ch_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(ch_salloc, i64, "ch_sa_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "ch_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(ch_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_salloc, ch_c)
            .map_err(llvm_err)?;
        let ch_fat = self.string_type.get_undef();
        let ch_fat_tag = self
            .builder
            .build_insert_value(ch_fat, self.i64_ty().const_int(1, false), 0, "tag")
            .map_err(llvm_err)?;
        let ch_fat_val = self
            .builder
            .build_insert_value(ch_fat_tag, ch_salloc, 1, "data")
            .map_err(llvm_err)?;
        let ch_cur = self
            .builder
            .build_load(self.list_type, ch_list_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_push = self.call_rt(
            "action_list_push",
            &[ch_cur.into(), ch_fat_val.as_basic_value_enum().into()],
        )?;
        let ch_new = ch_push.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(ch_list_alloc, ch_new)
            .map_err(llvm_err)?;
        let ch_ni = self
            .builder
            .build_int_add(ch_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_i_alloc, ch_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ch_loop);
        self.builder.position_at_end(ch_done);
        let ch_rv = self
            .builder
            .build_load(self.list_type, ch_list_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ch_rv));

        // ---- action_list_with_index({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let wi_fn = self.module.add_function(
            "action_list_with_index",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(wi_fn, "entry");
        self.builder.position_at_end(entry);
        let wi_list = wi_fn.get_first_param().unwrap().into_struct_value();
        let wi_len = self
            .builder
            .build_extract_value(wi_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let cc = self.call_rt("action_list_create", &[wi_len.into()])?;
        let wi_new_init = cc.try_as_basic_value().unwrap_basic().into_struct_value();
        let wi_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wi_new_alloc, wi_new_init)
            .map_err(llvm_err)?;
        let wi_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(wi_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let wi_cache_a = self
            .builder
            .build_alloca(i8.array_type(32), "cache")
            .map_err(llvm_err)?;
        let wi_cache_i8 = self
            .builder
            .build_pointer_cast(wi_cache_a, ptr, "cache_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wi_cache_i8, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let wi_loop = self.context.append_basic_block(wi_fn, "loop");
        let wi_body = self.context.append_basic_block(wi_fn, "body");
        let wi_done = self.context.append_basic_block(wi_fn, "done");
        let _ = self.builder.build_unconditional_branch(wi_loop);
        self.builder.position_at_end(wi_loop);
        let wi_i = self
            .builder
            .build_load(i64, wi_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let wi_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, wi_i, wi_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(wi_cond, wi_body, wi_done);
        self.builder.position_at_end(wi_body);
        let wi_get_cached_fn = self.module.get_function("action_list_get_cached").unwrap();
        let wi_ev = self
            .builder
            .build_call(
                wi_get_cached_fn,
                &[wi_list.into(), wi_i.into(), wi_cache_a.into()],
                "ev",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?
            .into_struct_value();
        // Create pair tuple {i64 index, fat_elem}
        let wi_tup_ty = self
            .context
            .struct_type(&[i64.into(), self.string_type.into()], false);
        let wi_tup = self
            .builder
            .build_call(malloc_rc_fn, &[i64.const_int(24, false).into()], "tup")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated tuple
        let wi_tup_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(wi_tup, i64, "wi_tup_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "wi_tup_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(wi_tup_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        let wi_tup_i = self
            .builder
            .build_struct_gep(wi_tup_ty, wi_tup, 0, "ti")
            .map_err(llvm_err)?;
        let wi_tup_e = self
            .builder
            .build_struct_gep(wi_tup_ty, wi_tup, 1, "te")
            .map_err(llvm_err)?;
        self.builder.build_store(wi_tup_i, wi_i).map_err(llvm_err)?;
        self.builder
            .build_store(wi_tup_e, wi_ev)
            .map_err(llvm_err)?;
        // Wrap in fat struct tag=5 (Struct)
        let wi_fat = self.string_type.get_undef();
        let wi_fat1 = self
            .builder
            .build_insert_value(wi_fat, i64.const_int(5, false), 0, "tag")
            .map_err(llvm_err)?;
        let wi_fat2 = self
            .builder
            .build_insert_value(wi_fat1, wi_tup, 1, "data")
            .map_err(llvm_err)?;
        let wi_cur = self
            .builder
            .build_load(self.list_type, wi_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let cc2 = self.call_rt(
            "action_list_push",
            &[wi_cur.into(), wi_fat2.as_basic_value_enum().into()],
        )?;
        let wi_nv = cc2.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(wi_new_alloc, wi_nv)
            .map_err(llvm_err)?;
        let wi_ni = self
            .builder
            .build_int_add(wi_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wi_i_alloc, wi_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wi_loop);
        self.builder.position_at_end(wi_done);
        let wi_rv = self
            .builder
            .build_load(self.list_type, wi_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&wi_rv));

        // ---- action_list_unique({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let unq_fn = self.module.add_function(
            "action_list_unique",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(unq_fn, "entry");
        self.builder.position_at_end(entry);
        let unq_list = unq_fn.get_first_param().unwrap().into_struct_value();
        let unq_len = self
            .builder
            .build_extract_value(unq_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let cc3 = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let unq_new_init = cc3.try_as_basic_value().unwrap_basic().into_struct_value();
        let unq_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(unq_new_alloc, unq_new_init)
            .map_err(llvm_err)?;
        let unq_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(unq_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let unq_cache_a = self
            .builder
            .build_alloca(i8.array_type(32), "cache")
            .map_err(llvm_err)?;
        let unq_cache_i8 = self
            .builder
            .build_pointer_cast(unq_cache_a, ptr, "cache_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(unq_cache_i8, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let unq_loop = self.context.append_basic_block(unq_fn, "loop");
        let unq_body = self.context.append_basic_block(unq_fn, "body");
        let unq_done = self.context.append_basic_block(unq_fn, "done");
        let _ = self.builder.build_unconditional_branch(unq_loop);
        self.builder.position_at_end(unq_loop);
        let unq_i = self
            .builder
            .build_load(i64, unq_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let unq_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, unq_i, unq_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(unq_cond, unq_body, unq_done);
        self.builder.position_at_end(unq_body);
        let unq_get_cached_fn = self.module.get_function("action_list_get_cached").unwrap();
        let unq_ev = self
            .builder
            .build_call(
                unq_get_cached_fn,
                &[unq_list.into(), unq_i.into(), unq_cache_a.into()],
                "ev",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?
            .into_struct_value();
        let unq_cur = self
            .builder
            .build_load(self.list_type, unq_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        // Check if already in result: call action_list_contains
        let cc4 = self.call_rt(
            "action_list_contains",
            &[unq_cur.into(), unq_ev.as_basic_value_enum().into()],
        )?;
        let unq_found = cc4.try_as_basic_value().unwrap_basic().into_int_value();
        let unq_push_bb = self.context.append_basic_block(unq_fn, "push");
        let unq_skip_bb = self.context.append_basic_block(unq_fn, "skip");
        let _ = self
            .builder
            .build_conditional_branch(unq_found, unq_skip_bb, unq_push_bb);
        self.builder.position_at_end(unq_push_bb);
        let cc5 = self.call_rt(
            "action_list_push",
            &[unq_cur.into(), unq_ev.as_basic_value_enum().into()],
        )?;
        let unq_nv = cc5.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(unq_new_alloc, unq_nv)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(unq_skip_bb);
        self.builder.position_at_end(unq_skip_bb);
        let unq_ni = self
            .builder
            .build_int_add(unq_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(unq_i_alloc, unq_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(unq_loop);
        self.builder.position_at_end(unq_done);
        let unq_rv = self
            .builder
            .build_load(self.list_type, unq_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&unq_rv));

        Ok(())
    }
}
