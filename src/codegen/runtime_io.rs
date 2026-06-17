// Submodule: runtime_io

use inkwell::IntPredicate;

use super::{llvm_err, CodeGen};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn emit_read_line_runtime(&self) -> Result<(), String> {
        if self.module.get_function("action_read_line").is_some() {
            return Ok(());
        }
        let saved_pos = self.builder.get_insert_block();

        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();

        let strlen_fn = self.module.get_function("strlen").unwrap();

        let rl_ret_ty = self
            .context
            .struct_type(&[i64.into(), ptr.into(), self.bool_ty().into()], false);
        let rl_fn =
            self.module
                .add_function("action_read_line", rl_ret_ty.fn_type(&[], false), None);
        let fgets_fn = self.module.get_function("fgets").unwrap();
        let entry = self.context.append_basic_block(rl_fn, "entry");
        self.builder.position_at_end(entry);
        let buf_size = i64.const_int(4096, false);
        let buf = self.malloc_rc(buf_size)?;
        // Set RC=1 for newly allocated buffer (malloc_rc starts at 0)
        let rl_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(buf, i64, "rl_buf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "rl_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(rl_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        // Get stdin FILE* — platform-specific:
        //   Linux/glibc:   stdin is a global symbol exported by libc
        //   Windows/MSVC:  stdin is not a symbol; use __acrt_iob_func(0) instead
        let stdin_ptr = {
            #[cfg(target_os = "windows")]
            {
                let acrt_fn = self.module.add_function(
                    "__acrt_iob_func",
                    ptr.fn_type(&[i32.into()], false),
                    None,
                );
                self.builder
                    .build_call(acrt_fn, &[i32.const_int(0, false).into()], "stdin")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value()
            }
            #[cfg(not(target_os = "windows"))]
            {
                let stdin_g = self.add_module_global(ptr, "stdin")?;
                self.builder
                    .build_load(ptr, stdin_g.as_pointer_value(), "stdin_ptr")
                    .map_err(llvm_err)?
                    .into_pointer_value()
            }
        };
        let fgets_ret = self
            .builder
            .build_call(
                fgets_fn,
                &[
                    buf.into(),
                    i32.const_int(4096, false).into(),
                    stdin_ptr.into(),
                ],
                "",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let is_eof = self
            .builder
            .build_int_compare(IntPredicate::EQ, fgets_ret, ptr.const_zero(), "is_eof")
            .map_err(llvm_err)?;
        let eof_bb = self.context.append_basic_block(rl_fn, "eof");
        let ok_bb = self.context.append_basic_block(rl_fn, "ok");
        let merge_bb = self.context.append_basic_block(rl_fn, "merge");
        let _ = self.builder.build_conditional_branch(is_eof, eof_bb, ok_bb);
        self.builder.position_at_end(eof_bb);
        let eof_undef = rl_ret_ty.get_undef();
        let eof_r1 = self
            .builder
            .build_insert_value(eof_undef, i64.const_int(0, false), 0, "eof_len")
            .map_err(llvm_err)?;
        let eof_r2 = self
            .builder
            .build_insert_value(eof_r1, ptr.const_zero(), 1, "eof_ptr")
            .map_err(llvm_err)?;
        let eof_r3 = self
            .builder
            .build_insert_value(eof_r2, self.bool_ty().const_zero(), 2, "eof_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        self.builder.position_at_end(ok_bb);
        let str_len = self
            .builder
            .build_call(strlen_fn, &[buf.into()], "len")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let last_idx = self
            .builder
            .build_int_sub(str_len, i64.const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        let last_ptr = unsafe {
            self.builder
                .build_gep(i8, buf, &[last_idx], "last_ptr")
                .map_err(llvm_err)
        }?;
        let last_ch = self
            .builder
            .build_load(i8, last_ptr, "last_ch")
            .map_err(llvm_err)?
            .into_int_value();
        let is_nl = self
            .builder
            .build_int_compare(IntPredicate::EQ, last_ch, i8.const_int(10, false), "is_nl")
            .map_err(llvm_err)?;
        let adj_len = self
            .builder
            .build_select(is_nl, last_idx, str_len, "adj_len")
            .map_err(llvm_err)?;
        let ok_undef = rl_ret_ty.get_undef();
        let ok_r1 = self
            .builder
            .build_insert_value(ok_undef, adj_len.into_int_value(), 0, "ok_len")
            .map_err(llvm_err)?;
        let ok_r2 = self
            .builder
            .build_insert_value(ok_r1, buf, 1, "ok_ptr")
            .map_err(llvm_err)?;
        let ok_r3 = self
            .builder
            .build_insert_value(ok_r2, self.bool_ty().const_int(1, false), 2, "ok_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        self.builder.position_at_end(merge_bb);
        let rl_phi = self
            .builder
            .build_phi(rl_ret_ty, "rl_ret")
            .map_err(llvm_err)?;
        rl_phi.add_incoming(&[(&eof_r3, eof_bb), (&ok_r3, ok_bb)]);
        let _ = self.builder.build_return(Some(&rl_phi.as_basic_value()));

        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }
        Ok(())
    }

    pub(super) fn emit_read_dir_runtime(&self) -> Result<(), String> {
        if self.module.get_function("action_read_dir").is_some() {
            return Ok(());
        }
        let saved_pos = self.builder.get_insert_block();

        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let i8 = self.context.i8_type();
        let str_ty = self.string_type;
        let list_ty = self.list_type;

        let strlen_fn = self.module.get_function("strlen").unwrap();

        let rd_fn = self.module.add_function(
            "action_read_dir",
            list_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let rd_entry = self.context.append_basic_block(rd_fn, "entry");
        self.builder.position_at_end(rd_entry);
        let rd_path = rd_fn.get_first_param().unwrap().into_struct_value();
        let rd_path_data = self
            .builder
            .build_extract_value(rd_path, 1, "path_data")
            .map_err(llvm_err)?
            .into_pointer_value();

        let rd_empty = self.module.get_function("action_list_create").unwrap();
        let rd_init = self
            .builder
            .build_call(rd_empty, &[i64.const_int(0, false).into()], "rd_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();

        #[cfg(not(target_os = "windows"))]
        {
            // POSIX: opendir / readdir / closedir
            let opendir_fn =
                self.module
                    .add_function("opendir", ptr.fn_type(&[ptr.into()], false), None);
            let readdir_fn =
                self.module
                    .add_function("readdir", ptr.fn_type(&[ptr.into()], false), None);
            let closedir_fn = self.module.add_function(
                "closedir",
                self.i32_ty().fn_type(&[ptr.into()], false),
                None,
            );

            let rd_dir_ptr = self
                .builder
                .build_call(opendir_fn, &[rd_path_data.into()], "dir")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let rd_dir_null = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_ptr_to_int(rd_dir_ptr, i64, "")
                        .map_err(llvm_err)?,
                    self.builder
                        .build_ptr_to_int(ptr.const_null(), i64, "")
                        .map_err(llvm_err)?,
                    "dir_null",
                )
                .map_err(llvm_err)?;
            let rd_opendir_ok_bb = self.context.append_basic_block(rd_fn, "dir_ok");
            let rd_opendir_fail_bb = self.context.append_basic_block(rd_fn, "dir_fail");
            let rd_merge_bb = self.context.append_basic_block(rd_fn, "rd_merge");
            let _ = self.builder.build_conditional_branch(
                rd_dir_null,
                rd_opendir_fail_bb,
                rd_opendir_ok_bb,
            );
            self.builder.position_at_end(rd_opendir_ok_bb);
            let rd_cur_a = self
                .builder
                .build_alloca(list_ty, "rd_cur")
                .map_err(llvm_err)?;
            self.builder
                .build_store(rd_cur_a, rd_init)
                .map_err(llvm_err)?;
            let rd_hdr = self.context.append_basic_block(rd_fn, "rd_hdr");
            let rd_bdy = self.context.append_basic_block(rd_fn, "rd_bdy");
            let rd_done = self.context.append_basic_block(rd_fn, "rd_done");
            let _ = self.builder.build_unconditional_branch(rd_hdr);
            self.builder.position_at_end(rd_hdr);
            let rd_ent = self
                .builder
                .build_call(readdir_fn, &[rd_dir_ptr.into()], "ent")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let rd_ent_null = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_ptr_to_int(rd_ent, i64, "")
                        .map_err(llvm_err)?,
                    self.builder
                        .build_ptr_to_int(ptr.const_null(), i64, "")
                        .map_err(llvm_err)?,
                    "ent_null",
                )
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(rd_ent_null, rd_done, rd_bdy);
            self.builder.position_at_end(rd_bdy);
            let rd_name = unsafe {
                self.builder
                    .build_gep(i8, rd_ent, &[i64.const_int(19, false)], "name")
                    .map_err(llvm_err)
            }?;
            let rd_nlen = self
                .builder
                .build_call(strlen_fn, &[rd_name.into()], "nlen")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let rd_asc_fn = self.module.get_function("action_string_create").unwrap();
            let rd_new_str = self
                .builder
                .build_call(rd_asc_fn, &[rd_name.into(), rd_nlen.into()], "")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            let rd_push_fn = self.module.get_function("action_list_push").unwrap();
            let rd_cur_list = self
                .builder
                .build_load(list_ty, rd_cur_a, "rd_cur_v")
                .map_err(llvm_err)?;
            let rd_pushed = self
                .builder
                .build_call(rd_push_fn, &[rd_cur_list.into(), rd_new_str.into()], "")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            self.builder
                .build_store(rd_cur_a, rd_pushed)
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rd_hdr);
            self.builder.position_at_end(rd_done);
            let _ = self
                .builder
                .build_call(closedir_fn, &[rd_dir_ptr.into()], "")
                .map_err(llvm_err)?;
            let rd_result = self
                .builder
                .build_load(list_ty, rd_cur_a, "rd_result")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rd_merge_bb);
            self.builder.position_at_end(rd_opendir_fail_bb);
            let _ = self.builder.build_unconditional_branch(rd_merge_bb);
            self.builder.position_at_end(rd_merge_bb);
            let rd_phi = self
                .builder
                .build_phi(list_ty, "rd_phi")
                .map_err(llvm_err)?;
            rd_phi.add_incoming(&[(&rd_result, rd_done), (&rd_init, rd_opendir_fail_bb)]);
            let _ = self.builder.build_return(Some(&rd_phi.as_basic_value()));
        }

        #[cfg(target_os = "windows")]
        {
            // Windows: FindFirstFileA / FindNextFileA / FindClose
            let i32 = self.context.i32_type();
            let malloc_fn = self.module.get_function("malloc").unwrap();
            let memcpy_fn = self.module.get_function("memcpy").unwrap();
            let rd_path_len = self
                .builder
                .build_extract_value(rd_path, 0, "path_len")
                .map_err(llvm_err)?
                .into_int_value();

            let ff_fn = self.module.add_function(
                "FindFirstFileA",
                ptr.fn_type(&[ptr.into(), ptr.into()], false),
                None,
            );
            let fn_fn = self.module.add_function(
                "FindNextFileA",
                i32.fn_type(&[ptr.into(), ptr.into()], false),
                None,
            );
            let fc_fn =
                self.module
                    .add_function("FindClose", i32.fn_type(&[ptr.into()], false), None);
            // WIN32_FIND_DATAA = 320 bytes; cFileName at offset 44
            let find_data_size = i64.const_int(320, false);
            let cfile_name_offset = 44u64;

            // Build search pattern: path + "\*"
            // pattern = malloc(path_len + 3)
            let pat_len = self
                .builder
                .build_int_add(rd_path_len, i64.const_int(3, false), "pat_len")
                .map_err(llvm_err)?;
            let pat_buf = self
                .builder
                .build_call(malloc_fn, &[pat_len.into()], "pat_buf")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let _ = self
                .builder
                .build_call(
                    memcpy_fn,
                    &[pat_buf.into(), rd_path_data.into(), rd_path_len.into()],
                    "",
                )
                .map_err(llvm_err)?;
            let pat_slash = unsafe {
                self.builder
                    .build_gep(i8, pat_buf, &[rd_path_len], "pat_slash")
                    .map_err(llvm_err)
            }?;
            self.builder
                .build_store(pat_slash, i8.const_int(0x5C, false))
                .map_err(llvm_err)?;
            let pat_star = unsafe {
                self.builder
                    .build_gep(
                        i8,
                        pat_buf,
                        &[self
                            .builder
                            .build_int_add(rd_path_len, i64.const_int(1, false), "")
                            .map_err(llvm_err)?],
                        "pat_star",
                    )
                    .map_err(llvm_err)
            }?;
            self.builder
                .build_store(pat_star, i8.const_int(0x2A, false))
                .map_err(llvm_err)?;
            let pat_null = unsafe {
                self.builder
                    .build_gep(
                        i8,
                        pat_buf,
                        &[self
                            .builder
                            .build_int_add(rd_path_len, i64.const_int(2, false), "")
                            .map_err(llvm_err)?],
                        "pat_null",
                    )
                    .map_err(llvm_err)
            }?;
            self.builder
                .build_store(pat_null, i8.const_int(0, false))
                .map_err(llvm_err)?;

            // Allocate WIN32_FIND_DATAA
            let fd_ptr = self
                .builder
                .build_call(malloc_fn, &[find_data_size.into()], "fd")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();

            // FindFirstFileA(pattern, &findData)
            let h_find = self
                .builder
                .build_call(ff_fn, &[pat_buf.into(), fd_ptr.into()], "hfind")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            // INVALID_HANDLE_VALUE = -1
            let invalid_handle = self
                .builder
                .build_int_to_ptr(i64.const_int((-1i64) as u64, true), ptr, "invalid_handle")
                .map_err(llvm_err)?;
            let is_invalid = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_ptr_to_int(h_find, i64, "")
                        .map_err(llvm_err)?,
                    self.builder
                        .build_ptr_to_int(invalid_handle, i64, "")
                        .map_err(llvm_err)?,
                    "is_invalid",
                )
                .map_err(llvm_err)?;

            let ff_ok_bb = self.context.append_basic_block(rd_fn, "ff_ok");
            let ff_fail_bb = self.context.append_basic_block(rd_fn, "ff_fail");
            let rd_merge_bb = self.context.append_basic_block(rd_fn, "rd_merge");
            let _ = self
                .builder
                .build_conditional_branch(is_invalid, ff_fail_bb, ff_ok_bb);

            // ff_ok: iterate entries
            self.builder.position_at_end(ff_ok_bb);
            let rd_cur_a = self
                .builder
                .build_alloca(list_ty, "rd_cur")
                .map_err(llvm_err)?;
            self.builder
                .build_store(rd_cur_a, rd_init)
                .map_err(llvm_err)?;
            let rd_loop_hdr = self.context.append_basic_block(rd_fn, "rd_loop");
            let rd_loop_bdy = self.context.append_basic_block(rd_fn, "rd_body");
            let rd_loop_next = self.context.append_basic_block(rd_fn, "rd_next");
            let rd_done = self.context.append_basic_block(rd_fn, "rd_done");
            let _ = self.builder.build_unconditional_branch(rd_loop_hdr);

            // Loop header: extract filename from findData.cFileName
            self.builder.position_at_end(rd_loop_hdr);
            let rd_name = unsafe {
                self.builder
                    .build_gep(
                        i8,
                        fd_ptr,
                        &[i64.const_int(cfile_name_offset, false)],
                        "name",
                    )
                    .map_err(llvm_err)
            }?;
            // Skip "." and ".." entries
            let rd_name_first = self
                .builder
                .build_load(i8, rd_name, "first_char")
                .map_err(llvm_err)?
                .into_int_value();
            let is_dot = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    rd_name_first,
                    i8.const_int(0x2E, false),
                    "is_dot",
                )
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(is_dot, rd_loop_next, rd_loop_bdy);

            // rd_loop_bdy: add filename to list
            self.builder.position_at_end(rd_loop_bdy);
            let rd_nlen = self
                .builder
                .build_call(strlen_fn, &[rd_name.into()], "nlen")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let rd_asc_fn = self.module.get_function("action_string_create").unwrap();
            let rd_new_str = self
                .builder
                .build_call(rd_asc_fn, &[rd_name.into(), rd_nlen.into()], "")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            let rd_push_fn = self.module.get_function("action_list_push").unwrap();
            let rd_cur_list = self
                .builder
                .build_load(list_ty, rd_cur_a, "rd_cur_v")
                .map_err(llvm_err)?;
            let rd_pushed = self
                .builder
                .build_call(rd_push_fn, &[rd_cur_list.into(), rd_new_str.into()], "")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            self.builder
                .build_store(rd_cur_a, rd_pushed)
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rd_loop_next);

            // rd_loop_next: FindNextFileA, branch back or done
            self.builder.position_at_end(rd_loop_next);
            let has_next = self
                .builder
                .build_call(fn_fn, &[h_find.into(), fd_ptr.into()], "has_next")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let is_end = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    has_next,
                    i32.const_int(0, false),
                    "is_end",
                )
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(is_end, rd_done, rd_loop_hdr);

            // rd_done: close handle and return list
            self.builder.position_at_end(rd_done);
            let _ = self
                .builder
                .build_call(fc_fn, &[h_find.into()], "")
                .map_err(llvm_err)?;
            let rd_result = self
                .builder
                .build_load(list_ty, rd_cur_a, "rd_result")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rd_merge_bb);

            // ff_fail: return empty list
            self.builder.position_at_end(ff_fail_bb);
            let _ = self.builder.build_unconditional_branch(rd_merge_bb);

            // rd_merge: phi (result, init)
            self.builder.position_at_end(rd_merge_bb);
            let rd_phi = self
                .builder
                .build_phi(list_ty, "rd_phi")
                .map_err(llvm_err)?;
            rd_phi.add_incoming(&[(&rd_result, rd_done), (&rd_init, ff_fail_bb)]);
            let _ = self.builder.build_return(Some(&rd_phi.as_basic_value()));
        }

        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }
        Ok(())
    }
}
