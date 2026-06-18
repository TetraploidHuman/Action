// Submodule: builtins_range

use action_frontend::ast::*;
use inkwell::types::BasicTypeEnum;
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// range.contains(value): check if value is within the range [start, end) or [start, end]
    pub(super) fn builtin_range_contains(
        &mut self,
        range_expr: &Expr,
        val_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let range_val = self.compile_expr(range_expr)?;
        let val_val = self.compile_expr(val_expr)?;
        let (ptr, st) = match range_val {
            TypedValue::Struct(p, s) => (p, s),
            _ => return Err("range.contains requires a range value".to_string()),
        };
        let val_int = match val_val {
            TypedValue::Int(v) => v,
            _ => return Err("range.contains requires an integer argument".to_string()),
        };
        let bt: BasicTypeEnum = st.into();
        let loaded = self
            .builder
            .build_load(bt, ptr, "range_ld")
            .map_err(llvm_err)?
            .into_struct_value();
        let start = self
            .builder
            .build_extract_value(loaded, 0, "r_start")
            .map_err(llvm_err)?
            .into_int_value();
        let end = self
            .builder
            .build_extract_value(loaded, 1, "r_end")
            .map_err(llvm_err)?
            .into_int_value();
        let _inclusive = self
            .builder
            .build_extract_value(loaded, 2, "r_inc")
            .map_err(llvm_err)?
            .into_int_value();
        let ge_start = self
            .builder
            .build_int_compare(IntPredicate::SGE, val_int, start, "ge_s")
            .map_err(llvm_err)?;
        // If inclusive, use SLE; otherwise SLT
        let end_cmp = self
            .builder
            .build_int_compare(IntPredicate::SLE, val_int, end, "le_e")
            .map_err(llvm_err)?;
        let result = self
            .builder
            .build_and(ge_start, end_cmp, "in_range")
            .map_err(llvm_err)?;
        Ok(TypedValue::Bool(result))
    }

    /// range.toList(): expand the range into a List<Int> of all values
    pub(super) fn builtin_range_to_list(
        &mut self,
        range_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let range_val = self.compile_expr(range_expr)?;
        let (ptr, st) = match range_val {
            TypedValue::Struct(p, s) => (p, s),
            _ => return Err("range.toList requires a range value".to_string()),
        };
        let bt: BasicTypeEnum = st.into();
        let loaded = self
            .builder
            .build_load(bt, ptr, "range_ld")
            .map_err(llvm_err)?
            .into_struct_value();
        let start_val = self
            .builder
            .build_extract_value(loaded, 0, "r_start")
            .map_err(llvm_err)?
            .into_int_value();
        let end_val = self
            .builder
            .build_extract_value(loaded, 1, "r_end")
            .map_err(llvm_err)?
            .into_int_value();
        let inclusive = self
            .builder
            .build_extract_value(loaded, 2, "r_inc")
            .map_err(llvm_err)?
            .into_int_value();

        // end_bound = end + inclusive (for inclusive range, iterate up to and including end)
        let end_bound = self
            .builder
            .build_int_add(end_val, inclusive, "end_bound")
            .map_err(llvm_err)?;

        // Create list and store in alloca
        let cap_val = self.i64_ty().const_int(16, false);
        let list_cc = self.call_rt("action_list_create", &[cap_val.into()])?;
        let list_bv = list_cc
            .try_as_basic_value()
            .basic()
            .ok_or("range_toList create fail")?;
        let list_alloca = self
            .builder
            .build_alloca(self.list_type, "rtl_list")
            .map_err(llvm_err)?;
        self.builder
            .build_store(list_alloca, list_bv)
            .map_err(llvm_err)?;

        // Loop to populate list
        let current_fn = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let entry_block = self.builder.get_insert_block().unwrap();
        let loop_block = self.context.append_basic_block(current_fn, "rtl_loop");
        let body_block = self.context.append_basic_block(current_fn, "rtl_body");
        let done_block = self.context.append_basic_block(current_fn, "rtl_done");
        self.builder
            .build_unconditional_branch(loop_block)
            .map_err(llvm_err)?;

        // Loop header: check if i < end_bound
        self.builder.position_at_end(loop_block);
        let i_phi = self
            .builder
            .build_phi(self.i64_ty(), "rtl_i")
            .map_err(llvm_err)?;
        let list_phi = self
            .builder
            .build_phi(self.list_type, "rtl_lphi")
            .map_err(llvm_err)?;
        i_phi.add_incoming(&[(&start_val, entry_block)]);
        list_phi.add_incoming(&[(&list_bv, entry_block)]);
        let done_cond = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                i_phi.as_basic_value().into_int_value(),
                end_bound,
                "rtl_done_cond",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(done_cond, done_block, body_block)
            .map_err(llvm_err)?;

        // Loop body: push current value
        self.builder.position_at_end(body_block);
        let val_i = i_phi.as_basic_value().into_int_value();
        let fat = self.make_int_fat(val_i)?;
        let cur_list = list_phi.as_basic_value();
        let pushed = self.call_rt("action_list_push", &[cur_list.into(), fat.into()])?;
        let new_list = pushed.try_as_basic_value().basic().ok_or("rtl push fail")?;
        let next_i = self
            .builder
            .build_int_add(val_i, self.i64_ty().const_int(1, false), "rtl_next")
            .map_err(llvm_err)?;
        let body_end_block = self.builder.get_insert_block().unwrap();
        i_phi.add_incoming(&[(&next_i, body_end_block)]);
        list_phi.add_incoming(&[(&new_list, body_end_block)]);
        self.builder
            .build_unconditional_branch(loop_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(done_block);
        let final_list = list_phi.as_basic_value();
        self.builder
            .build_store(list_alloca, final_list)
            .map_err(llvm_err)?;
        Ok(TypedValue::List(list_alloca))
    }
}
