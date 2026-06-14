// Submodule: runtime_decl/rc
//
// Reference counting and UTF-8 encoding runtime functions.

use super::{llvm_err, CodeGen};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_runtime_rc(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();

            // ---- RC (Reference Counting) runtime ----
            // action_rc_inc(i8* ptr): increment refcount at ptr-8. Null-safe.
            let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
            let rc_inc_entry = self.context.append_basic_block(rc_inc_fn, "entry");
            let rc_inc_do = self.context.append_basic_block(rc_inc_fn, "do_inc");
            let rc_inc_done = self.context.append_basic_block(rc_inc_fn, "done");
            self.builder.position_at_end(rc_inc_entry);
            let rc_inc_ptr = rc_inc_fn.get_first_param().unwrap().into_pointer_value();
            let rc_is_null = self
                .builder
                .build_is_null(rc_inc_ptr, "is_null")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(rc_is_null, rc_inc_done, rc_inc_do);
            self.builder.position_at_end(rc_inc_do);
            let rc_inc_i64 = self
                .builder
                .build_ptr_to_int(rc_inc_ptr, i64, "rc_i64")
                .map_err(llvm_err)?;
            let rc_inc_minus8 = self
                .builder
                .build_int_sub(rc_inc_i64, i64.const_int(8, false), "minus8")
                .map_err(llvm_err)?;
            let rc_inc_i64p = self
                .builder
                .build_int_to_ptr(rc_inc_minus8, ptr, "rc_i64p")
                .map_err(llvm_err)?;
            let rc_inc_val = self
                .builder
                .build_load(self.i64_ty(), rc_inc_i64p, "rc")
                .map_err(llvm_err)?
                .into_int_value();
            let rc_inc_new = self
                .builder
                .build_int_add(rc_inc_val, i64.const_int(1, false), "new_rc")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_store(rc_inc_i64p, rc_inc_new)
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rc_inc_done);
            self.builder.position_at_end(rc_inc_done);
            let _ = self.builder.build_return(None);

            // action_rc_dec(i8* ptr): decrement refcount at ptr-8, free if zero. Null-safe.
            let rc_dec_fn = self.module.get_function("action_rc_dec").unwrap();
            let rc_dec_entry = self.context.append_basic_block(rc_dec_fn, "entry");
            let rc_dec_null_bb = self.context.append_basic_block(rc_dec_fn, "null_check");
            let rc_dec_free_bb = self.context.append_basic_block(rc_dec_fn, "do_free");
            let rc_dec_done_bb = self.context.append_basic_block(rc_dec_fn, "done");
            self.builder.position_at_end(rc_dec_entry);
            let rc_dec_ptr = rc_dec_fn.get_first_param().unwrap().into_pointer_value();
            let rc_is_null2 = self
                .builder
                .build_is_null(rc_dec_ptr, "is_null")
                .map_err(llvm_err)?;
            let _ =
                self.builder
                    .build_conditional_branch(rc_is_null2, rc_dec_done_bb, rc_dec_null_bb);
            self.builder.position_at_end(rc_dec_null_bb);
            let rc_dec_i64 = self
                .builder
                .build_ptr_to_int(rc_dec_ptr, i64, "rc_i64")
                .map_err(llvm_err)?;
            let rc_dec_minus8 = self
                .builder
                .build_int_sub(rc_dec_i64, i64.const_int(8, false), "minus8")
                .map_err(llvm_err)?;
            let rc_dec_i64p = self
                .builder
                .build_int_to_ptr(rc_dec_minus8, ptr, "rc_i64p")
                .map_err(llvm_err)?;
            let rc_dec_val = self
                .builder
                .build_load(self.i64_ty(), rc_dec_i64p, "rc")
                .map_err(llvm_err)?
                .into_int_value();
            let rc_dec_new = self
                .builder
                .build_int_sub(rc_dec_val, i64.const_int(1, false), "new_rc")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_store(rc_dec_i64p, rc_dec_new)
                .map_err(llvm_err)?;
            let rc_is_zero = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    rc_dec_new,
                    i64.const_int(0, false),
                    "is_zero",
                )
                .map_err(llvm_err)?;
            let _ =
                self.builder
                    .build_conditional_branch(rc_is_zero, rc_dec_free_bb, rc_dec_done_bb);
            self.builder.position_at_end(rc_dec_free_bb);
            let free_func = self.module.get_function("free").unwrap();
            let rc_dec_free_ptr = self
                .builder
                .build_int_to_ptr(rc_dec_minus8, ptr, "free_ptr")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_call(free_func, &[rc_dec_free_ptr.into()], "")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rc_dec_done_bb);
            self.builder.position_at_end(rc_dec_done_bb);
            let _ = self.builder.build_return(None);

            // action_malloc_rc(i64 size) -> i8*: allocate size+8, zero rc, return ptr+8
            let malloc_rc_fn_body = self.module.get_function("action_malloc_rc").unwrap();
            let malloc_rc_entry = self.context.append_basic_block(malloc_rc_fn_body, "entry");
            self.builder.position_at_end(malloc_rc_entry);
            let malloc_rc_size = malloc_rc_fn_body
                .get_first_param()
                .unwrap()
                .into_int_value();
            let malloc_rc_total = self
                .builder
                .build_int_add(malloc_rc_size, i64.const_int(8, false), "total")
                .map_err(llvm_err)?;
            let malloc_rc_func = self.module.get_function("malloc").unwrap();
            let malloc_rc_raw = self
                .builder
                .build_call(malloc_rc_func, &[malloc_rc_total.into()], "raw")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let malloc_rc_i64p = self
                .builder
                .build_pointer_cast(malloc_rc_raw, ptr, "rc_i64p")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_store(malloc_rc_i64p, i64.const_int(0, false))
                .map_err(llvm_err)?;
            let malloc_rc_data = unsafe {
                self.builder
                    .build_gep(i8, malloc_rc_raw, &[i64.const_int(8, false)], "data")
                    .map_err(llvm_err)
            }?;
            let _ = self.builder.build_return(Some(&malloc_rc_data));

            // ---- action_rc_dec_list_node(ptr node_ptr, i64 height): recursive RC decrement for tree ----
            // height==0: leaf (elements), height>0: internal (children)
            let rdl_fn = self.module.add_function(
                "action_rc_dec_list_node",
                void.fn_type(&[ptr.into(), i64.into()], false),
                None,
            );
            let rdl_entry = self.context.append_basic_block(rdl_fn, "entry");
            let rdl_null_done = self.context.append_basic_block(rdl_fn, "null_done");
            let rdl_dec = self.context.append_basic_block(rdl_fn, "do_dec");
            let rdl_check_zero = self.context.append_basic_block(rdl_fn, "check_zero");
            let rdl_done = self.context.append_basic_block(rdl_fn, "done");
            let rdl_leaf_cleanup = self.context.append_basic_block(rdl_fn, "leaf_cleanup");
            let rdl_int_cleanup = self.context.append_basic_block(rdl_fn, "int_cleanup");
            let rdl_free_node = self.context.append_basic_block(rdl_fn, "free_node");
            let rdl_iter_body = self.context.append_basic_block(rdl_fn, "iter_body");
            let rdl_iter_next = self.context.append_basic_block(rdl_fn, "iter_next");

            // entry: null check
            self.builder.position_at_end(rdl_entry);
            let rdl_node = rdl_fn.get_first_param().unwrap().into_pointer_value();
            let rdl_height = rdl_fn.get_nth_param(1).unwrap().into_int_value();
            let rdl_is_null = self
                .builder
                .build_is_null(rdl_node, "is_null")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(rdl_is_null, rdl_null_done, rdl_dec);
            self.builder.position_at_end(rdl_null_done);
            let _ = self.builder.build_return(None);

            // do_dec: load rc at node_ptr - 8, decrement, store
            self.builder.position_at_end(rdl_dec);
            let rdl_ptr_i64 = self
                .builder
                .build_ptr_to_int(rdl_node, i64, "pi64")
                .map_err(llvm_err)?;
            let rdl_rc_addr = self
                .builder
                .build_int_sub(rdl_ptr_i64, i64.const_int(8, false), "rc_addr")
                .map_err(llvm_err)?;
            let rdl_rc_p = self
                .builder
                .build_int_to_ptr(rdl_rc_addr, ptr, "rc_p")
                .map_err(llvm_err)?;
            let rdl_rc = self
                .builder
                .build_load(i64, rdl_rc_p, "rc")
                .map_err(llvm_err)?
                .into_int_value();
            let rdl_new_rc = self
                .builder
                .build_int_sub(rdl_rc, i64.const_int(1, false), "new_rc")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_store(rdl_rc_p, rdl_new_rc)
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rdl_check_zero);

            // check_zero: if new_rc != 0, return early
            self.builder.position_at_end(rdl_check_zero);
            let rdl_is_zero = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    rdl_new_rc,
                    i64.const_int(0, false),
                    "is_zero",
                )
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(rdl_is_zero, rdl_leaf_cleanup, rdl_done);
            self.builder.position_at_end(rdl_done);
            let _ = self.builder.build_return(None);

            // leaf_cleanup: branch based on height (-1=concat, 0=leaf, >0=internal)
            self.builder.position_at_end(rdl_leaf_cleanup);
            let rdl_is_concat = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    rdl_height,
                    i64.const_int(-1i64 as u64, true),
                    "is_concat",
                )
                .map_err(llvm_err)?;
            let rdl_cleanup_normal = self.context.append_basic_block(rdl_fn, "cleanup_normal");
            let rdl_concat_cleanup = self.context.append_basic_block(rdl_fn, "concat_cleanup");
            let _ = self.builder.build_conditional_branch(
                rdl_is_concat,
                rdl_concat_cleanup,
                rdl_cleanup_normal,
            );

            // concat_cleanup: decrement RC of left and right subtrees, then free
            self.builder.position_at_end(rdl_concat_cleanup);
            // Load left list: {ptr node, i64 len, i64 height} at offset 16
            let rdll_node_ptr = unsafe {
                self.builder
                    .build_gep(i8, rdl_node, &[i64.const_int(16, false)], "rdll_np")
                    .map_err(llvm_err)
            }?;
            let rdll_node = self
                .builder
                .build_load(ptr, rdll_node_ptr, "rdll_n")
                .map_err(llvm_err)?
                .into_pointer_value();
            let rdll_h_ptr = unsafe {
                self.builder
                    .build_gep(i8, rdl_node, &[i64.const_int(32, false)], "rdll_hp")
                    .map_err(llvm_err)
            }?;
            let rdll_h = self
                .builder
                .build_load(i64, rdll_h_ptr, "rdll_h")
                .map_err(llvm_err)?
                .into_int_value();
            let _ = self
                .builder
                .build_call(rdl_fn, &[rdll_node.into(), rdll_h.into()], "")
                .map_err(llvm_err)?;
            // Load right list: at offset 40
            let rdlr_node_ptr = unsafe {
                self.builder
                    .build_gep(i8, rdl_node, &[i64.const_int(40, false)], "rdlr_np")
                    .map_err(llvm_err)
            }?;
            let rdlr_node = self
                .builder
                .build_load(ptr, rdlr_node_ptr, "rdlr_n")
                .map_err(llvm_err)?
                .into_pointer_value();
            let rdlr_h_ptr = unsafe {
                self.builder
                    .build_gep(i8, rdl_node, &[i64.const_int(56, false)], "rdlr_hp")
                    .map_err(llvm_err)
            }?;
            let rdlr_h = self
                .builder
                .build_load(i64, rdlr_h_ptr, "rdlr_h")
                .map_err(llvm_err)?
                .into_int_value();
            let _ = self
                .builder
                .build_call(rdl_fn, &[rdlr_node.into(), rdlr_h.into()], "")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rdl_free_node);

            // cleanup_normal: original logic for leaf (h=0) and internal (h>0)
            self.builder.position_at_end(rdl_cleanup_normal);
            let rdl_is_leaf = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    rdl_height,
                    i64.const_int(0, false),
                    "is_leaf",
                )
                .map_err(llvm_err)?;
            let _ = self.builder.build_conditional_branch(
                rdl_is_leaf,
                rdl_int_cleanup,
                rdl_int_cleanup,
            );
            // Both leaf and internal iterate entries at byte offset 16+i*16.
            // The pointer at that offset is: for leaf -> data ptr (call action_rc_dec),
            // for internal -> child ptr (call action_rc_dec_list_node with height-1).
            // We'll use the same iteration for both and branch on rdl_is_leaf for the action.

            // Start iteration: count at byte 0, i=0
            // This block is for both leaf and internal cleanup
            self.builder.position_at_end(rdl_int_cleanup);
            let rdl_count_raw = self
                .builder
                .build_load(i32, rdl_node, "count_raw")
                .map_err(llvm_err)?
                .into_int_value();
            let rdl_count = self
                .builder
                .build_int_z_extend(rdl_count_raw, i64, "count")
                .map_err(llvm_err)?;
            let rdl_count_zero = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    rdl_count,
                    i64.const_int(0, false),
                    "count_zero",
                )
                .map_err(llvm_err)?;
            let _ =
                self.builder
                    .build_conditional_branch(rdl_count_zero, rdl_free_node, rdl_iter_body);

            // iter_body: load entry pointer at byte 16 + i*16
            self.builder.position_at_end(rdl_iter_body);
            let rdl_phi_i = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
            rdl_phi_i.add_incoming(&[(&i64.const_int(0, false), rdl_int_cleanup)]);
            let rdl_i = rdl_phi_i.as_basic_value().into_int_value();
            let rdl_done_cond = self
                .builder
                .build_int_compare(IntPredicate::SGE, rdl_i, rdl_count, "done_cond")
                .map_err(llvm_err)?;
            let _ =
                self.builder
                    .build_conditional_branch(rdl_done_cond, rdl_free_node, rdl_iter_next);

            // iter_next: process entry i
            self.builder.position_at_end(rdl_iter_next);
            // Compute byte offset: 16 + i*16
            let rdl_i16 = self
                .builder
                .build_int_mul(rdl_i, i64.const_int(16, false), "i16")
                .map_err(llvm_err)?;
            let rdl_off = self
                .builder
                .build_int_add(i64.const_int(16, false), rdl_i16, "off")
                .map_err(llvm_err)?;
            let rdl_ep = unsafe {
                self.builder
                    .build_gep(i8, rdl_node, &[rdl_off], "ep")
                    .map_err(llvm_err)
            }?;
            let rdl_ptr = self
                .builder
                .build_load(ptr, rdl_ep, "ptr_val")
                .map_err(llvm_err)?
                .into_pointer_value();
            let rdl_ptr_nonnull = self
                .builder
                .build_is_not_null(rdl_ptr, "ptr_nonnull")
                .map_err(llvm_err)?;
            let rdl_call_skip = self.context.append_basic_block(rdl_fn, "call_skip");
            let rdl_call_do = self.context.append_basic_block(rdl_fn, "call_do");
            let _ =
                self.builder
                    .build_conditional_branch(rdl_ptr_nonnull, rdl_call_do, rdl_call_skip);

            // call_do: branch on leaf vs internal to handle the pointer correctly
            self.builder.position_at_end(rdl_call_do);
            // rdl_is_leaf from rdl_leaf_cleanup dominates this block, so use it directly
            let rdl_call_leaf = self.context.append_basic_block(rdl_fn, "call_leaf");
            let rdl_call_int = self.context.append_basic_block(rdl_fn, "call_int");
            let _ = self
                .builder
                .build_conditional_branch(rdl_is_leaf, rdl_call_leaf, rdl_call_int);

            // call_leaf: rc_dec the data pointer
            self.builder.position_at_end(rdl_call_leaf);
            let rc_dec_fn = self.module.get_function("action_rc_dec").unwrap();
            let _ = self
                .builder
                .build_call(rc_dec_fn, &[rdl_ptr.into()], "")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rdl_call_skip);

            // call_int: recurse on child node with height-1
            self.builder.position_at_end(rdl_call_int);
            let rdl_child_h = self
                .builder
                .build_int_sub(rdl_height, i64.const_int(1, false), "child_h")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_call(rdl_fn, &[rdl_ptr.into(), rdl_child_h.into()], "")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rdl_call_skip);

            // call_skip: increment i and loop back
            self.builder.position_at_end(rdl_call_skip);
            let rdl_next_i = self
                .builder
                .build_int_add(rdl_i, i64.const_int(1, false), "next_i")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rdl_iter_body);
            rdl_phi_i.add_incoming(&[(&rdl_next_i, rdl_call_skip)]);

            // iter_done and free_node: free the node
            // free_node: call free(node_ptr - 8)
            self.builder.position_at_end(rdl_free_node);
            let rdl_free_p = self
                .builder
                .build_int_to_ptr(rdl_rc_addr, ptr, "free_p")
                .map_err(llvm_err)?;
            let free_func = self.module.get_function("free").unwrap();
            let _ = self
                .builder
                .build_call(free_func, &[rdl_free_p.into()], "")
                .map_err(llvm_err)?;
            let _ = self.builder.build_return(None);

            // action_utf8_encode body: encode a Unicode code point into UTF-8 bytes

            // action_utf8_encode body: encode a Unicode code point into UTF-8 bytes
            // Takes (i64 code_point, i8* buf) -> returns i64 byte_count (1-4)
            let utf8_encode_fn_body = self.module.get_function("action_utf8_encode").unwrap();
            let utf8_entry = self
                .context
                .append_basic_block(utf8_encode_fn_body, "entry");
            let utf8_1b = self
                .context
                .append_basic_block(utf8_encode_fn_body, "one_byte");
            let utf8_2b = self
                .context
                .append_basic_block(utf8_encode_fn_body, "two_byte");
            let utf8_3b = self
                .context
                .append_basic_block(utf8_encode_fn_body, "three_byte");
            let utf8_4b = self
                .context
                .append_basic_block(utf8_encode_fn_body, "four_byte");
            self.builder.position_at_end(utf8_entry);
            let ucode = utf8_encode_fn_body
                .get_first_param()
                .unwrap()
                .into_int_value();
            let ubuf = utf8_encode_fn_body
                .get_nth_param(1)
                .unwrap()
                .into_pointer_value();
            let u0x7f = i64.const_int(0x7F, false);
            let u0x7ff = i64.const_int(0x7FF, false);
            let u0xffff = i64.const_int(0xFFFF, false);
            let is_1 = self
                .builder
                .build_int_compare(IntPredicate::ULE, ucode, u0x7f, "is1")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(is_1, utf8_1b, utf8_2b);
            // 1-byte: buf[0] = code (0x00-0x7F)
            self.builder.position_at_end(utf8_1b);
            let u1 = self
                .builder
                .build_int_truncate(ucode, i8, "u1")
                .map_err(llvm_err)?;
            let _ = self.builder.build_store(ubuf, u1).map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&i64.const_int(1, false)));
            // 2-byte check: code <= 0x7FF?
            self.builder.position_at_end(utf8_2b);
            let is_2 = self
                .builder
                .build_int_compare(IntPredicate::ULE, ucode, u0x7ff, "is2")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(is_2, utf8_3b, utf8_4b);
            // Write 2-byte: buf[0] = 0xC0 | (code >> 6); buf[1] = 0x80 | (code & 0x3F)
            self.builder.position_at_end(utf8_3b);
            let u6 = i64.const_int(6, false);
            let ucp6 = self
                .builder
                .build_right_shift(ucode, u6, false, "cp6")
                .map_err(llvm_err)?;
            let ulead2 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucp6, i8, "l2t")
                        .map_err(llvm_err)?,
                    i8.const_int(0xC0, false),
                    "lead2",
                )
                .map_err(llvm_err)?;
            let _ = self.builder.build_store(ubuf, ulead2).map_err(llvm_err)?;
            let umask = i64.const_int(0x3F, false);
            let ucont2 = self
                .builder
                .build_and(ucode, umask, "cont2")
                .map_err(llvm_err)?;
            let ub2 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucont2, i8, "c2t")
                        .map_err(llvm_err)?,
                    i8.const_int(0x80, false),
                    "b2",
                )
                .map_err(llvm_err)?;
            let ugp1 = unsafe {
                self.builder
                    .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp1")
                    .map_err(llvm_err)
            }?;
            let _ = self.builder.build_store(ugp1, ub2).map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&i64.const_int(2, false)));
            // 3-byte check: code <= 0xFFFF?
            self.builder.position_at_end(utf8_4b);
            let is_3 = self
                .builder
                .build_int_compare(IntPredicate::ULE, ucode, u0xffff, "is3")
                .map_err(llvm_err)?;
            let utf8_3b_write = self
                .context
                .append_basic_block(utf8_encode_fn_body, "three_byte_write");
            let utf8_4b_write = self
                .context
                .append_basic_block(utf8_encode_fn_body, "four_byte_write");
            let _ = self
                .builder
                .build_conditional_branch(is_3, utf8_3b_write, utf8_4b_write);
            // Write 3-byte: buf[0] = 0xE0 | (code >> 12); buf[1] = 0x80 | ((code >> 6) & 0x3F); buf[2] = 0x80 | (code & 0x3F)
            self.builder.position_at_end(utf8_3b_write);
            let u12 = i64.const_int(12, false);
            let ucp12 = self
                .builder
                .build_right_shift(ucode, u12, false, "cp12")
                .map_err(llvm_err)?;
            let ulead3 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucp12, i8, "l3t")
                        .map_err(llvm_err)?,
                    i8.const_int(0xE0, false),
                    "lead3",
                )
                .map_err(llvm_err)?;
            let _ = self.builder.build_store(ubuf, ulead3).map_err(llvm_err)?;
            let ucp6b = self
                .builder
                .build_right_shift(ucode, u6, false, "cp6b")
                .map_err(llvm_err)?;
            let ucont3_1 = self
                .builder
                .build_and(ucp6b, umask, "c3_1")
                .map_err(llvm_err)?;
            let ub3_1 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucont3_1, i8, "c3_1t")
                        .map_err(llvm_err)?,
                    i8.const_int(0x80, false),
                    "b3_1",
                )
                .map_err(llvm_err)?;
            let ugp3_1 = unsafe {
                self.builder
                    .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp3_1")
                    .map_err(llvm_err)
            }?;
            let _ = self.builder.build_store(ugp3_1, ub3_1).map_err(llvm_err)?;
            let ucont3_2 = self
                .builder
                .build_and(ucode, umask, "c3_2")
                .map_err(llvm_err)?;
            let ub3_2 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucont3_2, i8, "c3_2t")
                        .map_err(llvm_err)?,
                    i8.const_int(0x80, false),
                    "b3_2",
                )
                .map_err(llvm_err)?;
            let ugp3_2 = unsafe {
                self.builder
                    .build_gep(i8, ubuf, &[i64.const_int(2, false)], "gp3_2")
                    .map_err(llvm_err)
            }?;
            let _ = self.builder.build_store(ugp3_2, ub3_2).map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&i64.const_int(3, false)));
            // Write 4-byte: buf[0] = 0xF0 | (code >> 18); buf[1] = 0x80 | ((code >> 12) & 0x3F);
            //                buf[2] = 0x80 | ((code >> 6) & 0x3F); buf[3] = 0x80 | (code & 0x3F)
            self.builder.position_at_end(utf8_4b_write);
            let u18 = i64.const_int(18, false);
            let ucp18 = self
                .builder
                .build_right_shift(ucode, u18, false, "cp18")
                .map_err(llvm_err)?;
            let ulead4 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucp18, i8, "l4t")
                        .map_err(llvm_err)?,
                    i8.const_int(0xF0, false),
                    "lead4",
                )
                .map_err(llvm_err)?;
            let _ = self.builder.build_store(ubuf, ulead4).map_err(llvm_err)?;
            let u4_12 = i64.const_int(12, false);
            let u4_6 = i64.const_int(6, false);
            let ucp12b4 = self
                .builder
                .build_right_shift(ucode, u4_12, false, "cp12b4")
                .map_err(llvm_err)?;
            let ucont4_1 = self
                .builder
                .build_and(ucp12b4, umask, "c4_1")
                .map_err(llvm_err)?;
            let ub4_1 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucont4_1, i8, "c4_1t")
                        .map_err(llvm_err)?,
                    i8.const_int(0x80, false),
                    "b4_1",
                )
                .map_err(llvm_err)?;
            let ugp4_1 = unsafe {
                self.builder
                    .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp4_1")
                    .map_err(llvm_err)
            }?;
            let _ = self.builder.build_store(ugp4_1, ub4_1).map_err(llvm_err)?;
            let ucp6b4 = self
                .builder
                .build_right_shift(ucode, u4_6, false, "cp6b4")
                .map_err(llvm_err)?;
            let ucont4_2 = self
                .builder
                .build_and(ucp6b4, umask, "c4_2")
                .map_err(llvm_err)?;
            let ub4_2 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucont4_2, i8, "c4_2t")
                        .map_err(llvm_err)?,
                    i8.const_int(0x80, false),
                    "b4_2",
                )
                .map_err(llvm_err)?;
            let ugp4_2 = unsafe {
                self.builder
                    .build_gep(i8, ubuf, &[i64.const_int(2, false)], "gp4_2")
                    .map_err(llvm_err)
            }?;
            let _ = self.builder.build_store(ugp4_2, ub4_2).map_err(llvm_err)?;
            let ucont4_3 = self
                .builder
                .build_and(ucode, umask, "c4_3")
                .map_err(llvm_err)?;
            let ub4_3 = self
                .builder
                .build_or(
                    self.builder
                        .build_int_truncate(ucont4_3, i8, "c4_3t")
                        .map_err(llvm_err)?,
                    i8.const_int(0x80, false),
                    "b4_3",
                )
                .map_err(llvm_err)?;
            let ugp4_3 = unsafe {
                self.builder
                    .build_gep(i8, ubuf, &[i64.const_int(3, false)], "gp4_3")
                    .map_err(llvm_err)
            }?;
            let _ = self.builder.build_store(ugp4_3, ub4_3).map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&i64.const_int(4, false)));

            // action_utf8_byte_len body: determine UTF-8 byte count from leading byte
            let utf8_bl_fn = self.module.get_function("action_utf8_byte_len").unwrap();
            let bl_entry = self.context.append_basic_block(utf8_bl_fn, "entry");
            self.builder.position_at_end(bl_entry);
            let bl_byte = utf8_bl_fn.get_first_param().unwrap().into_int_value();
            let bl_byte_zext = self
                .builder
                .build_int_z_extend(bl_byte, i64, "zext")
                .map_err(llvm_err)?;
            // Check if continuation byte (10xxxxxx) → treat as 1
            let bl_80 = i64.const_int(0x80, false);
            let is_ascii = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_and(bl_byte_zext, bl_80, "and80")
                        .map_err(llvm_err)?,
                    i64.const_int(0, false),
                    "is_ascii",
                )
                .map_err(llvm_err)?;
            // Check 2-byte: (byte & 0xE0) == 0xC0
            let bl_e0 = i64.const_int(0xE0, false);
            let bl_c0 = i64.const_int(0xC0, false);
            let is_2b = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_and(bl_byte_zext, bl_e0, "andE0")
                        .map_err(llvm_err)?,
                    bl_c0,
                    "is_2b",
                )
                .map_err(llvm_err)?;
            // Check 3-byte: (byte & 0xF0) == 0xE0
            let bl_f0 = i64.const_int(0xF0, false);
            let bl_e0c = i64.const_int(0xE0, false);
            let is_3b = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_and(bl_byte_zext, bl_f0, "andF0")
                        .map_err(llvm_err)?,
                    bl_e0c,
                    "is_3b",
                )
                .map_err(llvm_err)?;
            // Check 4-byte: (byte & 0xF8) == 0xF0
            let bl_f8 = i64.const_int(0xF8, false);
            let bl_f0c = i64.const_int(0xF0, false);
            let _is_4b = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_and(bl_byte_zext, bl_f8, "andF8")
                        .map_err(llvm_err)?,
                    bl_f0c,
                    "is_4b",
                )
                .map_err(llvm_err)?;
            // Select: 3/4, 2/selected, 1/selected
            let one = i64.const_int(1, false);
            let two = i64.const_int(2, false);
            let three = i64.const_int(3, false);
            let four = i64.const_int(4, false);
            let bl_s3 = self
                .builder
                .build_select(is_3b, three, four, "s3")
                .map_err(llvm_err)?
                .into_int_value();
            let bl_s2 = self
                .builder
                .build_select(is_2b, two, bl_s3, "s2")
                .map_err(llvm_err)?
                .into_int_value();
            let bl_result = self
                .builder
                .build_select(is_ascii, one, bl_s2, "s1")
                .map_err(llvm_err)?
                .into_int_value();
            let _ = self.builder.build_return(Some(&bl_result));

            // Restore builder position
            if let Some(block) = saved_pos {
                self.builder.position_at_end(block);
            }

            Ok(())
        };

        // === Execute group closures ===
        define_print()?;
        define_str_basic()?;
        define_list_core()?;
        define_list_xform()?;
        define_str_util()?;
        define_map()?;
        define_str_extra()?;
        define_file_parse()?;
        define_rand()?;
        define_str_adv()?;
        define_list_extra()?;
        define_list_tree()?;
        define_math_ms()?;
        define_remaining()?;
        Ok(())
    }
    }
}
