//! For-loop codegen (R4-4).

use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn alloc_list_get_cache(&mut self) -> Result<PointerValue<'ctx>, String> {
        let i8 = self.context.i8_type();
        let cache = self
            .builder
            .build_alloca(i8.array_type(32), "list_get_cache")
            .map_err(llvm_err)?;
        let cache_i8 = self
            .builder
            .build_pointer_cast(
                cache,
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "cache_i8",
            )
            .map_err(llvm_err)?;
        let zero_i8 = i8.const_int(0, false);
        self.builder
            .build_store(cache_i8, zero_i8)
            .map_err(llvm_err)?;
        Ok(cache)
    }

    pub(crate) fn list_get_cached_tag(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        idx: IntValue<'ctx>,
        cache: PointerValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let loaded = self.load_list(list_ptr)?;
        let cc = self.call_rt(
            "action_list_get_cached",
            &[loaded.into(), idx.into(), cache.into()],
        )?;
        let fat_elem = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_get_cached failed")?;
        self.builder
            .build_extract_value(fat_elem.into_struct_value(), 0, "elem_tag")
            .map_err(llvm_err)
            .map(|v| v.into_int_value())
    }

    /// Like list_get_cached_tag but returns the full fat struct {tag, data}.
    pub(crate) fn list_get_cached_fat(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        idx: IntValue<'ctx>,
        cache: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let loaded = self.load_list(list_ptr)?;
        let cc = self.call_rt(
            "action_list_get_cached",
            &[loaded.into(), idx.into(), cache.into()],
        )?;
        cc.try_as_basic_value()
            .basic()
            .ok_or_else(|| "list_get_cached_fat failed".to_string())
    }

    /// `for idx < end { lst.get(idx); idx = idx + 1 }` — cached sequential walk.

    pub(crate) fn compile_sequential_list_get_loop(
        &mut self,
        list_ptr: inkwell::values::PointerValue<'ctx>,
        end_bound: inkwell::values::IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let idx_alloca = self
            .builder
            .build_alloca(i64, "seq_idx")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, zero)
            .map_err(llvm_err)?;
        let cache = self.alloc_list_get_cache()?;
        let tmp_alloca = self
            .builder
            .build_alloca(i64, "seq_tmp")
            .map_err(llvm_err)?;
        let header = self.context.append_basic_block(current_fn, "seq_hdr");
        let body_bb = self.context.append_basic_block(current_fn, "seq_body");
        let exit = self.context.append_basic_block(current_fn, "seq_exit");
        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(header);
        self.loop_control.break_target = Some(exit);
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(header);
        let cur = self
            .builder
            .build_load(i64, idx_alloca, "seq_i")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur, end_bound, "seq_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body_bb, exit)
            .map_err(llvm_err)?;
        self.builder.position_at_end(body_bb);
        let tag = self.list_get_cached_tag(list_ptr, cur, cache)?;
        self.builder
            .build_store(tmp_alloca, tag)
            .map_err(llvm_err)?;
        let next = self
            .builder
            .build_int_add(cur, one, "seq_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, next)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;
        Ok(TypedValue::Unit)
    }
}
