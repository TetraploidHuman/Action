// Submodule: runtime_decl/define_print
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_print(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let _i8 = self.context.i8_type();
        let _malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _fmt_lb_ptr = self.make_global_str(".fmt_lb", b"[\0");
        let _fmt_rb_ptr = self.make_global_str(".fmt_rb", b"]\0");
        let _fmt_sep_ptr = self.make_global_str(".fmt_sep", b", \0");
        let str_true_ptr = self.make_global_str(".str_true", b"true\0");
        let str_false_ptr = self.make_global_str(".str_false", b"false\0");
        let fmt_task_pre_ptr = self.make_global_str(".fmt_task_pre", b"Task(done=\0");
        let fmt_task_mid_ptr = self.make_global_str(".fmt_task_mid", b", cancelled=\0");
        let fmt_task_suf_ptr = self.make_global_str(".fmt_task_suf", b")\0");
        let fmt_struct_ptr = self.make_global_str(".fmt_struct", b"<struct>\0");
        let fmt_ev_pre = self.make_global_str(".fmt_ev_pre", b"EnumVariant<\0");
        let fmt_ev_gt = self.make_global_str(".fmt_ev_gt", b">\0");
        let fmt_ev_lp = self.make_global_str(".fmt_ev_lp", b">(\0");
        let fmt_ev_rp = self.make_global_str(".fmt_ev_rp", b")\0");
        let _fmt_lb_ptr = self.make_global_str(".fmt_lb", b"[\0");
        let _fmt_sep_ptr = self.make_global_str(".fmt_sep", b", \0");
        let _fmt_rb_ptr = self.make_global_str(".fmt_rb", b"]\0");
        let _malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _list_ty2 = self.list_type;

        // Format strings for print functions
        let fmt_nl_ptr = self.make_global_str(".fmt_nl", b"\n\0");
        let fmt_int_ptr = self.make_global_str(".fmt_int", b"%ld\0");
        let fmt_float_ptr = self.make_global_str(".fmt_float", b"%g\0");
        let fmt_str_ptr = self.make_global_str(".fmt_str", b"%s\0");
        let printf_fn = self.module.get_function("printf").unwrap();

        // ---- action_print_int(i64) ----
        let print_int_fn =
            self.module
                .add_function("action_print_int", void.fn_type(&[i64.into()], false), None);
        let entry = self.context.append_basic_block(print_int_fn, "entry");
        self.builder.position_at_end(entry);
        let n = print_int_fn.get_first_param().unwrap();
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), n.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_float(double) ----
        let print_float_fn = self.module.add_function(
            "action_print_float",
            void.fn_type(&[f64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(print_float_fn, "entry");
        self.builder.position_at_end(entry);
        let n = print_float_fn.get_first_param().unwrap();
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_float_ptr.into(), n.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_bool(i1) ----
        let print_bool_fn =
            self.module
                .add_function("action_print_bool", void.fn_type(&[b1.into()], false), None);
        let entry = self.context.append_basic_block(print_bool_fn, "entry");
        let true_block = self
            .context
            .append_basic_block(print_bool_fn, "true_branch");
        let false_block = self
            .context
            .append_basic_block(print_bool_fn, "false_branch");
        self.builder.position_at_end(entry);
        let b = print_bool_fn.get_first_param().unwrap().into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(b, true_block, false_block);
        self.builder.position_at_end(true_block);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_str_ptr.into(), str_true_ptr.into()], "");
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(false_block);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_str_ptr.into(), str_false_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_string({i64, ptr}) ----
        // Handles both: String (non-null data ptr) and Int (null data ptr, value in tag)
        let print_str_fn = self.module.add_function(
            "action_print_string",
            void.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(print_str_fn, "entry");
        self.builder.position_at_end(entry);
        let s = print_str_fn.get_first_param().unwrap().into_struct_value();
        let data = self
            .builder
            .build_extract_value(s, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let is_null = self
            .builder
            .build_is_null(data, "is_null")
            .map_err(llvm_err)?;
        let str_bb = self.context.append_basic_block(print_str_fn, "print_str");
        let int_bb = self.context.append_basic_block(print_str_fn, "print_int");
        let _ = self
            .builder
            .build_conditional_branch(is_null, int_bb, str_bb);
        self.builder.position_at_end(str_bb);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_str_ptr.into(), data.into()], "");
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(int_bb);
        let tag = self
            .builder
            .build_extract_value(s, 0, "tag")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), tag.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_println() ----
        let println_fn = self
            .module
            .add_function("action_println", void.fn_type(&[], false), None);
        let entry = self.context.append_basic_block(println_fn, "entry");
        self.builder.position_at_end(entry);
        let _ = self.builder.build_call(printf_fn, &[fmt_nl_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_task({pthread: i64, done: i64, cancelled: i64, result_list: list_type}) ----
        let task_print_fn = self.module.add_function(
            "action_print_task",
            void.fn_type(&[self.task_type.into()], false),
            None,
        );
        let tp_entry = self.context.append_basic_block(task_print_fn, "entry");
        self.builder.position_at_end(tp_entry);
        let tp_task = task_print_fn.get_first_param().unwrap().into_struct_value();
        let tp_done = self
            .builder
            .build_extract_value(tp_task, 1, "done")
            .map_err(llvm_err)?;
        let tp_canc = self
            .builder
            .build_extract_value(tp_task, 2, "canc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_task_pre_ptr.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), tp_done.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_task_mid_ptr.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), tp_canc.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_task_suf_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_struct() ----
        let struct_print_fn =
            self.module
                .add_function("action_print_struct", void.fn_type(&[], false), None);
        let sp_entry = self.context.append_basic_block(struct_print_fn, "entry");
        self.builder.position_at_end(sp_entry);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_struct_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_enum({i64, ptr}) ----
        // Prints EnumVariant<tag> for nullary variants or EnumVariant<tag>(val) for data-carrying ones.
        let enum_ty = self.context.struct_type(&[i64.into(), ptr.into()], false);
        let enum_print_fn = self.module.add_function(
            "action_print_enum",
            void.fn_type(&[enum_ty.into()], false),
            None,
        );
        let ep_entry = self.context.append_basic_block(enum_print_fn, "entry");
        self.builder.position_at_end(ep_entry);
        let ep_enum = enum_print_fn.get_first_param().unwrap().into_struct_value();
        let ep_tag = self
            .builder
            .build_extract_value(ep_enum, 0, "tag")
            .map_err(llvm_err)?;
        let ep_data = self
            .builder
            .build_extract_value(ep_enum, 1, "data")
            .map_err(llvm_err)?;
        let ep_data_ptr = self
            .builder
            .build_pointer_cast(ep_data.into_pointer_value(), ptr, "vp")
            .map_err(llvm_err)?;
        let is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, ep_data_ptr, ptr.const_zero(), "is_null")
            .map_err(llvm_err)?;
        let ep_data_bb = self.context.append_basic_block(enum_print_fn, "has_data");
        let ep_no_data_bb = self.context.append_basic_block(enum_print_fn, "no_data");
        let ep_merge_bb = self.context.append_basic_block(enum_print_fn, "merge");
        let _ = self
            .builder
            .build_conditional_branch(is_null, ep_no_data_bb, ep_data_bb);
        // Has data: print EnumVariant<tag>(val)
        self.builder.position_at_end(ep_data_bb);
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_pre.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), ep_tag.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_lp.into()], "");
        let ep_val = self
            .builder
            .build_load(i64, ep_data_ptr, "val")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), ep_val.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_rp.into()], "");
        let _ = self.builder.build_unconditional_branch(ep_merge_bb);
        // No data: print EnumVariant<tag>
        self.builder.position_at_end(ep_no_data_bb);
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_pre.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), ep_tag.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_gt.into()], "");
        let _ = self.builder.build_unconditional_branch(ep_merge_bb);
        self.builder.position_at_end(ep_merge_bb);
        let _ = self.builder.build_return(None);

        // ---- action_print_enum_float({i64, ptr}) ----
        // Same as action_print_enum but loads f64 from the heap instead of i64
        let epf_fn = self.module.add_function(
            "action_print_enum_float",
            void.fn_type(&[enum_ty.into()], false),
            None,
        );
        let epf_entry = self.context.append_basic_block(epf_fn, "entry");
        self.builder.position_at_end(epf_entry);
        let epf_enum = epf_fn.get_first_param().unwrap().into_struct_value();
        let epf_tag = self
            .builder
            .build_extract_value(epf_enum, 0, "tag")
            .map_err(llvm_err)?;
        let epf_data = self
            .builder
            .build_extract_value(epf_enum, 1, "data")
            .map_err(llvm_err)?;
        let epf_data_ptr = self
            .builder
            .build_pointer_cast(epf_data.into_pointer_value(), ptr, "vpf")
            .map_err(llvm_err)?;
        let epf_is_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                epf_data_ptr,
                ptr.const_zero(),
                "is_null_f",
            )
            .map_err(llvm_err)?;
        let epf_data_bb = self.context.append_basic_block(epf_fn, "has_data");
        let epf_no_data_bb = self.context.append_basic_block(epf_fn, "no_data");
        let epf_merge_bb = self.context.append_basic_block(epf_fn, "merge");
        let _ = self
            .builder
            .build_conditional_branch(epf_is_null, epf_no_data_bb, epf_data_bb);
        // Has data: print EnumVariant<tag>(val) with float
        self.builder.position_at_end(epf_data_bb);
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_pre.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), epf_tag.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_lp.into()], "");
        let epf_val = self
            .builder
            .build_load(f64, epf_data_ptr, "valf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_float_ptr.into(), epf_val.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_rp.into()], "");
        let _ = self.builder.build_unconditional_branch(epf_merge_bb);
        // No data: print EnumVariant<tag>
        self.builder.position_at_end(epf_no_data_bb);
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_pre.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), epf_tag.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_gt.into()], "");
        let _ = self.builder.build_unconditional_branch(epf_merge_bb);
        self.builder.position_at_end(epf_merge_bb);
        let _ = self.builder.build_return(None);

        Ok(())
    }
}
