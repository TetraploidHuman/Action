// Submodule: builtins_nullable

use inkwell::values::{FloatValue, IntValue, PointerValue, StructValue};
use inkwell::IntPredicate;

use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// Build nullable String? ({i1, {i64, ptr}}): flag=0 valid(fat_struct), flag=1 null(undef)
    /// The fat string struct is inlined — no heap allocation needed.
    pub(crate) fn build_nullable_str(
        &mut self,
        fat_alloca: PointerValue<'ctx>,
        found_flag_a: PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let is_found = self
            .builder
            .build_load(self.bool_ty(), found_flag_a, "is_found")
            .map_err(llvm_err)?
            .into_int_value();
        let nullable_ty = self.get_nullable_type(self.string_type.into(), "Nullable<Str>");
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let some_bb = self.context.append_basic_block(current_fn, "nls_some");
        let none_bb = self.context.append_basic_block(current_fn, "nls_none");
        let merge_bb = self.context.append_basic_block(current_fn, "nls_merge");
        let is_found_cond = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_found,
                self.bool_ty().const_zero(),
                "is_found_cond",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_found_cond, some_bb, none_bb);
        // Some: {flag=0, fat_val} — value inlined
        self.builder.position_at_end(some_bb);
        let fat_val = self
            .builder
            .build_load(self.string_type, fat_alloca, "fat_val")
            .map_err(llvm_err)?
            .into_struct_value();
        let some_undef = nullable_ty.get_undef();
        let s1 = self
            .builder
            .build_insert_value(
                some_undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "s_flag",
            )
            .map_err(llvm_err)?;
        let some_val = self
            .builder
            .build_insert_value(s1, fat_val, 1, "s_val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // None: {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let none_undef = nullable_ty.get_undef();
        let none_val = self
            .builder
            .build_insert_value(
                none_undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "n_flag",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(nullable_ty, "nls_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);
        let alloca = self
            .builder
            .build_alloca(nullable_ty, "nls_alloca")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }

    /// Build nullable Int? ({i1, i64}): flag=0 valid(val), flag=1 null(undef)
    pub(crate) fn build_nullable_int(
        &mut self,
        val: IntValue<'ctx>,
        is_some: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let nullable_ty = self.get_nullable_type(self.i64_ty().into(), "Nullable<Int>");
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let some_bb = self.context.append_basic_block(current_fn, "nli_some");
        let none_bb = self.context.append_basic_block(current_fn, "nli_none");
        let merge_bb = self.context.append_basic_block(current_fn, "nli_merge");
        let is_some_cond = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_some,
                self.bool_ty().const_zero(),
                "is_some_cond",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_some_cond, some_bb, none_bb);
        // Some: {flag=0, val}
        self.builder.position_at_end(some_bb);
        let some_undef = nullable_ty.get_undef();
        let s1 = self
            .builder
            .build_insert_value(
                some_undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "s_flag",
            )
            .map_err(llvm_err)?;
        let some_val = self
            .builder
            .build_insert_value(s1, val, 1, "s_val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // None: {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let none_undef = nullable_ty.get_undef();
        let none_val = self
            .builder
            .build_insert_value(
                none_undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "n_flag",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(nullable_ty, "nli_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);
        let alloca = self
            .builder
            .build_alloca(nullable_ty, "nli_alloca")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }

    /// Build nullable Float? ({i1, f64}): flag=0 valid(val), flag=1 null(undef)
    pub(crate) fn build_nullable_float(
        &mut self,
        val: FloatValue<'ctx>,
        is_some: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let nullable_ty = self.get_nullable_type(self.f64_ty().into(), "Nullable<Float>");
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let some_bb = self.context.append_basic_block(current_fn, "nlf_some");
        let none_bb = self.context.append_basic_block(current_fn, "nlf_none");
        let merge_bb = self.context.append_basic_block(current_fn, "nlf_merge");
        let is_some_cond = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_some,
                self.bool_ty().const_zero(),
                "is_some_cond",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_some_cond, some_bb, none_bb);
        // Some: {flag=0, val}
        self.builder.position_at_end(some_bb);
        let some_undef = nullable_ty.get_undef();
        let s1 = self
            .builder
            .build_insert_value(
                some_undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "s_flag",
            )
            .map_err(llvm_err)?;
        let some_val = self
            .builder
            .build_insert_value(s1, val, 1, "s_val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // None: {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let none_undef = nullable_ty.get_undef();
        let none_val = self
            .builder
            .build_insert_value(
                none_undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "n_flag",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(nullable_ty, "nlf_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);
        let alloca = self
            .builder
            .build_alloca(nullable_ty, "nlf_alloca")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }

    /// Build nullable List? ({i1, list_struct}): flag=0 valid(list_val), flag=1 null(undef)
    pub(crate) fn build_nullable_list(
        &mut self,
        list_val: StructValue<'ctx>,
        is_empty_value: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let nullable_ty = self.get_nullable_type(self.list_type.into(), "Nullable<List>");
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let some_bb = self.context.append_basic_block(current_fn, "nll_some");
        let none_bb = self.context.append_basic_block(current_fn, "nll_none");
        let merge_bb = self.context.append_basic_block(current_fn, "nll_merge");
        let is_empty_cond = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_empty_value,
                self.bool_ty().const_zero(),
                "is_empty_cond",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_empty_cond, none_bb, some_bb);
        // Some: {flag=0, list_val} — value inlined, no heap allocation
        self.builder.position_at_end(some_bb);
        let some_undef = nullable_ty.get_undef();
        let s1 = self
            .builder
            .build_insert_value(
                some_undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "s_flag",
            )
            .map_err(llvm_err)?;
        let some_val = self
            .builder
            .build_insert_value(s1, list_val, 1, "s_val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // None: {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let none_undef = nullable_ty.get_undef();
        let none_val = self
            .builder
            .build_insert_value(
                none_undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "n_flag",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(nullable_ty, "nll_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);
        let alloca = self
            .builder
            .build_alloca(nullable_ty, "nll_alloca")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }
}
