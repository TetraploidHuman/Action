use crate::{llvm_err, CodeGen};
use inkwell::values::{BasicValue, BasicValueEnum};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn define_ht_from_list(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let ht_create = self.module.get_function("action_ht_create").unwrap();
        let ht_insert = self.module.get_function("action_ht_insert").unwrap();
        let list_len_fn = self.module.get_function("action_list_len").unwrap();
        let list_get_cached_fn = self.module.get_function("action_list_get_cached").unwrap();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let null_val: BasicValueEnum = {
            let undef = str_ty.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, zero, 0, "sn0")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(r1, self.ptr_ty().const_zero(), 1, "sn1")
                .map_err(llvm_err)?
                .as_basic_value_enum()
        };

        let f = self.module.add_function(
            "action_ht_from_list",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        let loop_bb = self.context.append_basic_block(f, "loop");
        let body = self.context.append_basic_block(f, "body");
        let done = self.context.append_basic_block(f, "done");

        self.builder.position_at_end(entry);
        let lst = f.get_first_param().unwrap().into_struct_value();
        let len_cc = self
            .builder
            .build_call(list_len_fn, &[lst.into()], "ll")
            .map_err(llvm_err)?;
        let len = len_cc.try_as_basic_value().unwrap_basic().into_int_value();
        let set_cc = self
            .builder
            .build_call(ht_create, &[len.into()], "set")
            .map_err(llvm_err)?;
        let set0 = set_cc.try_as_basic_value().unwrap_basic();
        let set_a = self
            .builder
            .build_alloca(self.list_type, "sa")
            .map_err(llvm_err)?;
        self.builder.build_store(set_a, set0).map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(i_a, zero).map_err(llvm_err)?;
        let cache_a = self
            .builder
            .build_alloca(i8.array_type(32), "cache")
            .map_err(llvm_err)?;
        let cache_i8 = self
            .builder
            .build_pointer_cast(cache_a, ptr, "cache_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(cache_i8, i8.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_bb);
        let iv = self.load_i64(i_a, "iv")?;
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, len, "cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body, done)
            .map_err(llvm_err)?;

        self.builder.position_at_end(body);
        let set_v = self
            .builder
            .build_load(self.list_type, set_a, "sv")
            .map_err(llvm_err)?
            .into_struct_value();
        let elem = self
            .builder
            .build_call(
                list_get_cached_fn,
                &[lst.into(), iv.into(), cache_a.into()],
                "el",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let ins = self
            .builder
            .build_call(
                ht_insert,
                &[set_v.into(), elem.into(), null_val.into()],
                "ins",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(set_a, ins).map_err(llvm_err)?;
        let niv = self
            .builder
            .build_int_add(iv, one, "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, niv).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(done);
        let ret = self
            .builder
            .build_load(self.list_type, set_a, "ret")
            .map_err(llvm_err)?;
        self.builder.build_return(Some(&ret)).map_err(llvm_err)?;
        Ok(())
    }
}
