// h=0 full leaf middle insert: wrap CoW leaf in a 1-child internal, delegate to split_child, return h=1.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_insert_h0_mid(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let split_child_fn = self
            .module
            .get_function("action_list_insert_rec_split_child")
            .unwrap();

        let fn_ty = self.list_type.fn_type(
            &[self.list_type.into(), i64.into(), self.string_type.into()],
            false,
        );
        let h0_mid_fn = self
            .module
            .add_function("action_list_insert_h0_mid", fn_ty, None);

        let entry = self.context.append_basic_block(h0_mid_fn, "entry");
        let ok = self.context.append_basic_block(h0_mid_fn, "ok");
        let fail = self.context.append_basic_block(h0_mid_fn, "fail");

        self.builder.position_at_end(entry);
        let list = h0_mid_fn.get_first_param().unwrap().into_struct_value();
        let idx = h0_mid_fn.get_nth_param(1).unwrap().into_int_value();
        let elem = h0_mid_fn.get_nth_param(2).unwrap().into_struct_value();

        let leaf_node = self
            .builder
            .build_extract_value(list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();

        // Path-copy leaf so the original list binding stays isolated (CoW).
        let leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let cow_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "cow_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[cow_leaf.into(), leaf_node.into(), leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;

        let int_sz = self.internal_type.size_of().ok_or("internal size")?;
        let intl = self
            .builder
            .build_call(malloc_rc_fn, &[int_sz.into()], "intl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let intl_i8 = self
            .builder
            .build_pointer_cast(intl, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_i8, i32.const_int(1, false))
            .map_err(llvm_err)?;
        let total_p = unsafe {
            self.builder
                .build_gep(i64, intl_i8, &[one], "total_p")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(total_p, len).map_err(llvm_err)?;
        let children_base = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "cb")
                .map_err(llvm_err)?
        };
        let child0 = unsafe {
            self.builder
                .build_gep(self.child_entry_type, children_base, &[zero], "c0")
                .map_err(llvm_err)?
        };
        let c0_p = self
            .builder
            .build_pointer_cast(child0, ptr, "c0_p")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(c0_p, cow_leaf).map_err(llvm_err)?;
        let c0_st = unsafe {
            self.builder
                .build_gep(i64, c0_p, &[one], "c0_st")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(c0_st, len).map_err(llvm_err)?;
        let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[cow_leaf.into()], "")
            .map_err(llvm_err)?;

        let split_result = self
            .builder
            .build_call(
                split_child_fn,
                &[
                    intl.into(),
                    zero.into(),
                    cow_leaf.into(),
                    idx.into(),
                    elem.into(),
                    one.into(), // unique copy; not shared with old list root
                ],
                "split",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let split_ok = self
            .builder
            .build_int_compare(IntPredicate::NE, split_result, ptr.const_null(), "split_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(split_ok, ok, fail);

        self.builder.position_at_end(ok);
        let new_len = self
            .builder
            .build_int_add(len, one, "new_len")
            .map_err(llvm_err)?;
        let r0 = self
            .builder
            .build_insert_value(self.list_type.get_undef(), split_result, 0, "r0")
            .map_err(llvm_err)?;
        let r1 = self
            .builder
            .build_insert_value(r0, new_len, 1, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, one, 2, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        self.builder.position_at_end(fail);
        let _ = self.builder.build_return(Some(&list));

        Ok(())
    }
}
