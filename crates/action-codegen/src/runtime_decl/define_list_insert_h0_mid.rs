// h=0 full leaf middle insert: take + push + drop + concat (same semantics as insert fallback).

use super::{llvm_err, CodeGen};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_insert_h0_mid(&self) -> Result<(), String> {
        let h0_mid_fn = self
            .module
            .get_function("action_list_insert_h0_mid")
            .unwrap();

        let entry = self.context.append_basic_block(h0_mid_fn, "entry");
        self.builder.position_at_end(entry);

        let list = h0_mid_fn.get_first_param().unwrap().into_struct_value();
        let idx = h0_mid_fn.get_nth_param(1).unwrap().into_int_value();
        let elem = h0_mid_fn.get_nth_param(2).unwrap().into_struct_value();

        let take_fn = self.module.get_function("action_list_take").unwrap();
        let drop_fn = self.module.get_function("action_list_drop").unwrap();
        let push_fn = self.module.get_function("action_list_push").unwrap();
        let concat_fn = self.module.get_function("action_list_concat").unwrap();

        let left = self
            .builder
            .build_call(take_fn, &[list.into(), idx.into()], "left")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let right = self
            .builder
            .build_call(drop_fn, &[list.into(), idx.into()], "right")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let left_with = self
            .builder
            .build_call(push_fn, &[left.into(), elem.into()], "left_with")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let result = self
            .builder
            .build_call(
                concat_fn,
                &[left_with.into(), right.into()],
                "result",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&result));

        Ok(())
    }
}
