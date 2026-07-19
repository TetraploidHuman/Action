// Submodule: runtime_decl/define_file_parse
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_file_parse(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let fclose_fn = self.module.get_function("fclose").unwrap();
        let fseek_fn = self.module.get_function("fseek").unwrap();
        let ftell_fn = self.module.get_function("ftell").unwrap();

        let strlen_fn = self.module.get_function("strlen").unwrap();
        let fread_fn = self.module.get_function("fread").unwrap();
        let fwrite_fn = self.module.get_function("fwrite").unwrap();

        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        // ---- action_parse_int({i64, ptr}) -> {i64, i1} (value, success) ----
        let pi_ret_ty = self
            .context
            .struct_type(&[i64.into(), self.bool_ty().into()], false);
        let pi_fn = self.module.add_function(
            "action_parse_int",
            pi_ret_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(pi_fn, "entry");
        self.builder.position_at_end(entry);
        let pi_s = pi_fn.get_first_param().unwrap().into_struct_value();
        let pi_len = self
            .builder
            .build_extract_value(pi_s, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        // Resolve slices before digit GEP (M40).
        let pi_data = {
            let str_data_fn = self.module.get_function("action_string_data").unwrap();
            self.builder
                .build_call(str_data_fn, &[pi_s.into()], "data")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value()
        };
        // Initialize result=0, sign=1, i=0, valid=0
        let pi_result = self.builder.build_alloca(i64, "result").map_err(llvm_err)?;
        let pi_sign = self.builder.build_alloca(i64, "sign").map_err(llvm_err)?;
        let pi_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        let pi_valid = self
            .builder
            .build_alloca(self.bool_ty(), "valid")
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_result, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_sign, i64.const_int(1, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_valid, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        // Check for leading '-'
        let pi_has_chars = self
            .builder
            .build_int_compare(
                IntPredicate::UGT,
                pi_len,
                i64.const_int(0, false),
                "has_chars",
            )
            .map_err(llvm_err)?;
        let pi_ck = self.context.append_basic_block(pi_fn, "check_sign");
        let pi_setup = self.context.append_basic_block(pi_fn, "setup");
        let pi_loop_hdr = self.context.append_basic_block(pi_fn, "loop_hdr");
        let pi_loop_body = self.context.append_basic_block(pi_fn, "loop_body");
        let pi_done = self.context.append_basic_block(pi_fn, "done");
        let _ = self
            .builder
            .build_conditional_branch(pi_has_chars, pi_ck, pi_done);

        // check_sign: check first char for '-', then branch to setup
        self.builder.position_at_end(pi_ck);
        let pi_first = self
            .builder
            .build_load(i8, pi_data, "first")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_is_minus = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                pi_first,
                i8.const_int(b'-' as u64, false),
                "is_minus",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pi_setup);

        // setup: set sign and start index based on whether first char is '-'
        self.builder.position_at_end(pi_setup);
        let pi_sign_val = self
            .builder
            .build_select(
                pi_is_minus,
                i64.const_int(0xffffffffffffffffu64, true),
                i64.const_int(1, false),
                "sign_val",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let pi_start_i = self
            .builder
            .build_select(
                pi_is_minus,
                i64.const_int(1, false),
                i64.const_int(0, false),
                "start_i",
            )
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(pi_sign, pi_sign_val)
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_i, pi_start_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pi_loop_hdr);

        self.builder.position_at_end(pi_loop_hdr);
        let pi_iv = self
            .builder
            .build_load(i64, pi_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_not_done = self
            .builder
            .build_int_compare(IntPredicate::ULT, pi_iv, pi_len, "not_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pi_not_done, pi_loop_body, pi_done);

        self.builder.position_at_end(pi_loop_body);
        let pi_chp = unsafe {
            self.builder
                .build_gep(i8, pi_data, &[pi_iv], "chp")
                .map_err(llvm_err)
        }?;
        let pi_ch = self
            .builder
            .build_load(i8, pi_chp, "ch")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_is_digit = self
            .builder
            .build_int_compare(
                IntPredicate::UGE,
                pi_ch,
                i8.const_int(b'0' as u64, false),
                "ge0",
            )
            .map_err(llvm_err)?;
        let pi_is_digit2 = self
            .builder
            .build_int_compare(
                IntPredicate::ULE,
                pi_ch,
                i8.const_int(b'9' as u64, false),
                "le9",
            )
            .map_err(llvm_err)?;
        let pi_is_d = self
            .builder
            .build_and(pi_is_digit, pi_is_digit2, "is_digit")
            .map_err(llvm_err)?;
        let pi_body_ck = self.context.append_basic_block(pi_fn, "body_ck");
        let pi_body_next = self.context.append_basic_block(pi_fn, "body_next");
        let _ = self
            .builder
            .build_conditional_branch(pi_is_d, pi_body_ck, pi_done);

        self.builder.position_at_end(pi_body_ck);
        let pi_cur = self
            .builder
            .build_load(i64, pi_result, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_mul = self
            .builder
            .build_int_mul(pi_cur, i64.const_int(10, false), "mul")
            .map_err(llvm_err)?;
        let pi_dval = self
            .builder
            .build_int_sub(pi_ch, i8.const_int(b'0' as u64, false), "dval")
            .map_err(llvm_err)?;
        let pi_dval64 = self
            .builder
            .build_int_z_extend(pi_dval, i64, "dval64")
            .map_err(llvm_err)?;
        let pi_add = self
            .builder
            .build_int_add(pi_mul, pi_dval64, "add")
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_result, pi_add)
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_valid, self.bool_ty().const_int(1, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pi_body_next);

        self.builder.position_at_end(pi_body_next);
        let pi_niv = self
            .builder
            .build_int_add(pi_iv, i64.const_int(1, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(pi_i, pi_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pi_loop_hdr);

        self.builder.position_at_end(pi_done);
        let pi_final = self
            .builder
            .build_load(i64, pi_result, "final")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_final_sign = self
            .builder
            .build_load(i64, pi_sign, "final_sign")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_mul_sign = self
            .builder
            .build_int_mul(pi_final, pi_final_sign, "mul_sign")
            .map_err(llvm_err)?;
        let pi_valid_val = self
            .builder
            .build_load(self.bool_ty(), pi_valid, "valid_val")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_ret_undef = pi_ret_ty.get_undef();
        let pi_ret1 = self
            .builder
            .build_insert_value(pi_ret_undef, pi_mul_sign, 0, "ret_val")
            .map_err(llvm_err)?;
        let pi_ret2 = self
            .builder
            .build_insert_value(pi_ret1, pi_valid_val, 1, "ret_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&pi_ret2));

        let host_file_write_fn = self
            .module
            .get_function("action_host_file_write")
            .ok_or("action_host_file_write not declared")?;
        let host_file_append_fn = self
            .module
            .get_function("action_host_file_append")
            .ok_or("action_host_file_append not declared")?;
        let host_file_read_fn = self
            .module
            .get_function("action_host_file_read")
            .ok_or("action_host_file_read not declared")?;
        let host_file_exists_fn = self
            .module
            .get_function("action_host_file_exists")
            .ok_or("action_host_file_exists not declared")?;
        let host_file_delete_fn = self
            .module
            .get_function("action_host_file_delete")
            .ok_or("action_host_file_delete not declared")?;
        let host_file_open_fn = self
            .module
            .get_function("action_host_file_open")
            .ok_or("action_host_file_open not declared")?;
        let str_data_fn = self
            .module
            .get_function("action_string_data")
            .ok_or("action_string_data not declared")?;

        // ---- action_read_file({i64, ptr}) -> {i64, ptr} ----
        // Resolve path slices, then host length-aware read (M40).
        let rf_fn = self.module.add_function(
            "action_read_file",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(rf_fn, "entry");
        self.builder.position_at_end(entry);
        let rf_path_s = rf_fn.get_first_param().unwrap().into_struct_value();
        let rf_path_len = self
            .builder
            .build_extract_value(rf_path_s, 0, "path_len")
            .map_err(llvm_err)?
            .into_int_value();
        let rf_path_data = self
            .builder
            .build_call(str_data_fn, &[rf_path_s.into()], "path_data")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let rf_str = self
            .builder
            .build_call(
                host_file_read_fn,
                &[rf_path_data.into(), rf_path_len.into()],
                "rf_str",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let _ = self.builder.build_return(Some(&rf_str));

        // ---- action_write_file({i64, ptr}, {i64, ptr}) -> i1 ----
        // Resolve slices via action_string_data, then host truncating write (len-aware).
        let wf_fn = self.module.add_function(
            "action_write_file",
            self.bool_ty()
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(wf_fn, "entry");
        self.builder.position_at_end(entry);
        let wf_path = wf_fn.get_first_param().unwrap().into_struct_value();
        let wf_content = wf_fn.get_nth_param(1).unwrap().into_struct_value();
        let wf_plen = self
            .builder
            .build_extract_value(wf_path, 0, "plen")
            .map_err(llvm_err)?
            .into_int_value();
        let wf_pdata = self
            .builder
            .build_call(str_data_fn, &[wf_path.into()], "pdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let wf_clen = self
            .builder
            .build_extract_value(wf_content, 0, "clen")
            .map_err(llvm_err)?
            .into_int_value();
        let wf_cdata = self
            .builder
            .build_call(str_data_fn, &[wf_content.into()], "cdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let wf_ok_i8 = self
            .builder
            .build_call(
                host_file_write_fn,
                &[
                    wf_pdata.into(),
                    wf_plen.into(),
                    wf_cdata.into(),
                    wf_clen.into(),
                ],
                "ok_i8",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let wf_ok = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                wf_ok_i8,
                self.context.i8_type().const_int(0, false),
                "ok",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&wf_ok));

        // ---- action_file_exists({i64, ptr}) -> i1 ----
        let fe_fn = self.module.add_function(
            "action_file_exists",
            self.bool_ty().fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fe_fn, "entry");
        self.builder.position_at_end(entry);
        let fe_path = fe_fn.get_first_param().unwrap().into_struct_value();
        let fe_plen = self
            .builder
            .build_extract_value(fe_path, 0, "plen")
            .map_err(llvm_err)?
            .into_int_value();
        let fe_pdata = self
            .builder
            .build_call(str_data_fn, &[fe_path.into()], "pdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fe_ok_i8 = self
            .builder
            .build_call(
                host_file_exists_fn,
                &[fe_pdata.into(), fe_plen.into()],
                "ok_i8",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fe_ok = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                fe_ok_i8,
                self.context.i8_type().const_int(0, false),
                "ok",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fe_ok));

        // ---- action_file_append({i64, ptr}, {i64, ptr}) -> i1 ----
        // Resolve slices, then host-rt cached append (action_host_file_append).
        let fa_fn = self.module.add_function(
            "action_file_append",
            self.bool_ty()
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fa_fn, "entry");
        self.builder.position_at_end(entry);
        let fa_path = fa_fn.get_first_param().unwrap().into_struct_value();
        let fa_content = fa_fn.get_nth_param(1).unwrap().into_struct_value();
        let fa_plen = self
            .builder
            .build_extract_value(fa_path, 0, "plen")
            .map_err(llvm_err)?
            .into_int_value();
        let fa_pdata = self
            .builder
            .build_call(str_data_fn, &[fa_path.into()], "pdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fa_clen = self
            .builder
            .build_extract_value(fa_content, 0, "clen")
            .map_err(llvm_err)?
            .into_int_value();
        let fa_cdata = self
            .builder
            .build_call(str_data_fn, &[fa_content.into()], "cdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fa_ok_i8 = self
            .builder
            .build_call(
                host_file_append_fn,
                &[
                    fa_pdata.into(),
                    fa_plen.into(),
                    fa_cdata.into(),
                    fa_clen.into(),
                ],
                "ok_i8",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fa_ok = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                fa_ok_i8,
                self.context.i8_type().const_int(0, false),
                "ok",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fa_ok));

        // ---- action_file_delete({i64, ptr}) -> i1 ----
        let fd_fn = self.module.add_function(
            "action_file_delete",
            self.bool_ty().fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fd_fn, "entry");
        self.builder.position_at_end(entry);
        let fd_path = fd_fn.get_first_param().unwrap().into_struct_value();
        let fd_plen = self
            .builder
            .build_extract_value(fd_path, 0, "plen")
            .map_err(llvm_err)?
            .into_int_value();
        let fd_pdata = self
            .builder
            .build_call(str_data_fn, &[fd_path.into()], "pdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fd_ok_i8 = self
            .builder
            .build_call(
                host_file_delete_fn,
                &[fd_pdata.into(), fd_plen.into()],
                "ok_i8",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fd_ok = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                fd_ok_i8,
                self.context.i8_type().const_int(0, false),
                "ok",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fd_ok));

        // ---- Streaming File I/O Runtime Functions ----

        // ---- action_file_open({i64, ptr}, {i64, ptr}) -> ptr (FILE*) ----
        // Resolve path/mode slices via action_string_data, then host fopen (len-aware).
        let fo_fn = self.module.add_function(
            "action_file_open",
            ptr.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fo_fn, "entry");
        self.builder.position_at_end(entry);
        let fo_path = fo_fn.get_first_param().unwrap().into_struct_value();
        let fo_mode = fo_fn.get_nth_param(1).unwrap().into_struct_value();
        let fo_plen = self
            .builder
            .build_extract_value(fo_path, 0, "plen")
            .map_err(llvm_err)?
            .into_int_value();
        let fo_pdata = self
            .builder
            .build_call(str_data_fn, &[fo_path.into()], "pdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fo_mlen = self
            .builder
            .build_extract_value(fo_mode, 0, "mlen")
            .map_err(llvm_err)?
            .into_int_value();
        let fo_mdata = self
            .builder
            .build_call(str_data_fn, &[fo_mode.into()], "mdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Host open also runs path-aware append barrier.
        let fo_file = self
            .builder
            .build_call(
                host_file_open_fn,
                &[
                    fo_pdata.into(),
                    fo_plen.into(),
                    fo_mdata.into(),
                    fo_mlen.into(),
                ],
                "file",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self.builder.build_return(Some(&fo_file));

        // ---- action_file_close(ptr) -> i32 ----
        // Closes a file handle. Returns 0 on success, EOF on failure.
        let fc_fn =
            self.module
                .add_function("action_file_close", i32.fn_type(&[ptr.into()], false), None);
        let entry = self.context.append_basic_block(fc_fn, "entry");
        self.builder.position_at_end(entry);
        let fc_handle = fc_fn.get_first_param().unwrap().into_pointer_value();
        let fc_ret = self
            .builder
            .build_call(fclose_fn, &[fc_handle.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&fc_ret));

        // ---- action_file_eof(ptr) -> i1 ----
        // Checks if file handle is at EOF. Uses feof().
        let feof_c_fn = self
            .module
            .add_function("feof", i32.fn_type(&[ptr.into()], false), None);
        let fe_fn = self.module.add_function(
            "action_file_eof",
            self.bool_ty().fn_type(&[ptr.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fe_fn, "entry");
        self.builder.position_at_end(entry);
        let fe_handle = fe_fn.get_first_param().unwrap().into_pointer_value();
        let fe_ret = self
            .builder
            .build_call(feof_c_fn, &[fe_handle.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fe_ok = self
            .builder
            .build_int_compare(IntPredicate::NE, fe_ret, i32.const_int(0, false), "is_eof")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fe_ok));

        // ---- action_file_read_line(ptr) -> {i64, ptr, i1} (len, data, success) ----
        // Reads one line from file handle. Returns string + success flag (0 on EOF).
        // Uses fgets with a 4096-byte buffer.
        let frl_ret_ty = self
            .context
            .struct_type(&[i64.into(), ptr.into(), self.bool_ty().into()], false);
        let frl_fn = self.module.add_function(
            "action_file_read_line",
            frl_ret_ty.fn_type(&[ptr.into()], false),
            None,
        );
        let fgets_fn = self.module.get_function("fgets").unwrap();
        let entry = self.context.append_basic_block(frl_fn, "entry");
        self.builder.position_at_end(entry);
        let frl_handle = frl_fn.get_first_param().unwrap().into_pointer_value();
        let frl_buf_size = i64.const_int(4096, false);
        let frl_buf = self
            .builder
            .build_call(malloc_rc_fn, &[frl_buf_size.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated buffer
        let frl_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(frl_buf, i64, "frl_buf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "frl_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(frl_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        let frl_ret = self
            .builder
            .build_call(
                fgets_fn,
                &[
                    frl_buf.into(),
                    i32.const_int(4096, false).into(),
                    frl_handle.into(),
                ],
                "",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Check if fgets returned NULL (EOF/error)
        let frl_is_eof = self
            .builder
            .build_int_compare(IntPredicate::EQ, frl_ret, ptr.const_zero(), "is_eof")
            .map_err(llvm_err)?;
        let frl_eof_bb = self.context.append_basic_block(frl_fn, "eof");
        let frl_ok_bb = self.context.append_basic_block(frl_fn, "ok");
        let frl_merge_bb = self.context.append_basic_block(frl_fn, "merge");
        let _ = self
            .builder
            .build_conditional_branch(frl_is_eof, frl_eof_bb, frl_ok_bb);
        // EOF path
        self.builder.position_at_end(frl_eof_bb);
        let frl_e_undef = frl_ret_ty.get_undef();
        let frl_e1 = self
            .builder
            .build_insert_value(frl_e_undef, i64.const_int(0, false), 0, "e_len")
            .map_err(llvm_err)?;
        let frl_e2 = self
            .builder
            .build_insert_value(frl_e1, ptr.const_zero(), 1, "e_ptr")
            .map_err(llvm_err)?;
        let frl_e3 = self
            .builder
            .build_insert_value(frl_e2, self.bool_ty().const_zero(), 2, "e_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(frl_merge_bb);
        // OK path: compute length, strip newline
        self.builder.position_at_end(frl_ok_bb);
        let frl_str_len = self
            .builder
            .build_call(strlen_fn, &[frl_buf.into()], "len")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let frl_last = self
            .builder
            .build_int_sub(frl_str_len, i64.const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        let frl_last_ptr = unsafe {
            self.builder
                .build_gep(i8, frl_buf, &[frl_last], "last_ptr")
                .map_err(llvm_err)
        }?;
        let frl_last_ch = self
            .builder
            .build_load(i8, frl_last_ptr, "last_ch")
            .map_err(llvm_err)?
            .into_int_value();
        let frl_is_nl = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                frl_last_ch,
                i8.const_int(10, false),
                "is_nl",
            )
            .map_err(llvm_err)?;
        // print_string for owned buffers walks to NUL — overwrite '\n' so adj_len matches.
        let frl_strip_bb = self.context.append_basic_block(frl_fn, "strip_nl");
        let frl_len_bb = self.context.append_basic_block(frl_fn, "len_done");
        let _ = self
            .builder
            .build_conditional_branch(frl_is_nl, frl_strip_bb, frl_len_bb);
        self.builder.position_at_end(frl_strip_bb);
        self.builder
            .build_store(frl_last_ptr, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(frl_len_bb);
        self.builder.position_at_end(frl_len_bb);
        let frl_adj_len = self
            .builder
            .build_select(frl_is_nl, frl_last, frl_str_len, "adj_len")
            .map_err(llvm_err)?;
        let frl_o_undef = frl_ret_ty.get_undef();
        let frl_o1 = self
            .builder
            .build_insert_value(frl_o_undef, frl_adj_len.into_int_value(), 0, "o_len")
            .map_err(llvm_err)?;
        let frl_o2 = self
            .builder
            .build_insert_value(frl_o1, frl_buf, 1, "o_ptr")
            .map_err(llvm_err)?;
        let frl_o3 = self
            .builder
            .build_insert_value(frl_o2, self.bool_ty().const_int(1, false), 2, "o_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(frl_merge_bb);
        // Merge
        self.builder.position_at_end(frl_merge_bb);
        let frl_phi = self
            .builder
            .build_phi(frl_ret_ty, "frl_ret")
            .map_err(llvm_err)?;
        frl_phi.add_incoming(&[(&frl_e3, frl_eof_bb), (&frl_o3, frl_len_bb)]);
        let _ = self.builder.build_return(Some(&frl_phi.as_basic_value()));

        // ---- action_file_read_bytes(ptr, i64) -> {i64, ptr} (actual_len, data) ----
        // Reads up to size bytes from file handle. Returns 0 length on EOF.
        let frb_ret_ty = self.context.struct_type(&[i64.into(), ptr.into()], false);
        let frb_fn = self.module.add_function(
            "action_file_read_bytes",
            frb_ret_ty.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(frb_fn, "entry");
        self.builder.position_at_end(entry);
        let frb_handle = frb_fn.get_first_param().unwrap().into_pointer_value();
        let frb_size = frb_fn.get_nth_param(1).unwrap().into_int_value();
        let frb_buf = self
            .builder
            .build_call(malloc_rc_fn, &[frb_size.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated buffer
        let frb_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(frb_buf, i64, "frb_buf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "frb_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(frb_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        let frb_read = self
            .builder
            .build_call(
                fread_fn,
                &[
                    frb_buf.into(),
                    i64.const_int(1, false).into(),
                    frb_size.into(),
                    frb_handle.into(),
                ],
                "read",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let frb_undef = frb_ret_ty.get_undef();
        let frb_r1 = self
            .builder
            .build_insert_value(frb_undef, frb_read, 0, "r_len")
            .map_err(llvm_err)?;
        let frb_r2 = self
            .builder
            .build_insert_value(frb_r1, frb_buf, 1, "r_ptr")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&frb_r2));

        // ---- action_file_write_bytes(ptr, ptr, i64) -> i1 ----
        // Writes data_len bytes from data to file. Returns true on success.
        let fwb_fn = self.module.add_function(
            "action_file_write_bytes",
            self.bool_ty()
                .fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fwb_fn, "entry");
        self.builder.position_at_end(entry);
        let fwb_handle = fwb_fn.get_first_param().unwrap().into_pointer_value();
        let fwb_data = fwb_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fwb_len = fwb_fn.get_nth_param(2).unwrap().into_int_value();
        let fwb_written = self
            .builder
            .build_call(
                fwrite_fn,
                &[
                    fwb_data.into(),
                    i64.const_int(1, false).into(),
                    fwb_len.into(),
                    fwb_handle.into(),
                ],
                "written",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fwb_ok = self
            .builder
            .build_int_compare(IntPredicate::EQ, fwb_written, fwb_len, "ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fwb_ok));

        // ---- action_file_seek(ptr, i64, i32) -> i1 ----
        // Seeks to position (offset from whence: 0=SET, 1=CUR, 2=END). Returns true on success.
        let fs_fn = self.module.add_function(
            "action_file_seek",
            self.bool_ty()
                .fn_type(&[ptr.into(), i64.into(), i32.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fs_fn, "entry");
        self.builder.position_at_end(entry);
        let fs_handle = fs_fn.get_first_param().unwrap().into_pointer_value();
        let fs_offset = fs_fn.get_nth_param(1).unwrap().into_int_value();
        let fs_whence = fs_fn.get_nth_param(2).unwrap().into_int_value();
        let fs_ret = self
            .builder
            .build_call(
                fseek_fn,
                &[fs_handle.into(), fs_offset.into(), fs_whence.into()],
                "ret",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fs_ok = self
            .builder
            .build_int_compare(IntPredicate::EQ, fs_ret, i32.const_int(0, false), "ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fs_ok));

        // ---- action_file_tell(ptr) -> i64 ----
        // Returns current file position.
        let ft_fn =
            self.module
                .add_function("action_file_tell", i64.fn_type(&[ptr.into()], false), None);
        let entry = self.context.append_basic_block(ft_fn, "entry");
        self.builder.position_at_end(entry);
        let ft_handle = ft_fn.get_first_param().unwrap().into_pointer_value();
        let ft_ret = self
            .builder
            .build_call(ftell_fn, &[ft_handle.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&ft_ret));

        // ---- action_file_flush(ptr) -> i1 ----
        // Flushes file handle. Returns true on success.
        let fflush_fn = self
            .module
            .add_function("fflush", i32.fn_type(&[ptr.into()], false), None);
        let ff_fn = self.module.add_function(
            "action_file_flush",
            self.bool_ty().fn_type(&[ptr.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(ff_fn, "entry");
        self.builder.position_at_end(entry);
        let ff_handle = ff_fn.get_first_param().unwrap().into_pointer_value();
        let ff_ret = self
            .builder
            .build_call(fflush_fn, &[ff_handle.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let ff_ok = self
            .builder
            .build_int_compare(IntPredicate::EQ, ff_ret, i32.const_int(0, false), "ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ff_ok));

        // ---- M42 bootstrap session buffers (Action String ABI → host ptr/len) ----
        let host_bs_clear = self
            .module
            .get_function("action_host_bs_buf_clear")
            .ok_or("action_host_bs_buf_clear not declared")?;
        let host_bs_append = self
            .module
            .get_function("action_host_bs_buf_append")
            .ok_or("action_host_bs_buf_append not declared")?;
        let host_bs_set = self
            .module
            .get_function("action_host_bs_buf_set")
            .ok_or("action_host_bs_buf_set not declared")?;
        let host_bs_get = self
            .module
            .get_function("action_host_bs_buf_get")
            .ok_or("action_host_bs_buf_get not declared")?;

        // action_bs_buf_clear(i64) -> i64
        let bs_clear_fn = self.module.add_function(
            "action_bs_buf_clear",
            i64.fn_type(&[i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(bs_clear_fn, "entry");
        self.builder.position_at_end(entry);
        let bs_slot = bs_clear_fn.get_first_param().unwrap().into_int_value();
        let bs_c_ret = self
            .builder
            .build_call(host_bs_clear, &[bs_slot.into()], "c")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&bs_c_ret));

        // action_bs_buf_append(i64, {i64,ptr}) -> i64
        let bs_append_fn = self.module.add_function(
            "action_bs_buf_append",
            i64.fn_type(&[i64.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(bs_append_fn, "entry");
        self.builder.position_at_end(entry);
        let bs_a_slot = bs_append_fn.get_first_param().unwrap().into_int_value();
        let bs_a_s = bs_append_fn.get_nth_param(1).unwrap().into_struct_value();
        let bs_a_len = self
            .builder
            .build_extract_value(bs_a_s, 0, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let bs_a_data = self
            .builder
            .build_call(str_data_fn, &[bs_a_s.into()], "adata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let bs_a_ret = self
            .builder
            .build_call(
                host_bs_append,
                &[bs_a_slot.into(), bs_a_data.into(), bs_a_len.into()],
                "a",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&bs_a_ret));

        // action_bs_buf_set(i64, {i64,ptr}) -> i64
        let bs_set_fn = self.module.add_function(
            "action_bs_buf_set",
            i64.fn_type(&[i64.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(bs_set_fn, "entry");
        self.builder.position_at_end(entry);
        let bs_s_slot = bs_set_fn.get_first_param().unwrap().into_int_value();
        let bs_s_s = bs_set_fn.get_nth_param(1).unwrap().into_struct_value();
        let bs_s_len = self
            .builder
            .build_extract_value(bs_s_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let bs_s_data = self
            .builder
            .build_call(str_data_fn, &[bs_s_s.into()], "sdata")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let bs_s_ret = self
            .builder
            .build_call(
                host_bs_set,
                &[bs_s_slot.into(), bs_s_data.into(), bs_s_len.into()],
                "s",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&bs_s_ret));

        // action_bs_buf_get(i64) -> {i64,ptr}
        let bs_get_fn = self.module.add_function(
            "action_bs_buf_get",
            str_ty.fn_type(&[i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(bs_get_fn, "entry");
        self.builder.position_at_end(entry);
        let bs_g_slot = bs_get_fn.get_first_param().unwrap().into_int_value();
        let bs_g_str = self
            .builder
            .build_call(host_bs_get, &[bs_g_slot.into()], "g")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let _ = self.builder.build_return(Some(&bs_g_str));

        // ---- M45 bootstrap Int session slots ----
        let host_bs_int_set = self
            .module
            .get_function("action_host_bs_int_set")
            .ok_or("action_host_bs_int_set not declared")?;
        let host_bs_int_get = self
            .module
            .get_function("action_host_bs_int_get")
            .ok_or("action_host_bs_int_get not declared")?;

        let bs_int_set_fn = self.module.add_function(
            "action_bs_int_set",
            i64.fn_type(&[i64.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(bs_int_set_fn, "entry");
        self.builder.position_at_end(entry);
        let bi_s_slot = bs_int_set_fn.get_first_param().unwrap().into_int_value();
        let bi_s_val = bs_int_set_fn.get_nth_param(1).unwrap().into_int_value();
        let bi_s_ret = self
            .builder
            .build_call(host_bs_int_set, &[bi_s_slot.into(), bi_s_val.into()], "is")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&bi_s_ret));

        let bs_int_get_fn =
            self.module
                .add_function("action_bs_int_get", i64.fn_type(&[i64.into()], false), None);
        let entry = self.context.append_basic_block(bs_int_get_fn, "entry");
        self.builder.position_at_end(entry);
        let bi_g_slot = bs_int_get_fn.get_first_param().unwrap().into_int_value();
        let bi_g_ret = self
            .builder
            .build_call(host_bs_int_get, &[bi_g_slot.into()], "ig")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&bi_g_ret));

        Ok(())
    }
}
