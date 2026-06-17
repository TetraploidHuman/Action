// Submodule: runtime_decl/define_list_core
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_core(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let _str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();

        let zero = self.i64_ty().const_int(0, false);
        let _malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let fmt_lb_ptr = self.make_global_str(".fmt_lb", b"[\0")?;
        let fmt_rb_ptr = self.make_global_str(".fmt_rb", b"]\0")?;
        let fmt_sep_ptr = self.make_global_str(".fmt_sep", b", \0")?;
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();

        let printf_fn = self.module.get_function("printf").unwrap();
        let fmt_int_ptr = self.make_global_str(".fmt_int", b"%ld\0")?;

        // ---- action_list_create(i64 cap) -> {ptr, i64, i64} ----
        // Block-based: allocates an empty leaf node (count=0). cap is ignored for compat.
        let list_create_fn = self.module.add_function(
            "action_list_create",
            self.list_type.fn_type(&[i64.into()], false),
            None,
        );
        let lc_entry = self.context.append_basic_block(list_create_fn, "entry");
        self.builder.position_at_end(lc_entry);
        // Allocate leaf node via malloc_rc — leaf type size is known at compile time
        let leaf_size = self.leaf_type.size_of().ok_or("Failed to get leaf size")?;
        let leaf_ptr = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Store count=0 at offset 0 (leaf_ptr points past RC header, at struct start)
        let lc_count_p = self
            .builder
            .build_pointer_cast(leaf_ptr, ptr, "cp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lc_count_p, i64.const_int(0, false))
            .map_err(llvm_err)?;
        // Return {node_ptr, total_len=0, height=0}
        let undef = self.list_type.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, leaf_ptr, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, zero, 1, "r2")
            .map_err(llvm_err)?;
        let r3 = self
            .builder
            .build_insert_value(r2, zero, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r3));

        // ---- action_list_push({ptr, i64, i64}, {i64, ptr}) -> {ptr, i64, i64} ----
        // Block-based B-tree push. Supports height=0 (single leaf, common case).
        // Height>0 (internal node) will be added in follow-up.
        let list_push_fn = self.module.add_function(
            "action_list_push",
            self.list_type
                .fn_type(&[self.list_type.into(), self.string_type.into()], false),
            None,
        );
        let lp_entry = self.context.append_basic_block(list_push_fn, "entry");
        let lp_concat_append = self
            .context
            .append_basic_block(list_push_fn, "concat_append");
        let lp_normal = self.context.append_basic_block(list_push_fn, "normal");
        let lp_h0 = self.context.append_basic_block(list_push_fn, "h0");
        let lp_h0_cow = self.context.append_basic_block(list_push_fn, "h0_cow");
        let lp_h0_room = self.context.append_basic_block(list_push_fn, "h0_room");
        let lp_h0_full = self.context.append_basic_block(list_push_fn, "h0_full");
        let lp_h0_done = self.context.append_basic_block(list_push_fn, "h0_done");
        let lp_hgt0 = self.context.append_basic_block(list_push_fn, "hgt0");
        self.builder.position_at_end(lp_entry);
        let list = list_push_fn.get_first_param().unwrap().into_struct_value();
        let elem = list_push_fn.get_nth_param(1).unwrap().into_struct_value();
        let node_ptr = self
            .builder
            .build_extract_value(list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let total_len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let height = self
            .builder
            .build_extract_value(list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        // Check if ConcatNode — lazy append via concat(list, singleton(elem))
        let lp_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                height,
                i64.const_int(-1i64 as u64, true),
                "lp_ic",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lp_is_concat, lp_concat_append, lp_normal);
        // ConcatNode: lazy concat append (same as insert at index == len)
        self.builder.position_at_end(lp_concat_append);
        let lp_create_fn = self.module.get_function("action_list_create").unwrap();
        let lp_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let lp_empty = self
            .builder
            .build_call(lp_create_fn, &[zero.into()], "lp_empty")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lp_sing = self
            .builder
            .build_call(list_push_fn, &[lp_empty.into(), elem.into()], "lp_sing")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lp_appended = self
            .builder
            .build_call(lp_concat_fn, &[list.into(), lp_sing.into()], "lp_appended")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lp_appended));
        // Normal (non-ConcatNode) path
        self.builder.position_at_end(lp_normal);
        let _lp_node2 = self
            .builder
            .build_extract_value(list, 0, "lp_n2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _lp_total2 = self
            .builder
            .build_extract_value(list, 1, "lp_t2")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_h2 = self
            .builder
            .build_extract_value(list, 2, "lp_h2")
            .map_err(llvm_err)?
            .into_int_value();
        let is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, lp_h2, zero, "is_h0")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(is_h0, lp_h0, lp_hgt0);

        // === Height == 0: single leaf ===
        self.builder.position_at_end(lp_h0);
        let leaf_ty = self.leaf_type;
        let leaf_size_val = leaf_ty.size_of().ok_or("leaf size")?;
        // CoW check: read rc at leaf_ptr - 8
        let node_int = self
            .builder
            .build_ptr_to_int(node_ptr, i64, "node_int")
            .map_err(llvm_err)?;
        let rc_addr = self
            .builder
            .build_int_sub(node_int, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let rc_ptr = self
            .builder
            .build_int_to_ptr(rc_addr, ptr, "rc_ptr")
            .map_err(llvm_err)?;
        let rc_val = self
            .builder
            .build_load(i64, rc_ptr, "rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let need_cow = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                rc_val,
                i64.const_int(1, false),
                "need_cow",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(need_cow, lp_h0_cow, lp_h0_room);

        // CoW: copy leaf (do NOT decrement old RC — caller scope cleanup handles that)
        self.builder.position_at_end(lp_h0_cow);
        let new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size_val.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let cow_memcpy = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                cow_memcpy,
                &[new_leaf.into(), node_ptr.into(), leaf_size_val.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_h0_room);

        // Check if leaf has room: phi for leaf pointer
        self.builder.position_at_end(lp_h0_room);
        let phi_leaf = self.builder.build_phi(ptr, "phi_leaf").map_err(llvm_err)?;
        phi_leaf.add_incoming(&[(&node_ptr, lp_h0), (&new_leaf, lp_h0_cow)]);
        let leaf = phi_leaf.as_basic_value().into_pointer_value();
        // Read count at offset 0 of leaf (i32)
        let leaf_i8 = self
            .builder
            .build_pointer_cast(leaf, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let count_raw = self
            .builder
            .build_load(i32, leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let count_load = self
            .builder
            .build_int_z_extend(count_raw, i64, "count_val")
            .map_err(llvm_err)?;
        let is_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                count_load,
                i64.const_int(64, false),
                "is_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_full, lp_h0_full, lp_h0_done);

        // Leaf is full (64 elements): split into two leaves + create internal node
        self.builder.position_at_end(lp_h0_full);
        // Allocate new leaf for second half
        let new_leaf2 = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size_val.into()], "nl2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Copy elements[32..64] from old leaf to new_leaf[0..32]
        // elements start at offset 8 in leaf struct
        let src_base = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "src_base")
                .map_err(llvm_err)
        }?;
        let src_elem32 = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    src_base,
                    &[i64.const_int(32, false)],
                    "src32",
                )
                .map_err(llvm_err)?
        };
        let nl2_i8 = self
            .builder
            .build_pointer_cast(new_leaf2, ptr, "nl2_i8")
            .map_err(llvm_err)?;
        let dst_base = unsafe {
            self.builder
                .build_gep(i8, nl2_i8, &[i64.const_int(8, false)], "dst_base")
                .map_err(llvm_err)
        }?;
        let dst_elem0 = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    dst_base,
                    &[i64.const_int(0, false)],
                    "dst0",
                )
                .map_err(llvm_err)?
        };
        let half_size = i64.const_int(32 * 16, false); // 32 elements * 16 bytes
        let _ = self
            .builder
            .build_call(
                cow_memcpy,
                &[dst_elem0.into(), src_elem32.into(), half_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Store new element at new_leaf[32]
        let nl2b = self
            .builder
            .build_pointer_cast(new_leaf2, ptr, "nl2b")
            .map_err(llvm_err)?;
        let nl2_elem_base = unsafe {
            self.builder
                .build_gep(i8, nl2b, &[i64.const_int(8, false)], "nl2_eb")
                .map_err(llvm_err)
        }?;
        let nl2_elem32 = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    nl2_elem_base,
                    &[i64.const_int(32, false)],
                    "nl2e32",
                )
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(nl2_elem32, elem)
            .map_err(llvm_err)?;
        // Set counts: old leaf = 32, new leaf = 33
        let _ = self
            .builder
            .build_store(leaf_i8, i64.const_int(32, false))
            .map_err(llvm_err)?;
        let nl2_count_p = self
            .builder
            .build_pointer_cast(new_leaf2, ptr, "nl2c")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2_count_p, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Create internal node with 2 children
        let internal_ty = self.internal_type;
        let internal_size = internal_ty.size_of().ok_or("internal size")?;
        let internal = self
            .builder
            .build_call(malloc_rc_fn, &[internal_size.into()], "intl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Store count=2, total=65
        let intl_i8 = self
            .builder
            .build_pointer_cast(internal, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_i8, i64.const_int(2, false))
            .map_err(llvm_err)?; // count at offset 0
                                 // total at offset 8 (after i32 count + i32 pad)
        let total_ptr = unsafe {
            self.builder
                .build_gep(i64, intl_i8, &[i64.const_int(1, false)], "total_p")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(total_ptr, i64.const_int(65, false))
            .map_err(llvm_err)?;
        // children array starts at offset 16 (after i32 count + i32 pad + i64 total)
        // child[0] = {leaf, 32}
        let children_base = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "children_base")
                .map_err(llvm_err)
        }?;
        let child0_ptr = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[i64.const_int(0, false)],
                    "c0",
                )
                .map_err(llvm_err)?
        };
        // child_entry = {ptr, i64} — store leaf ptr at offset 0, subtree_total at offset 8
        let c0_p = self
            .builder
            .build_pointer_cast(child0_ptr, ptr, "c0p")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(c0_p, leaf).map_err(llvm_err)?;
        let c0_t = unsafe {
            self.builder
                .build_gep(i64, c0_p, &[i64.const_int(1, false)], "c0t")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(c0_t, i64.const_int(32, false))
            .map_err(llvm_err)?;
        // child[1] = {new_leaf2, 33}
        let child1_ptr = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[i64.const_int(1, false)],
                    "c1",
                )
                .map_err(llvm_err)?
        };
        let c1_p = self
            .builder
            .build_pointer_cast(child1_ptr, ptr, "c1p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(c1_p, new_leaf2)
            .map_err(llvm_err)?;
        let c1_t = unsafe {
            self.builder
                .build_gep(i64, c1_p, &[i64.const_int(1, false)], "c1t")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(c1_t, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Increment RC of child[0] (old leaf or CoW copy) — internal node now references it
        // Without this, the caller's rc_dec on the old root frees a node still in the tree.
        let leaf_rc_ptr0 = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(leaf, i64, "leaf_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "leaf_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "leaf_rc_p0",
            )
            .map_err(llvm_err)?;
        let leaf_rc0 = self
            .builder
            .build_load(i64, leaf_rc_ptr0, "leaf_rc0")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_rc1 = self
            .builder
            .build_int_add(leaf_rc0, i64.const_int(1, false), "leaf_rc1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(leaf_rc_ptr0, leaf_rc1)
            .map_err(llvm_err)?;
        // Set RC of child[1] (new_leaf2) from 0 to 1 — internal node now references it
        let nl2_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(new_leaf2, i64, "nl2_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "nl2_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "nl2_rc_p",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Return root with internal node, height=1, new total_len
        let new_total = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_total")
            .map_err(llvm_err)?;
        let undef2 = self.list_type.get_undef();
        let sr1 = self
            .builder
            .build_insert_value(undef2, internal, 0, "sr1")
            .map_err(llvm_err)?;
        let sr2 = self
            .builder
            .build_insert_value(sr1, new_total, 1, "sr2")
            .map_err(llvm_err)?;
        let sr3 = self
            .builder
            .build_insert_value(sr2, i64.const_int(1, false), 2, "sr3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sr3));

        // Leaf has room: store element and return
        self.builder.position_at_end(lp_h0_done);
        // Store elem at elements[count]
        // GEP: leaf + 8 (skip count+pad) = elements base, then index by count_load
        let leaf_b = self
            .builder
            .build_pointer_cast(leaf, ptr, "leaf_b")
            .map_err(llvm_err)?;
        let elem_base = unsafe {
            self.builder
                .build_gep(i8, leaf_b, &[i64.const_int(8, false)], "elem_base")
                .map_err(llvm_err)
        }?;
        let elem_gep = unsafe {
            self.builder
                .build_gep(self.string_type, elem_base, &[count_load], "elem_gep")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(elem_gep, elem).map_err(llvm_err)?;
        // Increment count
        let new_count = self
            .builder
            .build_int_add(count_load, i64.const_int(1, false), "new_count")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(leaf_i8, new_count)
            .map_err(llvm_err)?;
        // Return updated root (height=0, same leaf)
        let new_total_h0 = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "nt_h0")
            .map_err(llvm_err)?;
        let undef_h0 = self.list_type.get_undef();
        let h0r1 = self
            .builder
            .build_insert_value(undef_h0, leaf, 0, "h0r1")
            .map_err(llvm_err)?;
        let h0r2 = self
            .builder
            .build_insert_value(h0r1, new_total_h0, 1, "h0r2")
            .map_err(llvm_err)?;
        let h0r3 = self
            .builder
            .build_insert_value(h0r2, zero, 2, "h0r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&h0r3));

        // === Height > 0: descend to rightmost internal node at h=1 ===
        self.builder.position_at_end(lp_hgt0);
        // Allocate variables for descent + parent tracking
        let lp_cur_node = self
            .builder
            .build_alloca(ptr, "lp_cur_node")
            .map_err(llvm_err)?;
        let lp_cur_h = self
            .builder
            .build_alloca(i64, "lp_cur_h")
            .map_err(llvm_err)?;
        let lp_parent_ptr = self
            .builder
            .build_alloca(ptr, "lp_parent_ptr")
            .map_err(llvm_err)?;
        let lp_parent_node = self
            .builder
            .build_alloca(ptr, "lp_parent_node")
            .map_err(llvm_err)?;
        let null_ptr = ptr.const_null();
        self.builder
            .build_store(lp_cur_node, node_ptr)
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_cur_h, height)
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_parent_ptr, null_ptr)
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_parent_node, null_ptr)
            .map_err(llvm_err)?;
        let lp_descend_loop = self
            .context
            .append_basic_block(list_push_fn, "descend_loop");
        let lp_descend_body = self
            .context
            .append_basic_block(list_push_fn, "descend_body");
        let lp_at_h1 = self.context.append_basic_block(list_push_fn, "at_h1");
        let _ = self.builder.build_unconditional_branch(lp_descend_loop);

        // descend_loop: iterate through internal nodes until we reach h=1
        self.builder.position_at_end(lp_descend_loop);
        let lp_ch = self
            .builder
            .build_load(i64, lp_cur_h, "ch")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_ch_gt_1 = self
            .builder
            .build_int_compare(IntPredicate::SGT, lp_ch, i64.const_int(1, false), "ch_gt_1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lp_ch_gt_1, lp_descend_body, lp_at_h1);

        // descend_body: save parent info, move to rightmost child, decrease height
        self.builder.position_at_end(lp_descend_body);
        let lp_cn = self
            .builder
            .build_load(ptr, lp_cur_node, "cn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_cn_i8 = self
            .builder
            .build_pointer_cast(lp_cn, ptr, "cn_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_parent_node, lp_cn_i8)
            .map_err(llvm_err)?;
        let lp_dcnt_raw = self
            .builder
            .build_load(i32, lp_cn_i8, "dcnt_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_dcnt = self
            .builder
            .build_int_z_extend(lp_dcnt_raw, i64, "dcnt")
            .map_err(llvm_err)?;
        let lp_dlast = self
            .builder
            .build_int_sub(lp_dcnt, i64.const_int(1, false), "dlast")
            .map_err(llvm_err)?;
        let lp_dchildren = unsafe {
            self.builder
                .build_gep(i8, lp_cn_i8, &[i64.const_int(16, false)], "dchildren")
                .map_err(llvm_err)
        }?;
        let lp_dslot = unsafe {
            self.builder
                .build_gep(self.child_entry_type, lp_dchildren, &[lp_dlast], "dslot")
                .map_err(llvm_err)
        }?;
        let lp_st_slot = unsafe {
            self.builder
                .build_gep(i64, lp_dslot, &[i64.const_int(1, false)], "st_slot")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(lp_parent_ptr, lp_st_slot)
            .map_err(llvm_err)?;
        let lp_dchild = self
            .builder
            .build_load(ptr, lp_dslot, "dchild")
            .map_err(llvm_err)?
            .into_pointer_value();
        self.builder
            .build_store(lp_cur_node, lp_dchild)
            .map_err(llvm_err)?;
        let lp_ch_new = self
            .builder
            .build_int_sub(lp_ch, i64.const_int(1, false), "ch_new")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_cur_h, lp_ch_new)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_descend_loop);

        // At h=1: internal node whose children are leaves
        self.builder.position_at_end(lp_at_h1);
        let intl_base = self
            .builder
            .build_load(ptr, lp_cur_node, "intl_base")
            .map_err(llvm_err)?
            .into_pointer_value();
        let intl_base_i8 = self
            .builder
            .build_pointer_cast(intl_base, ptr, "intl_base_i8")
            .map_err(llvm_err)?;
        let intl_count_raw = self
            .builder
            .build_load(i32, intl_base_i8, "intl_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let intl_count = self
            .builder
            .build_int_z_extend(intl_count_raw, i64, "intl_count")
            .map_err(llvm_err)?;
        // Last child index = count - 1
        let last_idx = self
            .builder
            .build_int_sub(intl_count, i64.const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        // children array at offset 16, child entry = {ptr, i64} = 16 bytes
        let children_base = unsafe {
            self.builder
                .build_gep(
                    i8,
                    intl_base_i8,
                    &[i64.const_int(16, false)],
                    "intl_children",
                )
                .map_err(llvm_err)
        }?;
        let last_child_slot = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[last_idx],
                    "last_child_slot",
                )
                .map_err(llvm_err)
        }?;
        let last_child_ptr = self
            .builder
            .build_load(ptr, last_child_slot, "last_child")
            .map_err(llvm_err)?
            .into_pointer_value();
        let subtree_total_ptr = unsafe {
            self.builder
                .build_gep(i64, last_child_slot, &[i64.const_int(1, false)], "st_ptr")
                .map_err(llvm_err)
        }?;
        let subtree_total = self
            .builder
            .build_load(i64, subtree_total_ptr, "st")
            .map_err(llvm_err)?
            .into_int_value();
        // Check RC of leaf, copy if needed
        let leaf_int = self
            .builder
            .build_ptr_to_int(last_child_ptr, i64, "leaf_int")
            .map_err(llvm_err)?;
        let leaf_rc_addr = self
            .builder
            .build_int_sub(leaf_int, i64.const_int(8, false), "leaf_rc_addr")
            .map_err(llvm_err)?;
        let leaf_rc_ptr = self
            .builder
            .build_int_to_ptr(leaf_rc_addr, ptr, "leaf_rc_ptr")
            .map_err(llvm_err)?;
        let leaf_rc = self
            .builder
            .build_load(i64, leaf_rc_ptr, "leaf_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_shared = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                leaf_rc,
                i64.const_int(1, false),
                "leaf_shared",
            )
            .map_err(llvm_err)?;
        let lp_cow_leaf = self.context.append_basic_block(list_push_fn, "lp_cow_leaf");
        let lp_leaf_ready = self
            .context
            .append_basic_block(list_push_fn, "lp_leaf_ready");
        let _ = self
            .builder
            .build_conditional_branch(leaf_shared, lp_cow_leaf, lp_leaf_ready);
        self.builder.position_at_end(lp_cow_leaf);
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let copied_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "copied_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                self.module.get_function("memcpy").unwrap(),
                &[copied_leaf.into(), last_child_ptr.into(), leaf_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Update child pointer in internal node
        let _ = self
            .builder
            .build_store(last_child_slot, copied_leaf)
            .map_err(llvm_err)?;
        // Set RC of copied_leaf to 1 — internal node now references it
        let copied_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(copied_leaf, i64, "cop_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "cop_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "cop_rc_p",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(copied_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Decrement RC of old leaf — internal node no longer references it
        let old_rc_p = self
            .builder
            .build_int_to_ptr(leaf_rc_addr, ptr, "old_rc_p")
            .map_err(llvm_err)?;
        let old_rc = self
            .builder
            .build_load(i64, old_rc_p, "old_rc_v")
            .map_err(llvm_err)?
            .into_int_value();
        let new_old_rc = self
            .builder
            .build_int_sub(old_rc, i64.const_int(1, false), "new_old_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(old_rc_p, new_old_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_leaf_ready);
        self.builder.position_at_end(lp_leaf_ready);
        let phi_leaf = self.builder.build_phi(ptr, "phi_leaf").map_err(llvm_err)?;
        phi_leaf.add_incoming(&[(&last_child_ptr, lp_at_h1), (&copied_leaf, lp_cow_leaf)]);
        let target_leaf = phi_leaf.as_basic_value().into_pointer_value();
        // Read leaf count (i32)
        let leaf_bytes = self
            .builder
            .build_pointer_cast(target_leaf, ptr, "leaf_bytes")
            .map_err(llvm_err)?;
        let leaf_count_raw = self
            .builder
            .build_load(i32, leaf_bytes, "leaf_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_count = self
            .builder
            .build_int_z_extend(leaf_count_raw, i64, "leaf_count")
            .map_err(llvm_err)?;
        let elem_base_x = unsafe {
            self.builder
                .build_gep(i8, leaf_bytes, &[i64.const_int(8, false)], "elem_base")
                .map_err(llvm_err)
        }?;
        let intl_total_ptr = unsafe {
            self.builder
                .build_gep(i64, intl_base_i8, &[i64.const_int(1, false)], "intl_total")
                .map_err(llvm_err)
        }?;
        let intl_old_total = self
            .builder
            .build_load(i64, intl_total_ptr, "old_total")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                leaf_count,
                i64.const_int(64, false),
                "leaf_full",
            )
            .map_err(llvm_err)?;
        let lp_store_leaf = self
            .context
            .append_basic_block(list_push_fn, "lp_store_leaf");
        let lp_split_leaf = self
            .context
            .append_basic_block(list_push_fn, "lp_split_leaf");
        let _ = self
            .builder
            .build_conditional_branch(leaf_full, lp_split_leaf, lp_store_leaf);
        // Store element in leaf (has room)
        self.builder.position_at_end(lp_store_leaf);
        let elem_slot = unsafe {
            self.builder
                .build_gep(self.string_type, elem_base_x, &[leaf_count], "elem_slot")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(elem_slot, elem)
            .map_err(llvm_err)?;
        let new_leaf_count = self
            .builder
            .build_int_add(leaf_count, i64.const_int(1, false), "new_lc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(leaf_bytes, new_leaf_count)
            .map_err(llvm_err)?;
        // Update subtree_total
        let new_st = self
            .builder
            .build_int_add(subtree_total, i64.const_int(1, false), "new_st")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(subtree_total_ptr, new_st)
            .map_err(llvm_err)?;
        // Update internal total
        let intl_new_total = self
            .builder
            .build_int_add(intl_old_total, i64.const_int(1, false), "new_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_total_ptr, intl_new_total)
            .map_err(llvm_err)?;
        // Update parent if we descended from height > 1
        let lp_st_slot_val = self
            .builder
            .build_load(ptr, lp_parent_ptr, "st_slot_val")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_has_parent = self
            .builder
            .build_int_compare(IntPredicate::NE, lp_st_slot_val, null_ptr, "has_parent")
            .map_err(llvm_err)?;
        let lp_do_parent = self
            .context
            .append_basic_block(list_push_fn, "lp_do_parent");
        let lp_parent_done = self
            .context
            .append_basic_block(list_push_fn, "lp_parent_done");
        let _ = self
            .builder
            .build_conditional_branch(lp_has_parent, lp_do_parent, lp_parent_done);
        self.builder.position_at_end(lp_do_parent);
        let st_cur = self
            .builder
            .build_load(i64, lp_st_slot_val, "st_cur")
            .map_err(llvm_err)?
            .into_int_value();
        let st_new = self
            .builder
            .build_int_add(st_cur, i64.const_int(1, false), "st_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lp_st_slot_val, st_new)
            .map_err(llvm_err)?;
        let pn_val = self
            .builder
            .build_load(ptr, lp_parent_node, "pn_val")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pn_tp = unsafe {
            self.builder
                .build_gep(i64, pn_val, &[i64.const_int(1, false)], "pn_tp")
                .map_err(llvm_err)
        }?;
        let pn_tot = self
            .builder
            .build_load(i64, pn_tp, "pn_tot")
            .map_err(llvm_err)?
            .into_int_value();
        let pn_tot_new = self
            .builder
            .build_int_add(pn_tot, i64.const_int(1, false), "pn_tot_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pn_tp, pn_tot_new)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_parent_done);
        self.builder.position_at_end(lp_parent_done);
        let new_list_len = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_len")
            .map_err(llvm_err)?;
        let undef_hgt0 = self.list_type.get_undef();
        let r_hgt0_1 = self
            .builder
            .build_insert_value(undef_hgt0, node_ptr, 0, "r1")
            .map_err(llvm_err)?;
        let r_hgt0_2 = self
            .builder
            .build_insert_value(r_hgt0_1, new_list_len, 1, "r2")
            .map_err(llvm_err)?;
        let r_hgt0_3 = self
            .builder
            .build_insert_value(r_hgt0_2, height, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r_hgt0_3));
        // Leaf full: split rightmost leaf, handle internal overflow by creating new root
        self.builder.position_at_end(lp_split_leaf);
        let leaf_size_val2 = leaf_ty.size_of().ok_or("leaf size2")?;
        let new_leaf2_gt = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size_val2.into()], "nl2_gt")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Copy elements[32..64] to new leaf
        let src_elem32_gt = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    elem_base_x,
                    &[i64.const_int(32, false)],
                    "src32_gt",
                )
                .map_err(llvm_err)
        }?;
        let nl2_bytes = self
            .builder
            .build_pointer_cast(new_leaf2_gt, ptr, "nl2_bytes")
            .map_err(llvm_err)?;
        let dst_elem_base = unsafe {
            self.builder
                .build_gep(i8, nl2_bytes, &[i64.const_int(8, false)], "dst_base_gt")
                .map_err(llvm_err)
        }?;
        let dst_elem0_gt = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    dst_elem_base,
                    &[i64.const_int(0, false)],
                    "dst0_gt",
                )
                .map_err(llvm_err)
        }?;
        let half_sz = i64.const_int(32 * 16, false);
        let _ = self
            .builder
            .build_call(
                self.module.get_function("memcpy").unwrap(),
                &[dst_elem0_gt.into(), src_elem32_gt.into(), half_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Store new element at new_leaf[32]
        let nl2_elem32_gt = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    dst_elem_base,
                    &[i64.const_int(32, false)],
                    "nl2e32_gt",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nl2_elem32_gt, elem)
            .map_err(llvm_err)?;
        // Set counts
        let _ = self
            .builder
            .build_store(leaf_bytes, i64.const_int(32, false))
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2_bytes, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Update original child's subtree_total to 32
        let _ = self
            .builder
            .build_store(subtree_total_ptr, i64.const_int(32, false))
            .map_err(llvm_err)?;
        // Set RC of new_leaf2_gt to 1
        let nl2g_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(new_leaf2_gt, i64, "nl2g_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "nl2g_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "nl2g_rc_p",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2g_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Check if internal node is full (count >= 64)
        let intl_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                intl_count,
                i64.const_int(64, false),
                "intl_full",
            )
            .map_err(llvm_err)?;
        let lp_add_child = self
            .context
            .append_basic_block(list_push_fn, "lp_add_child");
        let lp_split_intl = self
            .context
            .append_basic_block(list_push_fn, "lp_split_intl");
        let _ = self
            .builder
            .build_conditional_branch(intl_full, lp_split_intl, lp_add_child);

        // Internal node has room: add new child normally
        self.builder.position_at_end(lp_add_child);
        let new_child_idx = intl_count;
        let new_child_slot = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[new_child_idx],
                    "new_child",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_child_slot, new_leaf2_gt)
            .map_err(llvm_err)?;
        let nc_st_ptr = unsafe {
            self.builder
                .build_gep(i64, new_child_slot, &[i64.const_int(1, false)], "nc_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nc_st_ptr, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // RC-inc new_leaf2_gt (internal node now references it, one more reference)
        let nl2g_rc2 = self
            .builder
            .build_load(i64, nl2g_rc_ptr, "nl2g_rc2")
            .map_err(llvm_err)?
            .into_int_value();
        let nl2g_rc3 = self
            .builder
            .build_int_add(nl2g_rc2, i64.const_int(1, false), "nl2g_rc3")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2g_rc_ptr, nl2g_rc3)
            .map_err(llvm_err)?;
        let new_intl_count = self
            .builder
            .build_int_add(intl_count, i64.const_int(1, false), "new_intl_count")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_base_i8, new_intl_count)
            .map_err(llvm_err)?;
        // Update internal total
        let new_intl_total = self
            .builder
            .build_int_add(intl_old_total, i64.const_int(1, false), "new_intl_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_total_ptr, new_intl_total)
            .map_err(llvm_err)?;
        // Update parent if we descended from height > 1
        let lp_st_slot_val2 = self
            .builder
            .build_load(ptr, lp_parent_ptr, "st_slot_val2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_has_parent2 = self
            .builder
            .build_int_compare(IntPredicate::NE, lp_st_slot_val2, null_ptr, "has_parent2")
            .map_err(llvm_err)?;
        let lp_do_parent2 = self
            .context
            .append_basic_block(list_push_fn, "lp_do_parent2");
        let lp_parent_done2 = self
            .context
            .append_basic_block(list_push_fn, "lp_parent_done2");
        let _ =
            self.builder
                .build_conditional_branch(lp_has_parent2, lp_do_parent2, lp_parent_done2);
        self.builder.position_at_end(lp_do_parent2);
        let st_cur2 = self
            .builder
            .build_load(i64, lp_st_slot_val2, "st_cur2")
            .map_err(llvm_err)?
            .into_int_value();
        let st_new2 = self
            .builder
            .build_int_add(st_cur2, i64.const_int(1, false), "st_new2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lp_st_slot_val2, st_new2)
            .map_err(llvm_err)?;
        let pn_val2 = self
            .builder
            .build_load(ptr, lp_parent_node, "pn_val2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pn_tp2 = unsafe {
            self.builder
                .build_gep(i64, pn_val2, &[i64.const_int(1, false)], "pn_tp2")
                .map_err(llvm_err)
        }?;
        let pn_tot2 = self
            .builder
            .build_load(i64, pn_tp2, "pn_tot2")
            .map_err(llvm_err)?
            .into_int_value();
        let pn_tot_new2 = self
            .builder
            .build_int_add(pn_tot2, i64.const_int(1, false), "pn_tot_new2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pn_tp2, pn_tot_new2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_parent_done2);
        self.builder.position_at_end(lp_parent_done2);
        let new_list_len2 = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_len2")
            .map_err(llvm_err)?;
        let undef_hgt0b = self.list_type.get_undef();
        let r_hgt0b_1 = self
            .builder
            .build_insert_value(undef_hgt0b, node_ptr, 0, "rb1")
            .map_err(llvm_err)?;
        let r_hgt0b_2 = self
            .builder
            .build_insert_value(r_hgt0b_1, new_list_len2, 1, "rb2")
            .map_err(llvm_err)?;
        let r_hgt0b_3 = self
            .builder
            .build_insert_value(r_hgt0b_2, height, 2, "rb3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r_hgt0b_3));

        // Internal node is full: create new internal sibling or new root
        self.builder.position_at_end(lp_split_intl);
        // The rightmost leaf's subtree_total changed from subtree_total to 32.
        // Fix intl_base's total: intl_old_total - subtree_total + 32
        let thirty2 = i64.const_int(32, false);
        let intl_st_delta = self
            .builder
            .build_int_sub(subtree_total, thirty2, "st_delta")
            .map_err(llvm_err)?;
        let intl_corrected_total = self
            .builder
            .build_int_sub(intl_old_total, intl_st_delta, "corrected_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_total_ptr, intl_corrected_total)
            .map_err(llvm_err)?;
        // Allocate new internal node for the split-off right side
        let internal_size = self.internal_type.size_of().ok_or("internal size")?;
        let new_intl = self
            .builder
            .build_call(malloc_rc_fn, &[internal_size.into()], "new_intl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let new_intl_i8 = self
            .builder
            .build_pointer_cast(new_intl, ptr, "new_intl_i8")
            .map_err(llvm_err)?;
        // Set new_intl count = 1
        let _ = self
            .builder
            .build_store(new_intl_i8, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Set new_intl total = 33
        let new_intl_tp = unsafe {
            self.builder
                .build_gep(i64, new_intl_i8, &[i64.const_int(1, false)], "nitp")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_intl_tp, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Set new_intl children[0] = {new_leaf2_gt, 33}
        let new_intl_cbase = unsafe {
            self.builder
                .build_gep(i8, new_intl_i8, &[i64.const_int(16, false)], "nicbase")
                .map_err(llvm_err)
        }?;
        let new_intl_c0 = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    new_intl_cbase,
                    &[i64.const_int(0, false)],
                    "nic0",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_intl_c0, new_leaf2_gt)
            .map_err(llvm_err)?;
        let nic0_st = unsafe {
            self.builder
                .build_gep(i64, new_intl_c0, &[i64.const_int(1, false)], "nic0_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nic0_st, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // RC-inc new_leaf2_gt once more (new internal node references it)
        let nl2g_rc_v = self
            .builder
            .build_load(i64, nl2g_rc_ptr, "nl2g_rc_v")
            .map_err(llvm_err)?
            .into_int_value();
        let nl2g_rc_new = self
            .builder
            .build_int_add(nl2g_rc_v, i64.const_int(1, false), "nl2g_rc_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2g_rc_ptr, nl2g_rc_new)
            .map_err(llvm_err)?;
        // Compute RC pointers for later use
        let new_intl_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(new_intl, i64, "ni_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "ni_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "ni_rc_p",
            )
            .map_err(llvm_err)?;
        let intl_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(intl_base, i64, "intl_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "intl_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "intl_rc_p",
            )
            .map_err(llvm_err)?;
        // Check if we have a parent (original height > 1)
        let lp_st_slot_val3 = self
            .builder
            .build_load(ptr, lp_parent_ptr, "st_slot_val3")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_has_parent3 = self
            .builder
            .build_int_compare(IntPredicate::NE, lp_st_slot_val3, null_ptr, "has_parent3")
            .map_err(llvm_err)?;
        let lp_split_has_parent = self
            .context
            .append_basic_block(list_push_fn, "split_has_parent");
        let lp_split_no_parent = self
            .context
            .append_basic_block(list_push_fn, "split_no_parent");
        let _ = self.builder.build_conditional_branch(
            lp_has_parent3,
            lp_split_has_parent,
            lp_split_no_parent,
        );

        // Has parent: add new_intl as a new sibling child in the parent
        // This avoids creating new_mid and keeps tree heights consistent.
        self.builder.position_at_end(lp_split_has_parent);
        // Update parent's subtree_total for intl_base to corrected_total
        // (it changed because the rightmost leaf split: 64 -> 32)
        let _ = self
            .builder
            .build_store(lp_st_slot_val3, intl_corrected_total)
            .map_err(llvm_err)?;
        // Set RC of new_intl to 1 (parent will reference it)
        let _ = self
            .builder
            .build_store(new_intl_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Load parent node
        let pn_val3 = self
            .builder
            .build_load(ptr, lp_parent_node, "pn_val3")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pn_pc_raw = self
            .builder
            .build_load(i32, pn_val3, "pn_pc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let pn_count = self
            .builder
            .build_int_z_extend(pn_pc_raw, i64, "pn_count")
            .map_err(llvm_err)?;
        // Parent children array at offset 16
        let pn_cbase = unsafe {
            self.builder
                .build_gep(i8, pn_val3, &[i64.const_int(16, false)], "pn_cbase")
                .map_err(llvm_err)
        }?;
        // New child slot at children[pn_count]
        let pn_new_child = unsafe {
            self.builder
                .build_gep(self.child_entry_type, pn_cbase, &[pn_count], "pn_nc")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(pn_new_child, new_intl)
            .map_err(llvm_err)?;
        let pn_nc_st = unsafe {
            self.builder
                .build_gep(i64, pn_new_child, &[i64.const_int(1, false)], "pn_nc_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(pn_nc_st, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Update parent count
        let pn_new_count = self
            .builder
            .build_int_add(pn_count, i64.const_int(1, false), "pn_new_count")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pn_val3, pn_new_count)
            .map_err(llvm_err)?;
        // Update parent total += 1
        let pn_tp3 = unsafe {
            self.builder
                .build_gep(i64, pn_val3, &[i64.const_int(1, false)], "pn_tp3")
                .map_err(llvm_err)
        }?;
        let pn_tot3 = self
            .builder
            .build_load(i64, pn_tp3, "pn_tot3")
            .map_err(llvm_err)?
            .into_int_value();
        let pn_tot_new3 = self
            .builder
            .build_int_add(pn_tot3, i64.const_int(1, false), "pn_tot_new3")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pn_tp3, pn_tot_new3)
            .map_err(llvm_err)?;
        let new_list_len3 = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_len3")
            .map_err(llvm_err)?;
        let undef_split_p = self.list_type.get_undef();
        let r_split_p_1 = self
            .builder
            .build_insert_value(undef_split_p, node_ptr, 0, "rsp1")
            .map_err(llvm_err)?;
        let r_split_p_2 = self
            .builder
            .build_insert_value(r_split_p_1, new_list_len3, 1, "rsp2")
            .map_err(llvm_err)?;
        let r_split_p_3 = self
            .builder
            .build_insert_value(r_split_p_2, height, 2, "rsp3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r_split_p_3));

        // No parent (original height == 1): create new_mid as new root
        self.builder.position_at_end(lp_split_no_parent);
        // Set RC of new_intl to 1 — new_mid will reference it
        let _ = self
            .builder
            .build_store(new_intl_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let new_mid = self
            .builder
            .build_call(malloc_rc_fn, &[internal_size.into()], "new_mid")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let new_mid_i8 = self
            .builder
            .build_pointer_cast(new_mid, ptr, "new_mid_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(new_mid_i8, i64.const_int(2, false))
            .map_err(llvm_err)?;
        let new_mid_tp = unsafe {
            self.builder
                .build_gep(i64, new_mid_i8, &[i64.const_int(1, false)], "nmid_tp")
                .map_err(llvm_err)
        }?;
        let thirty3 = i64.const_int(33, false);
        let new_mid_total = self
            .builder
            .build_int_add(intl_corrected_total, thirty3, "new_mid_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(new_mid_tp, new_mid_total)
            .map_err(llvm_err)?;
        let new_mid_cbase = unsafe {
            self.builder
                .build_gep(i8, new_mid_i8, &[i64.const_int(16, false)], "nmid_cbase")
                .map_err(llvm_err)
        }?;
        let new_mid_c0 = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    new_mid_cbase,
                    &[i64.const_int(0, false)],
                    "nmid_c0",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_mid_c0, intl_base)
            .map_err(llvm_err)?;
        let nmid_c0_st = unsafe {
            self.builder
                .build_gep(i64, new_mid_c0, &[i64.const_int(1, false)], "nmid_c0_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nmid_c0_st, intl_corrected_total)
            .map_err(llvm_err)?;
        // RC-inc intl_base (new_mid now references it)
        let intl_rc_v = self
            .builder
            .build_load(i64, intl_rc_ptr, "intl_rc_v")
            .map_err(llvm_err)?
            .into_int_value();
        let intl_rc_new = self
            .builder
            .build_int_add(intl_rc_v, i64.const_int(1, false), "intl_rc_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_rc_ptr, intl_rc_new)
            .map_err(llvm_err)?;
        let new_mid_c1 = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    new_mid_cbase,
                    &[i64.const_int(1, false)],
                    "nmid_c1",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_mid_c1, new_intl)
            .map_err(llvm_err)?;
        let nmid_c1_st = unsafe {
            self.builder
                .build_gep(i64, new_mid_c1, &[i64.const_int(1, false)], "nmid_c1_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nmid_c1_st, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // RC-inc new_intl (new_mid references it, adds to the 1 already set)
        let ni_rc_np = self
            .builder
            .build_load(i64, new_intl_rc_ptr, "ni_rc_np")
            .map_err(llvm_err)?
            .into_int_value();
        let ni_rc_new = self
            .builder
            .build_int_add(ni_rc_np, i64.const_int(1, false), "ni_rc_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(new_intl_rc_ptr, ni_rc_new)
            .map_err(llvm_err)?;
        let new_h = self
            .builder
            .build_int_add(height, i64.const_int(1, false), "new_h")
            .map_err(llvm_err)?;
        let new_list_len4 = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_len4")
            .map_err(llvm_err)?;
        let undef_split = self.list_type.get_undef();
        let r_split_1 = self
            .builder
            .build_insert_value(undef_split, new_mid, 0, "rs1")
            .map_err(llvm_err)?;
        let r_split_2 = self
            .builder
            .build_insert_value(r_split_1, new_list_len4, 1, "rs2")
            .map_err(llvm_err)?;
        let r_split_3 = self
            .builder
            .build_insert_value(r_split_2, new_h, 2, "rs3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r_split_3));

        // ---- action_list_get({ptr, i64, i64}, i64) -> {i64, ptr} ----
        // Block-based: traverse tree to find element at index.
        let list_get_fn = self.module.get_function("action_list_get").unwrap();
        let lg_entry = self.context.append_basic_block(list_get_fn, "entry");
        let lg_concat_loop = self.context.append_basic_block(list_get_fn, "concat_loop");
        let lg_h0 = self.context.append_basic_block(list_get_fn, "h0");
        let lg_h0_body = self.context.append_basic_block(list_get_fn, "h0_body");
        let lg_hgt0 = self.context.append_basic_block(list_get_fn, "hgt0");
        let lg_hgt0_loop = self.context.append_basic_block(list_get_fn, "hgt0_loop");
        let lg_hgt0_found = self.context.append_basic_block(list_get_fn, "hgt0_found");
        let lg_hgt0_next = self.context.append_basic_block(list_get_fn, "hgt0_next");
        let lg_ret = self.context.append_basic_block(list_get_fn, "ret");
        self.builder.position_at_end(lg_entry);
        let list = list_get_fn.get_first_param().unwrap().into_struct_value();
        let idx = list_get_fn.get_nth_param(1).unwrap().into_int_value();
        let node_ptr = self
            .builder
            .build_extract_value(list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let height = self
            .builder
            .build_extract_value(list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        // Check if ConcatNode (height == -1) — delegate through ConcatNode chain
        let is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_concat, lg_concat_loop, lg_h0);

        // ConcatNode delegation loop: use cached left_len in ConcatNode, descend in O(depth)
        self.builder.position_at_end(lg_concat_loop);
        let lg_phi_node = self.builder.build_phi(ptr, "lg_phi_n").map_err(llvm_err)?;
        let lg_phi_idx = self.builder.build_phi(i64, "lg_phi_i").map_err(llvm_err)?;
        lg_phi_node.add_incoming(&[(&node_ptr, lg_entry)]);
        lg_phi_idx.add_incoming(&[(&idx, lg_entry)]);
        let cc_node = lg_phi_node.as_basic_value().into_pointer_value();
        let cc_idx = lg_phi_idx.as_basic_value().into_int_value();
        // Cached left subtree size at ConcatNode offset 3 (left list len field)
        let cc_left_len_p = unsafe {
            self.builder
                .build_gep(i64, cc_node, &[i64.const_int(3, false)], "cc_llp")
                .map_err(llvm_err)
        }?;
        let cc_left_len = self
            .builder
            .build_load(i64, cc_left_len_p, "cc_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_go_left = self
            .builder
            .build_int_compare(IntPredicate::SLT, cc_idx, cc_left_len, "cc_gl")
            .map_err(llvm_err)?;
        let cc_left_node_p = unsafe {
            self.builder
                .build_gep(ptr, cc_node, &[i64.const_int(2, false)], "cc_lnp")
                .map_err(llvm_err)
        }?;
        let cc_left_node = self
            .builder
            .build_load(ptr, cc_left_node_p, "cc_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc_left_h_p = unsafe {
            self.builder
                .build_gep(i64, cc_node, &[i64.const_int(4, false)], "cc_lhp")
                .map_err(llvm_err)
        }?;
        let cc_left_h = self
            .builder
            .build_load(i64, cc_left_h_p, "cc_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_right_node_p = unsafe {
            self.builder
                .build_gep(ptr, cc_node, &[i64.const_int(5, false)], "cc_rnp")
                .map_err(llvm_err)
        }?;
        let cc_right_node = self
            .builder
            .build_load(ptr, cc_right_node_p, "cc_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc_right_h_p = unsafe {
            self.builder
                .build_gep(i64, cc_node, &[i64.const_int(7, false)], "cc_rhp")
                .map_err(llvm_err)
        }?;
        let cc_right_h = self
            .builder
            .build_load(i64, cc_right_h_p, "cc_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_right_idx = self
            .builder
            .build_int_sub(cc_idx, cc_left_len, "cc_ni")
            .map_err(llvm_err)?;
        let cc_next_node = self
            .builder
            .build_select(cc_go_left, cc_left_node, cc_right_node, "cc_nn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc_next_h = self
            .builder
            .build_select(cc_go_left, cc_left_h, cc_right_h, "cc_nh")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_next_idx = self
            .builder
            .build_select(cc_go_left, cc_idx, cc_right_idx, "cc_ni2")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_neg1 = i64.const_int(-1i64 as u64, true);
        let cc_child_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, cc_next_h, cc_neg1, "cc_cic")
            .map_err(llvm_err)?;
        lg_phi_node.add_incoming(&[(&cc_next_node, lg_concat_loop)]);
        lg_phi_idx.add_incoming(&[(&cc_next_idx, lg_concat_loop)]);
        let _ = self
            .builder
            .build_conditional_branch(cc_child_is_concat, lg_concat_loop, lg_h0);
        let zero = i64.const_int(0, false);

        // Height == 0: single leaf, direct access
        // Phi nodes for resolved node, height, idx from entry and concat descent
        self.builder.position_at_end(lg_h0);
        let lg_resolved_node = self.builder.build_phi(ptr, "lg_rn").map_err(llvm_err)?;
        let lg_resolved_h = self.builder.build_phi(i64, "lg_rh").map_err(llvm_err)?;
        let lg_resolved_idx = self.builder.build_phi(i64, "lg_ri").map_err(llvm_err)?;
        lg_resolved_node.add_incoming(&[(&node_ptr, lg_entry)]);
        lg_resolved_h.add_incoming(&[(&height, lg_entry)]);
        lg_resolved_idx.add_incoming(&[(&idx, lg_entry)]);
        lg_resolved_node.add_incoming(&[(&cc_next_node, lg_concat_loop)]);
        lg_resolved_h.add_incoming(&[(&cc_next_h, lg_concat_loop)]);
        lg_resolved_idx.add_incoming(&[(&cc_next_idx, lg_concat_loop)]);
        let rn = lg_resolved_node.as_basic_value().into_pointer_value();
        let rh = lg_resolved_h.as_basic_value().into_int_value();
        let ri = lg_resolved_idx.as_basic_value().into_int_value();

        let is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, rh, zero, "is_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_h0, lg_h0_body, lg_hgt0);

        // h=0 body
        self.builder.position_at_end(lg_h0_body);
        let leaf_i8 = self
            .builder
            .build_pointer_cast(rn, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let elem_base = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "elem_base")
                .map_err(llvm_err)?
        };
        let elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, elem_base, &[ri], "elem_ptr")
                .map_err(llvm_err)?
        };
        let elem_val = self
            .builder
            .build_load(self.string_type, elem_ptr, "elem")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lg_ret);

        // Height > 0: traverse internal nodes
        // current_node = rn; remaining_height = rh; remaining_idx = ri
        self.builder.position_at_end(lg_hgt0);
        let _ = self.builder.build_unconditional_branch(lg_hgt0_loop);

        // Loop: iterate through internal nodes using subtree_total
        self.builder.position_at_end(lg_hgt0_loop);
        // Phi: {current_node, remaining_height, remaining_idx}
        let phi_node = self.builder.build_phi(ptr, "phi_node").map_err(llvm_err)?;
        let phi_height = self
            .builder
            .build_phi(i64, "phi_height")
            .map_err(llvm_err)?;
        let phi_idx = self.builder.build_phi(i64, "phi_idx").map_err(llvm_err)?;
        phi_node.add_incoming(&[(&rn, lg_hgt0)]);
        phi_height.add_incoming(&[(&rh, lg_hgt0)]);
        phi_idx.add_incoming(&[(&ri, lg_hgt0)]);
        let cur_node = phi_node.as_basic_value().into_pointer_value();
        let cur_height = phi_height.as_basic_value().into_int_value();
        let cur_idx = phi_idx.as_basic_value().into_int_value();
        // If height == 0, we've reached a leaf
        let is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, cur_height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_leaf, lg_hgt0_found, lg_hgt0_next);

        // Found leaf: load element
        self.builder.position_at_end(lg_hgt0_found);
        let found_leaf_i8 = self
            .builder
            .build_pointer_cast(cur_node, ptr, "fl_i8")
            .map_err(llvm_err)?;
        let found_elem_base = unsafe {
            self.builder
                .build_gep(i8, found_leaf_i8, &[i64.const_int(8, false)], "feb")
                .map_err(llvm_err)?
        };
        let found_elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, found_elem_base, &[cur_idx], "fe_p")
                .map_err(llvm_err)?
        };
        let found_elem = self
            .builder
            .build_load(self.string_type, found_elem_ptr, "fe")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lg_ret);

        // Internal node: find which child contains the index
        // children array at offset 16 (after i32 count + i32 pad + i64 total)
        // child_entry = {ptr child, i64 subtree_total}
        self.builder.position_at_end(lg_hgt0_next);
        let intl_i8 = self
            .builder
            .build_pointer_cast(cur_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let intl_count_raw = self
            .builder
            .build_load(i32, intl_i8, "intl_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let intl_count = self
            .builder
            .build_int_z_extend(intl_count_raw, i64, "intl_count")
            .map_err(llvm_err)?;
        // Iterate children: for i in 0..count, check if idx < child[i].subtree_total
        // For simplicity, scan linearly (B=64, so at most 64 iterations)
        // Use a loop or just unrolled scan
        // Store result: (child_ptr, child_subtree_total, child_idx)
        // For now: simple linear scan in a loop
        let scan_loop = self.context.append_basic_block(list_get_fn, "scan_loop");
        let scan_body = self.context.append_basic_block(list_get_fn, "scan_body");
        let scan_found = self.context.append_basic_block(list_get_fn, "scan_found");
        let scan_next = self.context.append_basic_block(list_get_fn, "scan_next");
        let _ = self.builder.build_unconditional_branch(scan_loop);
        self.builder.position_at_end(scan_loop);
        let phi_i = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
        let phi_acc = self.builder.build_phi(i64, "phi_acc").map_err(llvm_err)?;
        phi_i.add_incoming(&[(&zero, lg_hgt0_next)]);
        phi_acc.add_incoming(&[(&zero, lg_hgt0_next)]);
        let scan_i = phi_i.as_basic_value().into_int_value();
        let scan_acc = phi_acc.as_basic_value().into_int_value();
        let done_scan = self
            .builder
            .build_int_compare(IntPredicate::SGE, scan_i, intl_count, "done_scan")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_scan, scan_found, scan_body);

        self.builder.position_at_end(scan_body);
        // Load child[scan_i].subtree_total
        let scan_children_base = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "scb")
                .map_err(llvm_err)?
        };
        let child_entry_ptr = unsafe {
            self.builder
                .build_gep(self.child_entry_type, scan_children_base, &[scan_i], "cep")
                .map_err(llvm_err)?
        };
        let child_total = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, child_entry_ptr, "ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "ct",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let new_acc = self
            .builder
            .build_int_add(scan_acc, child_total, "new_acc")
            .map_err(llvm_err)?;
        let found_child = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur_idx, new_acc, "found_child")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(found_child, scan_found, scan_next);

        self.builder.position_at_end(scan_next);
        let next_i = self
            .builder
            .build_int_add(scan_i, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(scan_loop);
        phi_i.add_incoming(&[(&next_i, scan_next)]);
        phi_acc.add_incoming(&[(&new_acc, scan_next)]);

        self.builder.position_at_end(scan_found);
        // phi for the found child index and accumulated offset before this child
        let phi_found_i = self.builder.build_phi(i64, "phi_fi").map_err(llvm_err)?;
        let phi_found_acc = self.builder.build_phi(i64, "phi_fa").map_err(llvm_err)?;
        phi_found_i.add_incoming(&[(&scan_i, scan_body), (&scan_i, scan_loop)]);
        // The accumulated offset before this child is scan_acc (not new_acc)
        phi_found_acc.add_incoming(&[(&scan_acc, scan_body), (&scan_acc, scan_loop)]);
        let found_i = phi_found_i.as_basic_value().into_int_value();
        let offset_before = phi_found_acc.as_basic_value().into_int_value();
        // Load child[found_i].ptr
        let found_ce_base = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "fceb")
                .map_err(llvm_err)?
        };
        let found_ce_ptr = unsafe {
            self.builder
                .build_gep(self.child_entry_type, found_ce_base, &[found_i], "fcep")
                .map_err(llvm_err)?
        };
        let found_ce = self
            .builder
            .build_load(self.child_entry_type, found_ce_ptr, "fce")
            .map_err(llvm_err)?
            .into_struct_value();
        let child_ptr = self
            .builder
            .build_extract_value(found_ce, 0, "child_p")
            .map_err(llvm_err)?
            .into_pointer_value();
        let new_idx = self
            .builder
            .build_int_sub(cur_idx, offset_before, "new_idx")
            .map_err(llvm_err)?;
        let new_height = self
            .builder
            .build_int_sub(cur_height, i64.const_int(1, false), "new_h")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lg_hgt0_loop);
        phi_node.add_incoming(&[(&child_ptr, scan_found)]);
        phi_height.add_incoming(&[(&new_height, scan_found)]);
        phi_idx.add_incoming(&[(&new_idx, scan_found)]);

        // Return
        self.builder.position_at_end(lg_ret);
        let phi_ret = self
            .builder
            .build_phi(self.string_type, "phi_ret")
            .map_err(llvm_err)?;
        phi_ret.add_incoming(&[(&elem_val, lg_h0_body), (&found_elem, lg_hgt0_found)]);
        let _ = self.builder.build_return(Some(&phi_ret.as_basic_value()));

        // ---- action_list_print({ptr, i64, i64}) ----
        let list_print_fn = self.module.add_function(
            "action_list_print",
            void.fn_type(&[self.list_type.into()], false),
            None,
        );
        let lp_entry = self.context.append_basic_block(list_print_fn, "entry");
        self.builder.position_at_end(lp_entry);
        let lp_list = list_print_fn.get_first_param().unwrap().into_struct_value();
        let lp_len = self
            .builder
            .build_extract_value(lp_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        // Print "["
        let _ = self.builder.build_call(printf_fn, &[fmt_lb_ptr.into()], "");
        let lp_i = self.builder.build_alloca(i64, "lpi").map_err(llvm_err)?;
        self.builder
            .build_store(lp_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let lp_hdr = self.context.append_basic_block(list_print_fn, "lphdr");
        let lp_bdy = self.context.append_basic_block(list_print_fn, "lpbdy");
        let lp_ext = self.context.append_basic_block(list_print_fn, "lpext");
        let _ = self.builder.build_unconditional_branch(lp_hdr);
        self.builder.position_at_end(lp_hdr);
        let lp_iv = self
            .builder
            .build_load(i64, lp_i, "lpiv")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lp_iv, lp_len, "lpcond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lp_cond, lp_bdy, lp_ext);
        self.builder.position_at_end(lp_bdy);
        // Print ", " if not first
        let lp_is_first = self
            .builder
            .build_int_compare(IntPredicate::EQ, lp_iv, i64.const_int(0, false), "is_first")
            .map_err(llvm_err)?;
        let lp_sep_bb = self.context.append_basic_block(list_print_fn, "lpsep");
        let lp_val_bb = self.context.append_basic_block(list_print_fn, "lpval");
        let _ = self
            .builder
            .build_conditional_branch(lp_is_first, lp_val_bb, lp_sep_bb);
        self.builder.position_at_end(lp_sep_bb);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_sep_ptr.into()], "");
        let _ = self.builder.build_unconditional_branch(lp_val_bb);
        self.builder.position_at_end(lp_val_bb);
        let lp_elem_val = self
            .builder
            .build_call(list_get_fn, &[lp_list.into(), lp_iv.into()], "lpe")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?;
        let lp_elem = lp_elem_val.into_struct_value();
        let lp_tag = self
            .builder
            .build_extract_value(lp_elem, 0, "lptag")
            .map_err(llvm_err)?
            .into_int_value();
        // Print integer tag for now
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), lp_tag.into()], "");
        // Next
        let lp_next = self
            .builder
            .build_int_add(lp_iv, i64.const_int(1, false), "lpnext")
            .map_err(llvm_err)?;
        self.builder.build_store(lp_i, lp_next).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_hdr);
        self.builder.position_at_end(lp_ext);
        let _ = self.builder.build_call(printf_fn, &[fmt_rb_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_list_set_rec(ptr node, i64 height, i64 idx, {i64,ptr} val) -> ptr ----
        // B-tree path-copy single-element update. CoW when rc > 1.
        let lsr_fn = self.module.add_function(
            "action_list_set_rec",
            ptr.fn_type(
                &[ptr.into(), i64.into(), i64.into(), self.string_type.into()],
                false,
            ),
            None,
        );
        let lsr_entry = self.context.append_basic_block(lsr_fn, "entry");
        let lsr_leaf = self.context.append_basic_block(lsr_fn, "leaf");
        let lsr_leaf_cow = self.context.append_basic_block(lsr_fn, "leaf_cow");
        let lsr_leaf_cow_copy = self.context.append_basic_block(lsr_fn, "leaf_cow_copy");
        let lsr_leaf_store = self.context.append_basic_block(lsr_fn, "leaf_store");
        let lsr_int_scan_loop = self.context.append_basic_block(lsr_fn, "int_scan_loop");
        let lsr_int_scan_body = self.context.append_basic_block(lsr_fn, "int_scan_body");
        let lsr_int_scan_found = self.context.append_basic_block(lsr_fn, "int_scan_found");
        let lsr_int_scan_next = self.context.append_basic_block(lsr_fn, "int_scan_next");
        let lsr_int_cow = self.context.append_basic_block(lsr_fn, "int_cow");
        let lsr_int_cow_copy = self.context.append_basic_block(lsr_fn, "int_cow_copy");
        let lsr_int_update = self.context.append_basic_block(lsr_fn, "int_update");
        let lsr_int_ret = self.context.append_basic_block(lsr_fn, "int_ret");
        self.builder.position_at_end(lsr_entry);
        let lsr_node = lsr_fn.get_first_param().unwrap().into_pointer_value();
        let lsr_height = lsr_fn.get_nth_param(1).unwrap().into_int_value();
        let lsr_idx = lsr_fn.get_nth_param(2).unwrap().into_int_value();
        let lsr_val = lsr_fn.get_nth_param(3).unwrap().into_struct_value();
        let lsr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, lsr_height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lsr_is_leaf, lsr_leaf, lsr_int_scan_loop);

        // Leaf: CoW if shared, store element at idx
        self.builder.position_at_end(lsr_leaf);
        let lsr_leaf_int = self
            .builder
            .build_ptr_to_int(lsr_node, i64, "leaf_int")
            .map_err(llvm_err)?;
        let lsr_leaf_rc_a = self
            .builder
            .build_int_sub(lsr_leaf_int, i64.const_int(8, false), "leaf_rc_a")
            .map_err(llvm_err)?;
        let lsr_leaf_rc_p = self
            .builder
            .build_int_to_ptr(lsr_leaf_rc_a, ptr, "leaf_rc_p")
            .map_err(llvm_err)?;
        let lsr_leaf_rc = self
            .builder
            .build_load(i64, lsr_leaf_rc_p, "leaf_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_leaf_shared = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                lsr_leaf_rc,
                i64.const_int(1, false),
                "leaf_shared",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lsr_leaf_shared, lsr_leaf_cow, lsr_leaf_store);

        self.builder.position_at_end(lsr_leaf_cow);
        let lsr_need_copy = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                lsr_leaf_rc,
                i64.const_int(1, false),
                "need_copy",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lsr_need_copy, lsr_leaf_cow_copy, lsr_leaf_store);

        self.builder.position_at_end(lsr_leaf_cow_copy);
        let lsr_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let lsr_new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[lsr_leaf_sz.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let lsr_memcpy = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                lsr_memcpy,
                &[lsr_new_leaf.into(), lsr_node.into(), lsr_leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let lsr_new_leaf_rc = self
            .builder
            .build_int_sub(lsr_leaf_rc, i64.const_int(1, false), "new_leaf_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lsr_leaf_rc_p, lsr_new_leaf_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_leaf_store);

        self.builder.position_at_end(lsr_leaf_store);
        let lsr_leaf_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        lsr_leaf_phi.add_incoming(&[
            (&lsr_node, lsr_leaf),
            (&lsr_node, lsr_leaf_cow),
            (&lsr_new_leaf, lsr_leaf_cow_copy),
        ]);
        let lsr_leaf_ptr = lsr_leaf_phi.as_basic_value().into_pointer_value();
        let lsr_leaf_i8 = self
            .builder
            .build_pointer_cast(lsr_leaf_ptr, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let lsr_eb = unsafe {
            self.builder
                .build_gep(i8, lsr_leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)?
        };
        let lsr_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lsr_eb, &[lsr_idx], "ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(lsr_ep, lsr_val)
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lsr_leaf_ptr));

        // Internal: scan children to find target, recurse, path-copy on way up
        self.builder.position_at_end(lsr_int_scan_loop);
        let lsr_phi_i = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
        let lsr_phi_acc = self.builder.build_phi(i64, "phi_acc").map_err(llvm_err)?;
        lsr_phi_i.add_incoming(&[(&zero, lsr_entry)]);
        lsr_phi_acc.add_incoming(&[(&zero, lsr_entry)]);
        let lsr_scan_i = lsr_phi_i.as_basic_value().into_int_value();
        let lsr_scan_acc = lsr_phi_acc.as_basic_value().into_int_value();
        let lsr_int_i8 = self
            .builder
            .build_pointer_cast(lsr_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let lsr_int_count_raw = self
            .builder
            .build_load(i32, lsr_int_i8, "intl_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_int_count = self
            .builder
            .build_int_z_extend(lsr_int_count_raw, i64, "intl_count")
            .map_err(llvm_err)?;
        let lsr_done_scan = self
            .builder
            .build_int_compare(IntPredicate::SGE, lsr_scan_i, lsr_int_count, "done_scan")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            lsr_done_scan,
            lsr_int_scan_found,
            lsr_int_scan_body,
        );

        self.builder.position_at_end(lsr_int_scan_body);
        let lsr_children_base = unsafe {
            self.builder
                .build_gep(i8, lsr_int_i8, &[i64.const_int(16, false)], "scb")
                .map_err(llvm_err)?
        };
        let lsr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    lsr_children_base,
                    &[lsr_scan_i],
                    "cep",
                )
                .map_err(llvm_err)?
        };
        let lsr_child_total = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, lsr_child_ep, "ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "ct",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_new_acc = self
            .builder
            .build_int_add(lsr_scan_acc, lsr_child_total, "new_acc")
            .map_err(llvm_err)?;
        let lsr_found_child = self
            .builder
            .build_int_compare(IntPredicate::SLT, lsr_idx, lsr_new_acc, "found_child")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            lsr_found_child,
            lsr_int_scan_found,
            lsr_int_scan_next,
        );

        self.builder.position_at_end(lsr_int_scan_next);
        let lsr_next_i = self
            .builder
            .build_int_add(lsr_scan_i, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_int_scan_loop);
        lsr_phi_i.add_incoming(&[(&lsr_next_i, lsr_int_scan_next)]);
        lsr_phi_acc.add_incoming(&[(&lsr_new_acc, lsr_int_scan_next)]);

        self.builder.position_at_end(lsr_int_scan_found);
        let lsr_phi_found_i = self.builder.build_phi(i64, "phi_fi").map_err(llvm_err)?;
        let lsr_phi_found_acc = self.builder.build_phi(i64, "phi_fa").map_err(llvm_err)?;
        lsr_phi_found_i.add_incoming(&[
            (&lsr_scan_i, lsr_int_scan_body),
            (&lsr_scan_i, lsr_int_scan_loop),
        ]);
        lsr_phi_found_acc.add_incoming(&[
            (&lsr_scan_acc, lsr_int_scan_body),
            (&lsr_scan_acc, lsr_int_scan_loop),
        ]);
        let lsr_found_i = lsr_phi_found_i.as_basic_value().into_int_value();
        let lsr_offset_before = lsr_phi_found_acc.as_basic_value().into_int_value();
        let lsr_found_ce_base = unsafe {
            self.builder
                .build_gep(i8, lsr_int_i8, &[i64.const_int(16, false)], "fceb")
                .map_err(llvm_err)?
        };
        let lsr_found_ce_ptr = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    lsr_found_ce_base,
                    &[lsr_found_i],
                    "fcep",
                )
                .map_err(llvm_err)?
        };
        let lsr_old_child = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, lsr_found_ce_ptr, "fce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                0,
                "old_child",
            )
            .map_err(llvm_err)?
            .into_pointer_value();
        let lsr_local_idx = self
            .builder
            .build_int_sub(lsr_idx, lsr_offset_before, "local_idx")
            .map_err(llvm_err)?;
        let lsr_child_h = self
            .builder
            .build_int_sub(lsr_height, i64.const_int(1, false), "child_h")
            .map_err(llvm_err)?;
        let lsr_new_child = self
            .builder
            .build_call(
                lsr_fn,
                &[
                    lsr_old_child.into(),
                    lsr_child_h.into(),
                    lsr_local_idx.into(),
                    lsr_val.into(),
                ],
                "new_child",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self.builder.build_unconditional_branch(lsr_int_cow);

        // CoW internal node if shared
        self.builder.position_at_end(lsr_int_cow);
        let lsr_int_int = self
            .builder
            .build_ptr_to_int(lsr_node, i64, "int_int")
            .map_err(llvm_err)?;
        let lsr_int_rc_a = self
            .builder
            .build_int_sub(lsr_int_int, i64.const_int(8, false), "int_rc_a")
            .map_err(llvm_err)?;
        let lsr_int_rc_p = self
            .builder
            .build_int_to_ptr(lsr_int_rc_a, ptr, "int_rc_p")
            .map_err(llvm_err)?;
        let lsr_int_rc = self
            .builder
            .build_load(i64, lsr_int_rc_p, "int_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_int_shared = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                lsr_int_rc,
                i64.const_int(1, false),
                "int_shared",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lsr_int_shared, lsr_int_cow_copy, lsr_int_update);

        self.builder.position_at_end(lsr_int_cow_copy);
        let lsr_int_sz = self.internal_type.size_of().ok_or("internal size")?;
        let lsr_new_int = self
            .builder
            .build_call(malloc_rc_fn, &[lsr_int_sz.into()], "new_int")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                lsr_memcpy,
                &[lsr_new_int.into(), lsr_node.into(), lsr_int_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let lsr_new_int_rc = self
            .builder
            .build_int_sub(lsr_int_rc, i64.const_int(1, false), "new_int_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lsr_int_rc_p, lsr_new_int_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_int_update);

        self.builder.position_at_end(lsr_int_update);
        let lsr_work_phi = self.builder.build_phi(ptr, "work_phi").map_err(llvm_err)?;
        lsr_work_phi.add_incoming(&[(&lsr_node, lsr_int_cow), (&lsr_new_int, lsr_int_cow_copy)]);
        let lsr_work_node = lsr_work_phi.as_basic_value().into_pointer_value();
        let lsr_work_i8 = self
            .builder
            .build_pointer_cast(lsr_work_node, ptr, "work_i8")
            .map_err(llvm_err)?;
        let lsr_upd_ce_base = unsafe {
            self.builder
                .build_gep(i8, lsr_work_i8, &[i64.const_int(16, false)], "upb")
                .map_err(llvm_err)?
        };
        let lsr_upd_ce_ptr = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    lsr_upd_ce_base,
                    &[lsr_found_i],
                    "upcep",
                )
                .map_err(llvm_err)?
        };
        let lsr_child_slot = self
            .builder
            .build_pointer_cast(lsr_upd_ce_ptr, ptr, "child_slot")
            .map_err(llvm_err)?;
        let lsr_child_changed = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                self.builder
                    .build_ptr_to_int(lsr_new_child, i64, "nc_i")
                    .map_err(llvm_err)?,
                self.builder
                    .build_ptr_to_int(lsr_old_child, i64, "oc_i")
                    .map_err(llvm_err)?,
                "child_changed",
            )
            .map_err(llvm_err)?;
        let lsr_dec_old = self.context.append_basic_block(lsr_fn, "dec_old");
        let lsr_store_child = self.context.append_basic_block(lsr_fn, "store_child");
        let _ =
            self.builder
                .build_conditional_branch(lsr_child_changed, lsr_dec_old, lsr_store_child);
        self.builder.position_at_end(lsr_dec_old);
        let lsr_old_child_rc_a = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(lsr_old_child, i64, "oc_int")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "oc_rc_a",
            )
            .map_err(llvm_err)?;
        let lsr_old_child_rc_p = self
            .builder
            .build_int_to_ptr(lsr_old_child_rc_a, ptr, "oc_rc_p")
            .map_err(llvm_err)?;
        let lsr_old_child_rc = self
            .builder
            .build_load(i64, lsr_old_child_rc_p, "oc_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_old_child_rc_dec = self
            .builder
            .build_int_sub(lsr_old_child_rc, i64.const_int(1, false), "oc_rc_dec")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lsr_old_child_rc_p, lsr_old_child_rc_dec)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_store_child);
        self.builder.position_at_end(lsr_store_child);
        let _ = self
            .builder
            .build_store(lsr_child_slot, lsr_new_child)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_int_ret);

        self.builder.position_at_end(lsr_int_ret);
        let _ = self.builder.build_return(Some(&lsr_work_node));

        // ---- action_list_set({ptr, i64, i64}, i64, {i64, ptr}) -> {ptr, i64, i64} ----
        // Set element at index to value, CoW-safe. Returns new root.
        let list_set_fn = self.module.add_function(
            "action_list_set",
            self.list_type.fn_type(
                &[self.list_type.into(), i64.into(), self.string_type.into()],
                false,
            ),
            None,
        );
        let ls_entry = self.context.append_basic_block(list_set_fn, "entry");
        let ls_concat = self.context.append_basic_block(list_set_fn, "concat");
        let ls_normal = self.context.append_basic_block(list_set_fn, "normal");
        let ls_h0 = self.context.append_basic_block(list_set_fn, "h0");
        let ls_h0_cow = self.context.append_basic_block(list_set_fn, "h0_cow");
        let ls_h0_store = self.context.append_basic_block(list_set_fn, "h0_store");
        let ls_hgt0 = self.context.append_basic_block(list_set_fn, "hgt0");

        self.builder.position_at_end(ls_entry);
        let ls_list = list_set_fn.get_first_param().unwrap().into_struct_value();
        let ls_idx = list_set_fn.get_nth_param(1).unwrap().into_int_value();
        let ls_val = list_set_fn.get_nth_param(2).unwrap().into_struct_value();
        let ls_height = self
            .builder
            .build_extract_value(ls_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ls_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ls_is_concat, ls_concat, ls_normal);
        // ConcatNode: flatten then set
        self.builder.position_at_end(ls_concat);
        let ls_flatten_fn = self.module.get_function("action_list_flatten").unwrap();
        let ls_flat = self
            .builder
            .build_call(ls_flatten_fn, &[ls_list.into()], "flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let ls_pushed = self
            .builder
            .build_call(
                list_set_fn,
                &[ls_flat.into(), ls_idx.into(), ls_val.into()],
                "set",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&ls_pushed));
        // Normal path
        self.builder.position_at_end(ls_normal);
        let ls_node = self
            .builder
            .build_extract_value(ls_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ls_len = self
            .builder
            .build_extract_value(ls_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_h = self
            .builder
            .build_extract_value(ls_list, 2, "h")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, ls_h, zero, "is_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ls_is_h0, ls_h0, ls_hgt0);

        // Height == 0: direct manipulation
        self.builder.position_at_end(ls_h0);
        let ls_node_int = self
            .builder
            .build_ptr_to_int(ls_node, i64, "node_int")
            .map_err(llvm_err)?;
        let ls_rc_a = self
            .builder
            .build_int_sub(ls_node_int, i64.const_int(8, false), "rc_a")
            .map_err(llvm_err)?;
        let ls_rc_p = self
            .builder
            .build_int_to_ptr(ls_rc_a, ptr, "rc_p")
            .map_err(llvm_err)?;
        let ls_rc = self
            .builder
            .build_load(i64, ls_rc_p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_cow = self
            .builder
            .build_int_compare(IntPredicate::SGT, ls_rc, i64.const_int(1, false), "cow")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ls_cow, ls_h0_cow, ls_h0_store);

        self.builder.position_at_end(ls_h0_cow);
        let ls_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let ls_new = self
            .builder
            .build_call(malloc_rc_fn, &[ls_leaf_sz.into()], "new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let ls_cpy = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                ls_cpy,
                &[ls_new.into(), ls_node.into(), ls_leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let ls_new_rc = self
            .builder
            .build_int_sub(ls_rc, i64.const_int(1, false), "new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(ls_rc_p, ls_new_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ls_h0_store);

        self.builder.position_at_end(ls_h0_store);
        let ls_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        ls_phi.add_incoming(&[(&ls_node, ls_h0), (&ls_new, ls_h0_cow)]);
        let ls_leaf = ls_phi.as_basic_value().into_pointer_value();
        let ls_li8 = self
            .builder
            .build_pointer_cast(ls_leaf, ptr, "li8")
            .map_err(llvm_err)?;
        let ls_eb = unsafe {
            self.builder
                .build_gep(i8, ls_li8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)?
        };
        let ls_ep = unsafe {
            self.builder
                .build_gep(self.string_type, ls_eb, &[ls_idx], "ep")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(ls_ep, ls_val).map_err(llvm_err)?;
        let ls_undef = self.list_type.get_undef();
        let ls_r1 = self
            .builder
            .build_insert_value(ls_undef, ls_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let ls_r2 = self
            .builder
            .build_insert_value(ls_r1, ls_len, 1, "r2")
            .map_err(llvm_err)?;
        let ls_r3 = self
            .builder
            .build_insert_value(ls_r2, zero, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ls_r3));

        // Height > 0: B-tree path-copy via action_list_set_rec
        self.builder.position_at_end(ls_hgt0);
        let ls_set_rec_fn = self.module.get_function("action_list_set_rec").unwrap();
        let ls_new_root = self
            .builder
            .build_call(
                ls_set_rec_fn,
                &[ls_node.into(), ls_h.into(), ls_idx.into(), ls_val.into()],
                "new_root",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let ls_undef_h = self.list_type.get_undef();
        let ls_hr1 = self
            .builder
            .build_insert_value(ls_undef_h, ls_new_root, 0, "hr1")
            .map_err(llvm_err)?;
        let ls_hr2 = self
            .builder
            .build_insert_value(ls_hr1, ls_len, 1, "hr2")
            .map_err(llvm_err)?;
        let ls_hr3 = self
            .builder
            .build_insert_value(ls_hr2, ls_h, 2, "hr3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ls_hr3));

        // ---- action_list_head({ptr, i64, i64}) -> {i64, ptr} ----
        // Delegates to get(0), which handles ConcatNodes.
        let list_head_fn = self.module.add_function(
            "action_list_head",
            self.string_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(list_head_fn, "entry");
        self.builder.position_at_end(entry);
        let lh_list = list_head_fn.get_first_param().unwrap().into_struct_value();
        let lh_len = self
            .builder
            .build_extract_value(lh_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lh_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, lh_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let lh_has = self.context.append_basic_block(list_head_fn, "has");
        let lh_none = self.context.append_basic_block(list_head_fn, "none");
        let _ = self
            .builder
            .build_conditional_branch(lh_empty, lh_none, lh_has);
        self.builder.position_at_end(lh_none);
        let lh_none_val = self.string_type.const_zero();
        let _ = self.builder.build_return(Some(&lh_none_val));
        self.builder.position_at_end(lh_has);
        // For ConcatNode: get(0) delegates through ConcatNode chain
        let lh_get_fn = self.module.get_function("action_list_get").unwrap();
        let lh_val = self
            .builder
            .build_call(
                lh_get_fn,
                &[lh_list.into(), i64.const_int(0, false).into()],
                "val",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lh_val));

        // ---- action_list_len({ptr, i64, i64}) -> i64 ----
        let list_len_fn = self.module.add_function(
            "action_list_len",
            i64.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(list_len_fn, "entry");
        self.builder.position_at_end(entry);
        let list = list_len_fn.get_first_param().unwrap().into_struct_value();
        let len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&len));

        // ---- action_list_contains_walk(ptr node, i64 height, {i64,ptr} key) -> i1 ----
        // In-order B-tree scan: reads leaf slots directly instead of index-based get().
        let lc_walk_fn = self.module.add_function(
            "action_list_contains_walk",
            b1.fn_type(&[ptr.into(), i64.into(), self.string_type.into()], false),
            None,
        );
        let lw_entry = self.context.append_basic_block(lc_walk_fn, "entry");
        let lw_leaf_hdr = self.context.append_basic_block(lc_walk_fn, "leaf_hdr");
        let lw_leaf_bdy = self.context.append_basic_block(lc_walk_fn, "leaf_bdy");
        let lw_leaf_next = self.context.append_basic_block(lc_walk_fn, "leaf_next");
        let lw_leaf_chk = self.context.append_basic_block(lc_walk_fn, "leaf_chk");
        let lw_leaf_found = self.context.append_basic_block(lc_walk_fn, "leaf_found");
        let lw_leaf_content = self.context.append_basic_block(lc_walk_fn, "leaf_content");
        let lw_leaf_str_gate = self.context.append_basic_block(lc_walk_fn, "leaf_str_gate");
        let lw_leaf_str_cmp = self.context.append_basic_block(lc_walk_fn, "leaf_str_cmp");
        let lw_leaf_str_found = self
            .context
            .append_basic_block(lc_walk_fn, "leaf_str_found");
        let lw_int_hdr = self.context.append_basic_block(lc_walk_fn, "int_hdr");
        let lw_int_bdy = self.context.append_basic_block(lc_walk_fn, "int_bdy");
        let lw_int_next = self.context.append_basic_block(lc_walk_fn, "int_next");
        let lw_int_found = self.context.append_basic_block(lc_walk_fn, "int_found");
        let lw_miss = self.context.append_basic_block(lc_walk_fn, "miss");
        self.builder.position_at_end(lw_entry);
        let lw_node = lc_walk_fn.get_first_param().unwrap().into_pointer_value();
        let lw_height = lc_walk_fn.get_nth_param(1).unwrap().into_int_value();
        let lw_key = lc_walk_fn.get_nth_param(2).unwrap().into_struct_value();
        let lw_key_tag = self
            .builder
            .build_extract_value(lw_key, 0, "lw_ktag")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_key_data = self
            .builder
            .build_extract_value(lw_key, 1, "lw_kdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lw_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_height, zero, "lw_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_is_leaf, lw_leaf_hdr, lw_int_hdr);

        // Leaf scan
        self.builder.position_at_end(lw_leaf_hdr);
        let lw_leaf_i8 = self
            .builder
            .build_pointer_cast(lw_node, ptr, "lw_leaf_i8")
            .map_err(llvm_err)?;
        let lw_count_raw = self
            .builder
            .build_load(i32, lw_leaf_i8, "lw_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_count = self
            .builder
            .build_int_z_extend(lw_count_raw, i64, "lw_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lw_leaf_bdy);
        self.builder.position_at_end(lw_leaf_bdy);
        let lw_i = self.builder.build_phi(i64, "lw_i").map_err(llvm_err)?;
        let lw_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                lw_i.as_basic_value().into_int_value(),
                lw_count,
                "lw_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_done_leaf, lw_miss, lw_leaf_chk);
        self.builder.position_at_end(lw_leaf_chk);
        let lw_eb = unsafe {
            self.builder
                .build_gep(i8, lw_leaf_i8, &[i64.const_int(8, false)], "lw_eb")
                .map_err(llvm_err)?
        };
        let lw_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    lw_eb,
                    &[lw_i.as_basic_value().into_int_value()],
                    "lw_ep",
                )
                .map_err(llvm_err)?
        };
        let lw_elem = self
            .builder
            .build_load(self.string_type, lw_ep, "lw_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let lw_elem_tag = self
            .builder
            .build_extract_value(lw_elem, 0, "lw_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_elem_data = self
            .builder
            .build_extract_value(lw_elem, 1, "lw_edata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lw_tag_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_elem_tag, lw_key_tag, "lw_teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_tag_eq, lw_leaf_content, lw_leaf_next);
        self.builder.position_at_end(lw_leaf_content);
        let lw_null = self.ptr_ty().const_zero();
        let lw_ed_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_elem_data, lw_null, "lw_ed_null")
            .map_err(llvm_err)?;
        let lw_kd_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_key_data, lw_null, "lw_kd_null")
            .map_err(llvm_err)?;
        let lw_both_null = self
            .builder
            .build_and(lw_ed_null, lw_kd_null, "lw_both_null")
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lw_both_null, lw_leaf_found, lw_leaf_str_gate);
        self.builder.position_at_end(lw_leaf_str_gate);
        let lw_ed_nn = self
            .builder
            .build_not(lw_ed_null, "lw_ed_nn")
            .map_err(llvm_err)?;
        let lw_kd_nn = self
            .builder
            .build_not(lw_kd_null, "lw_kd_nn")
            .map_err(llvm_err)?;
        let lw_both_nn = self
            .builder
            .build_and(lw_ed_nn, lw_kd_nn, "lw_both_nn")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_both_nn, lw_leaf_str_cmp, lw_leaf_next);
        self.builder.position_at_end(lw_leaf_str_cmp);
        let lw_str_eq = self
            .call_rt(
                "action_string_eq",
                &[
                    lw_elem.as_basic_value_enum().into(),
                    lw_key.as_basic_value_enum().into(),
                ],
            )?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(lw_str_eq, lw_leaf_str_found, lw_leaf_next);
        self.builder.position_at_end(lw_leaf_found);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(lw_leaf_str_found);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(lw_leaf_next);
        let lw_next_i = self
            .builder
            .build_int_add(
                lw_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "lw_ni",
            )
            .map_err(llvm_err)?;
        let lw_leaf_next_bb = self.builder.get_insert_block().unwrap();
        lw_i.add_incoming(&[(&zero, lw_leaf_hdr), (&lw_next_i, lw_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(lw_leaf_bdy);

        // Internal node: recurse into each child in order
        self.builder.position_at_end(lw_int_hdr);
        let lw_int_i8 = self
            .builder
            .build_pointer_cast(lw_node, ptr, "lw_int_i8")
            .map_err(llvm_err)?;
        let lw_child_count_raw = self
            .builder
            .build_load(i32, lw_int_i8, "lw_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_child_count = self
            .builder
            .build_int_z_extend(lw_child_count_raw, i64, "lw_cc")
            .map_err(llvm_err)?;
        let lw_child_h = self
            .builder
            .build_int_sub(lw_height, i64.const_int(1, false), "lw_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lw_int_bdy);
        self.builder.position_at_end(lw_int_bdy);
        let lw_ci = self.builder.build_phi(i64, "lw_ci").map_err(llvm_err)?;
        let lw_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                lw_ci.as_basic_value().into_int_value(),
                lw_child_count,
                "lw_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_done_int, lw_miss, lw_int_found);
        self.builder.position_at_end(lw_int_found);
        let lw_children_base = unsafe {
            self.builder
                .build_gep(i8, lw_int_i8, &[i64.const_int(16, false)], "lw_cb")
                .map_err(llvm_err)?
        };
        let lw_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    lw_children_base,
                    &[lw_ci.as_basic_value().into_int_value()],
                    "lw_cep",
                )
                .map_err(llvm_err)?
        };
        let lw_child_entry = self
            .builder
            .build_load(self.child_entry_type, lw_child_ep, "lw_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let lw_child_ptr = self
            .builder
            .build_extract_value(lw_child_entry, 0, "lw_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lw_child_hit = self
            .builder
            .build_call(
                lc_walk_fn,
                &[lw_child_ptr.into(), lw_child_h.into(), lw_key.into()],
                "lw_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(lw_child_hit, lw_leaf_found, lw_int_next);
        self.builder.position_at_end(lw_int_next);
        let lw_next_ci = self
            .builder
            .build_int_add(
                lw_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "lw_nci",
            )
            .map_err(llvm_err)?;
        let lw_int_next_bb = self.builder.get_insert_block().unwrap();
        lw_ci.add_incoming(&[(&zero, lw_int_hdr), (&lw_next_ci, lw_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(lw_int_bdy);

        self.builder.position_at_end(lw_miss);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        // ---- action_list_map_walk_rec(ptr node, i64 height, ptr fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        // In-order B-tree scan: apply callback to each element, batch into leaf buffer.
        let lambda_fn_ty = self.string_type.fn_type(&[i64.into()], false);
        let push_leaf_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let mw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let mw_rec_fn = self.module.add_function(
            "action_list_map_walk_rec",
            void.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                ],
                false,
            ),
            None,
        );
        let mwr_entry = self.context.append_basic_block(mw_rec_fn, "entry");
        let mwr_leaf_hdr = self.context.append_basic_block(mw_rec_fn, "leaf_hdr");
        let mwr_leaf_bdy = self.context.append_basic_block(mw_rec_fn, "leaf_bdy");
        let mwr_leaf_chk = self.context.append_basic_block(mw_rec_fn, "leaf_chk");
        let mwr_leaf_flush = self.context.append_basic_block(mw_rec_fn, "leaf_flush");
        let mwr_leaf_next = self.context.append_basic_block(mw_rec_fn, "leaf_next");
        let mwr_leaf_done = self.context.append_basic_block(mw_rec_fn, "leaf_done");
        let mwr_int_hdr = self.context.append_basic_block(mw_rec_fn, "int_hdr");
        let mwr_int_bdy = self.context.append_basic_block(mw_rec_fn, "int_bdy");
        let mwr_int_child = self.context.append_basic_block(mw_rec_fn, "int_child");
        let mwr_int_next = self.context.append_basic_block(mw_rec_fn, "int_next");
        let mwr_concat = self.context.append_basic_block(mw_rec_fn, "concat");
        let mwr_normal = self.context.append_basic_block(mw_rec_fn, "normal");
        self.builder.position_at_end(mwr_entry);
        let mwr_node = mw_rec_fn.get_first_param().unwrap().into_pointer_value();
        let mwr_height = mw_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let mwr_fn = mw_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let mwr_acc = mw_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let mwr_buf_p = mw_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let mwr_buf_pos_p = mw_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let mwr_neg1 = i64.const_int(-1i64 as u64, true);
        let mwr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, mwr_height, mwr_neg1, "mwr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_is_concat, mwr_concat, mwr_normal);
        self.builder.position_at_end(mwr_concat);
        let mwr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, mwr_node, &[i64.const_int(2, false)], "mwr_ln_p")
                .map_err(llvm_err)
        }?;
        let mwr_left_node = self
            .builder
            .build_load(ptr, mwr_ln_p, "mwr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mwr_lh_p = unsafe {
            self.builder
                .build_gep(i64, mwr_node, &[i64.const_int(4, false)], "mwr_lh_p")
                .map_err(llvm_err)
        }?;
        let mwr_left_h = self
            .builder
            .build_load(i64, mwr_lh_p, "mwr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, mwr_node, &[i64.const_int(5, false)], "mwr_rn_p")
                .map_err(llvm_err)
        }?;
        let mwr_right_node = self
            .builder
            .build_load(ptr, mwr_rn_p, "mwr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mwr_rh_p = unsafe {
            self.builder
                .build_gep(i64, mwr_node, &[i64.const_int(7, false)], "mwr_rh_p")
                .map_err(llvm_err)
        }?;
        let mwr_right_h = self
            .builder
            .build_load(i64, mwr_rh_p, "mwr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                mw_rec_fn,
                &[
                    mwr_left_node.into(),
                    mwr_left_h.into(),
                    mwr_fn.into(),
                    mwr_acc.into(),
                    mwr_buf_p.into(),
                    mwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                mw_rec_fn,
                &[
                    mwr_right_node.into(),
                    mwr_right_h.into(),
                    mwr_fn.into(),
                    mwr_acc.into(),
                    mwr_buf_p.into(),
                    mwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(mwr_normal);
        let mwr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, mwr_height, zero, "mwr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_is_leaf, mwr_leaf_hdr, mwr_int_hdr);

        // Leaf scan
        self.builder.position_at_end(mwr_leaf_hdr);
        let mwr_leaf_i8 = self
            .builder
            .build_pointer_cast(mwr_node, ptr, "mwr_leaf_i8")
            .map_err(llvm_err)?;
        let mwr_count_raw = self
            .builder
            .build_load(i32, mwr_leaf_i8, "mwr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_count = self
            .builder
            .build_int_z_extend(mwr_count_raw, i64, "mwr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mwr_leaf_bdy);
        self.builder.position_at_end(mwr_leaf_bdy);
        let mwr_i = self.builder.build_phi(i64, "mwr_i").map_err(llvm_err)?;
        let mwr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mwr_i.as_basic_value().into_int_value(),
                mwr_count,
                "mwr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_done_leaf, mwr_leaf_done, mwr_leaf_chk);
        self.builder.position_at_end(mwr_leaf_chk);
        let mwr_eb = unsafe {
            self.builder
                .build_gep(i8, mwr_leaf_i8, &[i64.const_int(8, false)], "mwr_eb")
                .map_err(llvm_err)?
        };
        let mwr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    mwr_eb,
                    &[mwr_i.as_basic_value().into_int_value()],
                    "mwr_ep",
                )
                .map_err(llvm_err)?
        };
        let mwr_elem = self
            .builder
            .build_load(self.string_type, mwr_ep, "mwr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let mwr_elem_tag = self
            .builder
            .build_extract_value(mwr_elem, 0, "mwr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_mapped = self
            .builder
            .build_indirect_call(lambda_fn_ty, mwr_fn, &[mwr_elem_tag.into()], "mwr_mapped")
            .map_err(llvm_err)?;
        let mwr_mapped_bv = mwr_mapped
            .try_as_basic_value()
            .basic()
            .ok_or("map_walk indirect call failed")?;
        let mwr_buf = self
            .builder
            .build_load(ptr, mwr_buf_p, "mwr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mwr_pos = self
            .builder
            .build_load(i64, mwr_buf_pos_p, "mwr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_buf_i8 = self
            .builder
            .build_pointer_cast(mwr_buf, ptr, "mwr_buf_i8")
            .map_err(llvm_err)?;
        let mwr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, mwr_buf_i8, &[i64.const_int(8, false)], "mwr_buf_eb")
                .map_err(llvm_err)?
        };
        let mwr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, mwr_buf_eb, &[mwr_pos], "mwr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(mwr_buf_ep, mwr_mapped_bv)
            .map_err(llvm_err)?;
        let mwr_pos_inc = self
            .builder
            .build_int_add(mwr_pos, i64.const_int(1, false), "mwr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(mwr_buf_pos_p, mwr_pos_inc)
            .map_err(llvm_err)?;
        let mwr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                mwr_pos_inc,
                i64.const_int(64, false),
                "mwr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_buf_full, mwr_leaf_flush, mwr_leaf_next);

        self.builder.position_at_end(mwr_leaf_flush);
        let mwr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(mwr_buf_i8, mwr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mwr_acc.into(), mwr_buf.into()], "")
            .map_err(llvm_err)?;
        let mwr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[mw_leaf_sz.into()], "mwr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mwr_new_buf_i8 = self
            .builder
            .build_pointer_cast(mwr_new_buf, ptr, "mwr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mwr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mwr_buf_p, mwr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mwr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mwr_leaf_next);

        self.builder.position_at_end(mwr_leaf_next);
        let mwr_next_i = self
            .builder
            .build_int_add(
                mwr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mwr_ni",
            )
            .map_err(llvm_err)?;
        let mwr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        mwr_i.add_incoming(&[(&zero, mwr_leaf_hdr), (&mwr_next_i, mwr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mwr_leaf_bdy);
        self.builder.position_at_end(mwr_leaf_done);
        let _ = self.builder.build_return(None);

        // Internal node: recurse into each child in order
        self.builder.position_at_end(mwr_int_hdr);
        let mwr_int_i8 = self
            .builder
            .build_pointer_cast(mwr_node, ptr, "mwr_int_i8")
            .map_err(llvm_err)?;
        let mwr_child_count_raw = self
            .builder
            .build_load(i32, mwr_int_i8, "mwr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_child_count = self
            .builder
            .build_int_z_extend(mwr_child_count_raw, i64, "mwr_cc")
            .map_err(llvm_err)?;
        let mwr_child_h = self
            .builder
            .build_int_sub(mwr_height, i64.const_int(1, false), "mwr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mwr_int_bdy);
        self.builder.position_at_end(mwr_int_bdy);
        let mwr_ci = self.builder.build_phi(i64, "mwr_ci").map_err(llvm_err)?;
        let mwr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mwr_ci.as_basic_value().into_int_value(),
                mwr_child_count,
                "mwr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_done_int, mwr_leaf_done, mwr_int_child);
        self.builder.position_at_end(mwr_int_child);
        let mwr_children_base = unsafe {
            self.builder
                .build_gep(i8, mwr_int_i8, &[i64.const_int(16, false)], "mwr_cb")
                .map_err(llvm_err)?
        };
        let mwr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    mwr_children_base,
                    &[mwr_ci.as_basic_value().into_int_value()],
                    "mwr_cep",
                )
                .map_err(llvm_err)?
        };
        let mwr_child_entry = self
            .builder
            .build_load(self.child_entry_type, mwr_child_ep, "mwr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let mwr_child_ptr = self
            .builder
            .build_extract_value(mwr_child_entry, 0, "mwr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                mw_rec_fn,
                &[
                    mwr_child_ptr.into(),
                    mwr_child_h.into(),
                    mwr_fn.into(),
                    mwr_acc.into(),
                    mwr_buf_p.into(),
                    mwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mwr_int_next);
        self.builder.position_at_end(mwr_int_next);
        let mwr_next_ci = self
            .builder
            .build_int_add(
                mwr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mwr_nci",
            )
            .map_err(llvm_err)?;
        let mwr_int_next_bb = self.builder.get_insert_block().unwrap();
        mwr_ci.add_incoming(&[(&zero, mwr_int_hdr), (&mwr_next_ci, mwr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mwr_int_bdy);

        // ---- action_list_map_walk({ptr,i64,i64} list, ptr fn) -> {ptr,i64,i64} ----
        let create_fn = self.module.get_function("action_list_create").unwrap();
        let mw_fn = self.module.add_function(
            "action_list_map_walk",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let mw_entry = self.context.append_basic_block(mw_fn, "entry");
        let mw_walk = self.context.append_basic_block(mw_fn, "walk");
        let mw_flush = self.context.append_basic_block(mw_fn, "flush");
        let mw_done = self.context.append_basic_block(mw_fn, "done");
        self.builder.position_at_end(mw_entry);
        let mw_list = mw_fn.get_first_param().unwrap().into_struct_value();
        let mw_fn_ptr = mw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let mw_node = self
            .builder
            .build_extract_value(mw_list, 0, "mw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mw_len = self
            .builder
            .build_extract_value(mw_list, 1, "mw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let mw_height = self
            .builder
            .build_extract_value(mw_list, 2, "mw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let mw_acc = self
            .builder
            .build_alloca(self.list_type, "mw_acc")
            .map_err(llvm_err)?;
        let mw_buf_p = self
            .builder
            .build_alloca(ptr, "mw_buf_p")
            .map_err(llvm_err)?;
        let mw_buf_pos_p = self
            .builder
            .build_alloca(i64, "mw_buf_pos_p")
            .map_err(llvm_err)?;
        let mw_init = self
            .builder
            .build_call(create_fn, &[mw_len.into()], "mw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(mw_acc, mw_init)
            .map_err(llvm_err)?;
        let mw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[mw_leaf_sz.into()], "mw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mw_buf_init_i8 = self
            .builder
            .build_pointer_cast(mw_buf_init, ptr, "mw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mw_buf_p, mw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mw_walk);
        self.builder.position_at_end(mw_walk);
        let _ = self
            .builder
            .build_call(
                mw_rec_fn,
                &[
                    mw_node.into(),
                    mw_height.into(),
                    mw_fn_ptr.into(),
                    mw_acc.into(),
                    mw_buf_p.into(),
                    mw_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let mw_rem_pos = self
            .builder
            .build_load(i64, mw_buf_pos_p, "mw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, mw_rem_pos, zero, "mw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mw_has_rem, mw_flush, mw_done);
        self.builder.position_at_end(mw_flush);
        let mw_rem_buf = self
            .builder
            .build_load(ptr, mw_buf_p, "mw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(mw_rem_buf, ptr, "mw_rem_buf_i8")
            .map_err(llvm_err)?;
        let mw_rem_cnt = self
            .builder
            .build_int_truncate(mw_rem_pos, i32, "mw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mw_rem_buf_i8, mw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mw_acc.into(), mw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mw_done);
        self.builder.position_at_end(mw_done);
        let mw_res = self
            .builder
            .build_load(self.list_type, mw_acc, "mw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mw_res));

        // ---- action_list_filter_walk_rec(ptr node, i64 height, ptr fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        let fw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let fw_rec_fn = self.module.add_function(
            "action_list_filter_walk_rec",
            void.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                ],
                false,
            ),
            None,
        );
        let fwr_entry = self.context.append_basic_block(fw_rec_fn, "entry");
        let fwr_leaf_hdr = self.context.append_basic_block(fw_rec_fn, "leaf_hdr");
        let fwr_leaf_bdy = self.context.append_basic_block(fw_rec_fn, "leaf_bdy");
        let fwr_leaf_chk = self.context.append_basic_block(fw_rec_fn, "leaf_chk");
        let fwr_leaf_push = self.context.append_basic_block(fw_rec_fn, "leaf_push");
        let fwr_leaf_flush = self.context.append_basic_block(fw_rec_fn, "leaf_flush");
        let fwr_leaf_next = self.context.append_basic_block(fw_rec_fn, "leaf_next");
        let fwr_leaf_done = self.context.append_basic_block(fw_rec_fn, "leaf_done");
        let fwr_int_hdr = self.context.append_basic_block(fw_rec_fn, "int_hdr");
        let fwr_int_bdy = self.context.append_basic_block(fw_rec_fn, "int_bdy");
        let fwr_int_child = self.context.append_basic_block(fw_rec_fn, "int_child");
        let fwr_int_next = self.context.append_basic_block(fw_rec_fn, "int_next");
        let fwr_concat = self.context.append_basic_block(fw_rec_fn, "concat");
        let fwr_normal = self.context.append_basic_block(fw_rec_fn, "normal");
        self.builder.position_at_end(fwr_entry);
        let fwr_node = fw_rec_fn.get_first_param().unwrap().into_pointer_value();
        let fwr_height = fw_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let fwr_fn = fw_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let fwr_acc = fw_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let fwr_buf_p = fw_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let fwr_buf_pos_p = fw_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let fwr_neg1 = i64.const_int(-1i64 as u64, true);
        let fwr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, fwr_height, fwr_neg1, "fwr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_is_concat, fwr_concat, fwr_normal);
        self.builder.position_at_end(fwr_concat);
        let fwr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, fwr_node, &[i64.const_int(2, false)], "fwr_ln_p")
                .map_err(llvm_err)
        }?;
        let fwr_left_node = self
            .builder
            .build_load(ptr, fwr_ln_p, "fwr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fwr_lh_p = unsafe {
            self.builder
                .build_gep(i64, fwr_node, &[i64.const_int(4, false)], "fwr_lh_p")
                .map_err(llvm_err)
        }?;
        let fwr_left_h = self
            .builder
            .build_load(i64, fwr_lh_p, "fwr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, fwr_node, &[i64.const_int(5, false)], "fwr_rn_p")
                .map_err(llvm_err)
        }?;
        let fwr_right_node = self
            .builder
            .build_load(ptr, fwr_rn_p, "fwr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fwr_rh_p = unsafe {
            self.builder
                .build_gep(i64, fwr_node, &[i64.const_int(7, false)], "fwr_rh_p")
                .map_err(llvm_err)
        }?;
        let fwr_right_h = self
            .builder
            .build_load(i64, fwr_rh_p, "fwr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                fw_rec_fn,
                &[
                    fwr_left_node.into(),
                    fwr_left_h.into(),
                    fwr_fn.into(),
                    fwr_acc.into(),
                    fwr_buf_p.into(),
                    fwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                fw_rec_fn,
                &[
                    fwr_right_node.into(),
                    fwr_right_h.into(),
                    fwr_fn.into(),
                    fwr_acc.into(),
                    fwr_buf_p.into(),
                    fwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(fwr_normal);
        let fwr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, fwr_height, zero, "fwr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_is_leaf, fwr_leaf_hdr, fwr_int_hdr);

        self.builder.position_at_end(fwr_leaf_hdr);
        let fwr_leaf_i8 = self
            .builder
            .build_pointer_cast(fwr_node, ptr, "fwr_leaf_i8")
            .map_err(llvm_err)?;
        let fwr_count_raw = self
            .builder
            .build_load(i32, fwr_leaf_i8, "fwr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_count = self
            .builder
            .build_int_z_extend(fwr_count_raw, i64, "fwr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fwr_leaf_bdy);
        self.builder.position_at_end(fwr_leaf_bdy);
        let fwr_i = self.builder.build_phi(i64, "fwr_i").map_err(llvm_err)?;
        let fwr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fwr_i.as_basic_value().into_int_value(),
                fwr_count,
                "fwr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_done_leaf, fwr_leaf_done, fwr_leaf_chk);
        self.builder.position_at_end(fwr_leaf_chk);
        let fwr_eb = unsafe {
            self.builder
                .build_gep(i8, fwr_leaf_i8, &[i64.const_int(8, false)], "fwr_eb")
                .map_err(llvm_err)?
        };
        let fwr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    fwr_eb,
                    &[fwr_i.as_basic_value().into_int_value()],
                    "fwr_ep",
                )
                .map_err(llvm_err)?
        };
        let fwr_elem = self
            .builder
            .build_load(self.string_type, fwr_ep, "fwr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let fwr_elem_tag = self
            .builder
            .build_extract_value(fwr_elem, 0, "fwr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_pred = self
            .builder
            .build_indirect_call(lambda_fn_ty, fwr_fn, &[fwr_elem_tag.into()], "fwr_pred")
            .map_err(llvm_err)?;
        let fwr_pred_bv = fwr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("filter_walk indirect call failed")?;
        let fwr_pred_val = if fwr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(fwr_pred_bv.into_struct_value(), 0, "fwr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            fwr_pred_bv.into_int_value()
        };
        let fwr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, fwr_pred_val, zero, "fwr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_is_true, fwr_leaf_push, fwr_leaf_next);
        self.builder.position_at_end(fwr_leaf_push);
        let fwr_buf = self
            .builder
            .build_load(ptr, fwr_buf_p, "fwr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fwr_pos = self
            .builder
            .build_load(i64, fwr_buf_pos_p, "fwr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_buf_i8 = self
            .builder
            .build_pointer_cast(fwr_buf, ptr, "fwr_buf_i8")
            .map_err(llvm_err)?;
        let fwr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, fwr_buf_i8, &[i64.const_int(8, false)], "fwr_buf_eb")
                .map_err(llvm_err)?
        };
        let fwr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, fwr_buf_eb, &[fwr_pos], "fwr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(fwr_buf_ep, fwr_elem)
            .map_err(llvm_err)?;
        let fwr_pos_inc = self
            .builder
            .build_int_add(fwr_pos, i64.const_int(1, false), "fwr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(fwr_buf_pos_p, fwr_pos_inc)
            .map_err(llvm_err)?;
        let fwr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                fwr_pos_inc,
                i64.const_int(64, false),
                "fwr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_buf_full, fwr_leaf_flush, fwr_leaf_next);

        self.builder.position_at_end(fwr_leaf_flush);
        let fwr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(fwr_buf_i8, fwr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[fwr_acc.into(), fwr_buf.into()], "")
            .map_err(llvm_err)?;
        let fwr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[fw_leaf_sz.into()], "fwr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fwr_new_buf_i8 = self
            .builder
            .build_pointer_cast(fwr_new_buf, ptr, "fwr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fwr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fwr_buf_p, fwr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fwr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fwr_leaf_next);

        self.builder.position_at_end(fwr_leaf_next);
        let fwr_next_i = self
            .builder
            .build_int_add(
                fwr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fwr_ni",
            )
            .map_err(llvm_err)?;
        let fwr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        fwr_i.add_incoming(&[(&zero, fwr_leaf_hdr), (&fwr_next_i, fwr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fwr_leaf_bdy);
        self.builder.position_at_end(fwr_leaf_done);
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(fwr_int_hdr);
        let fwr_int_i8 = self
            .builder
            .build_pointer_cast(fwr_node, ptr, "fwr_int_i8")
            .map_err(llvm_err)?;
        let fwr_child_count_raw = self
            .builder
            .build_load(i32, fwr_int_i8, "fwr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_child_count = self
            .builder
            .build_int_z_extend(fwr_child_count_raw, i64, "fwr_cc")
            .map_err(llvm_err)?;
        let fwr_child_h = self
            .builder
            .build_int_sub(fwr_height, i64.const_int(1, false), "fwr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fwr_int_bdy);
        self.builder.position_at_end(fwr_int_bdy);
        let fwr_ci = self.builder.build_phi(i64, "fwr_ci").map_err(llvm_err)?;
        let fwr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fwr_ci.as_basic_value().into_int_value(),
                fwr_child_count,
                "fwr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_done_int, fwr_leaf_done, fwr_int_child);
        self.builder.position_at_end(fwr_int_child);
        let fwr_children_base = unsafe {
            self.builder
                .build_gep(i8, fwr_int_i8, &[i64.const_int(16, false)], "fwr_cb")
                .map_err(llvm_err)?
        };
        let fwr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    fwr_children_base,
                    &[fwr_ci.as_basic_value().into_int_value()],
                    "fwr_cep",
                )
                .map_err(llvm_err)?
        };
        let fwr_child_entry = self
            .builder
            .build_load(self.child_entry_type, fwr_child_ep, "fwr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let fwr_child_ptr = self
            .builder
            .build_extract_value(fwr_child_entry, 0, "fwr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                fw_rec_fn,
                &[
                    fwr_child_ptr.into(),
                    fwr_child_h.into(),
                    fwr_fn.into(),
                    fwr_acc.into(),
                    fwr_buf_p.into(),
                    fwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fwr_int_next);
        self.builder.position_at_end(fwr_int_next);
        let fwr_next_ci = self
            .builder
            .build_int_add(
                fwr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fwr_nci",
            )
            .map_err(llvm_err)?;
        let fwr_int_next_bb = self.builder.get_insert_block().unwrap();
        fwr_ci.add_incoming(&[(&zero, fwr_int_hdr), (&fwr_next_ci, fwr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fwr_int_bdy);

        // ---- action_list_filter_walk({ptr,i64,i64} list, ptr fn) -> {ptr,i64,i64} ----
        let fw_fn = self.module.add_function(
            "action_list_filter_walk",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let fw_entry = self.context.append_basic_block(fw_fn, "entry");
        let fw_walk = self.context.append_basic_block(fw_fn, "walk");
        let fw_flush = self.context.append_basic_block(fw_fn, "flush");
        let fw_done = self.context.append_basic_block(fw_fn, "done");
        self.builder.position_at_end(fw_entry);
        let fw_list = fw_fn.get_first_param().unwrap().into_struct_value();
        let fw_fn_ptr = fw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fw_node = self
            .builder
            .build_extract_value(fw_list, 0, "fw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fw_len = self
            .builder
            .build_extract_value(fw_list, 1, "fw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let fw_height = self
            .builder
            .build_extract_value(fw_list, 2, "fw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let fw_acc = self
            .builder
            .build_alloca(self.list_type, "fw_acc")
            .map_err(llvm_err)?;
        let fw_buf_p = self
            .builder
            .build_alloca(ptr, "fw_buf_p")
            .map_err(llvm_err)?;
        let fw_buf_pos_p = self
            .builder
            .build_alloca(i64, "fw_buf_pos_p")
            .map_err(llvm_err)?;
        let fw_init = self
            .builder
            .build_call(create_fn, &[fw_len.into()], "fw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(fw_acc, fw_init)
            .map_err(llvm_err)?;
        let fw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[fw_leaf_sz.into()], "fw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fw_buf_init_i8 = self
            .builder
            .build_pointer_cast(fw_buf_init, ptr, "fw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fw_buf_p, fw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fw_walk);
        self.builder.position_at_end(fw_walk);
        let _ = self
            .builder
            .build_call(
                fw_rec_fn,
                &[
                    fw_node.into(),
                    fw_height.into(),
                    fw_fn_ptr.into(),
                    fw_acc.into(),
                    fw_buf_p.into(),
                    fw_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let fw_rem_pos = self
            .builder
            .build_load(i64, fw_buf_pos_p, "fw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let fw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, fw_rem_pos, zero, "fw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fw_has_rem, fw_flush, fw_done);
        self.builder.position_at_end(fw_flush);
        let fw_rem_buf = self
            .builder
            .build_load(ptr, fw_buf_p, "fw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(fw_rem_buf, ptr, "fw_rem_buf_i8")
            .map_err(llvm_err)?;
        let fw_rem_cnt = self
            .builder
            .build_int_truncate(fw_rem_pos, i32, "fw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fw_rem_buf_i8, fw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[fw_acc.into(), fw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fw_done);
        self.builder.position_at_end(fw_done);
        let fw_res = self
            .builder
            .build_load(self.list_type, fw_acc, "fw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fw_res));

        // ---- action_list_fold_walk_rec / action_list_fold_walk ----
        // Int accumulator fast path: (i64, i64) -> i64 direct call, no fat-struct load/return.
        let fold_fn_ty = i64.fn_type(&[i64.into(), i64.into()], false);
        let fd_rec_fn = self.module.add_function(
            "action_list_fold_walk_rec",
            void.fn_type(&[ptr.into(), i64.into(), ptr.into(), ptr.into()], false),
            None,
        );
        let fdr_entry = self.context.append_basic_block(fd_rec_fn, "entry");
        let fdr_leaf_hdr = self.context.append_basic_block(fd_rec_fn, "leaf_hdr");
        let fdr_leaf_bdy = self.context.append_basic_block(fd_rec_fn, "leaf_bdy");
        let fdr_leaf_chk = self.context.append_basic_block(fd_rec_fn, "leaf_chk");
        let fdr_leaf_next = self.context.append_basic_block(fd_rec_fn, "leaf_next");
        let fdr_leaf_done = self.context.append_basic_block(fd_rec_fn, "leaf_done");
        let fdr_int_hdr = self.context.append_basic_block(fd_rec_fn, "int_hdr");
        let fdr_int_bdy = self.context.append_basic_block(fd_rec_fn, "int_bdy");
        let fdr_int_child = self.context.append_basic_block(fd_rec_fn, "int_child");
        let fdr_int_next = self.context.append_basic_block(fd_rec_fn, "int_next");
        let fdr_concat = self.context.append_basic_block(fd_rec_fn, "concat");
        let fdr_normal = self.context.append_basic_block(fd_rec_fn, "normal");
        self.builder.position_at_end(fdr_entry);
        let fdr_node = fd_rec_fn.get_first_param().unwrap().into_pointer_value();
        let fdr_height = fd_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let fdr_fn = fd_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let fdr_acc = fd_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let fdr_neg1 = i64.const_int(-1i64 as u64, true);
        let fdr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, fdr_height, fdr_neg1, "fdr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_is_concat, fdr_concat, fdr_normal);
        self.builder.position_at_end(fdr_concat);
        let fdr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, fdr_node, &[i64.const_int(2, false)], "fdr_ln_p")
                .map_err(llvm_err)
        }?;
        let fdr_left_node = self
            .builder
            .build_load(ptr, fdr_ln_p, "fdr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fdr_lh_p = unsafe {
            self.builder
                .build_gep(i64, fdr_node, &[i64.const_int(4, false)], "fdr_lh_p")
                .map_err(llvm_err)
        }?;
        let fdr_left_h = self
            .builder
            .build_load(i64, fdr_lh_p, "fdr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, fdr_node, &[i64.const_int(5, false)], "fdr_rn_p")
                .map_err(llvm_err)
        }?;
        let fdr_right_node = self
            .builder
            .build_load(ptr, fdr_rn_p, "fdr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fdr_rh_p = unsafe {
            self.builder
                .build_gep(i64, fdr_node, &[i64.const_int(7, false)], "fdr_rh_p")
                .map_err(llvm_err)
        }?;
        let fdr_right_h = self
            .builder
            .build_load(i64, fdr_rh_p, "fdr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                fd_rec_fn,
                &[
                    fdr_left_node.into(),
                    fdr_left_h.into(),
                    fdr_fn.into(),
                    fdr_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                fd_rec_fn,
                &[
                    fdr_right_node.into(),
                    fdr_right_h.into(),
                    fdr_fn.into(),
                    fdr_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(fdr_normal);
        let fdr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, fdr_height, zero, "fdr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_is_leaf, fdr_leaf_hdr, fdr_int_hdr);
        self.builder.position_at_end(fdr_leaf_hdr);
        let fdr_leaf_i8 = self
            .builder
            .build_pointer_cast(fdr_node, ptr, "fdr_leaf_i8")
            .map_err(llvm_err)?;
        let fdr_count_raw = self
            .builder
            .build_load(i32, fdr_leaf_i8, "fdr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_count = self
            .builder
            .build_int_z_extend(fdr_count_raw, i64, "fdr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fdr_leaf_bdy);
        self.builder.position_at_end(fdr_leaf_bdy);
        let fdr_i = self.builder.build_phi(i64, "fdr_i").map_err(llvm_err)?;
        let fdr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fdr_i.as_basic_value().into_int_value(),
                fdr_count,
                "fdr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_done_leaf, fdr_leaf_done, fdr_leaf_chk);
        self.builder.position_at_end(fdr_leaf_chk);
        let fdr_eb = unsafe {
            self.builder
                .build_gep(i8, fdr_leaf_i8, &[i64.const_int(8, false)], "fdr_eb")
                .map_err(llvm_err)?
        };
        let fdr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    fdr_eb,
                    &[fdr_i.as_basic_value().into_int_value()],
                    "fdr_ep",
                )
                .map_err(llvm_err)?
        };
        let fdr_elem_tag = self
            .builder
            .build_load(i64, fdr_ep, "fdr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_cur_acc = self
            .builder
            .build_load(i64, fdr_acc, "fdr_acc")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_new_acc = self
            .builder
            .build_indirect_call(
                fold_fn_ty,
                fdr_fn,
                &[fdr_cur_acc.into(), fdr_elem_tag.into()],
                "fdr_folded",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("fold_walk indirect call failed")?
            .into_int_value();
        self.builder
            .build_store(fdr_acc, fdr_new_acc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fdr_leaf_next);
        self.builder.position_at_end(fdr_leaf_next);
        let fdr_next_i = self
            .builder
            .build_int_add(
                fdr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fdr_ni",
            )
            .map_err(llvm_err)?;
        let fdr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        fdr_i.add_incoming(&[(&zero, fdr_leaf_hdr), (&fdr_next_i, fdr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fdr_leaf_bdy);
        self.builder.position_at_end(fdr_leaf_done);
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(fdr_int_hdr);
        let fdr_int_i8 = self
            .builder
            .build_pointer_cast(fdr_node, ptr, "fdr_int_i8")
            .map_err(llvm_err)?;
        let fdr_child_count_raw = self
            .builder
            .build_load(i32, fdr_int_i8, "fdr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_child_count = self
            .builder
            .build_int_z_extend(fdr_child_count_raw, i64, "fdr_cc")
            .map_err(llvm_err)?;
        let fdr_child_h = self
            .builder
            .build_int_sub(fdr_height, i64.const_int(1, false), "fdr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fdr_int_bdy);
        self.builder.position_at_end(fdr_int_bdy);
        let fdr_ci = self.builder.build_phi(i64, "fdr_ci").map_err(llvm_err)?;
        let fdr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fdr_ci.as_basic_value().into_int_value(),
                fdr_child_count,
                "fdr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_done_int, fdr_leaf_done, fdr_int_child);
        self.builder.position_at_end(fdr_int_child);
        let fdr_children_base = unsafe {
            self.builder
                .build_gep(i8, fdr_int_i8, &[i64.const_int(16, false)], "fdr_cb")
                .map_err(llvm_err)?
        };
        let fdr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    fdr_children_base,
                    &[fdr_ci.as_basic_value().into_int_value()],
                    "fdr_cep",
                )
                .map_err(llvm_err)?
        };
        let fdr_child_entry = self
            .builder
            .build_load(self.child_entry_type, fdr_child_ep, "fdr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let fdr_child_ptr = self
            .builder
            .build_extract_value(fdr_child_entry, 0, "fdr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                fd_rec_fn,
                &[
                    fdr_child_ptr.into(),
                    fdr_child_h.into(),
                    fdr_fn.into(),
                    fdr_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fdr_int_next);
        self.builder.position_at_end(fdr_int_next);
        let fdr_next_ci = self
            .builder
            .build_int_add(
                fdr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fdr_nci",
            )
            .map_err(llvm_err)?;
        let fdr_int_next_bb = self.builder.get_insert_block().unwrap();
        fdr_ci.add_incoming(&[(&zero, fdr_int_hdr), (&fdr_next_ci, fdr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fdr_int_bdy);

        let fd_fn = self.module.add_function(
            "action_list_fold_walk",
            i64.fn_type(&[self.list_type.into(), ptr.into(), i64.into()], false),
            None,
        );
        let fd_entry = self.context.append_basic_block(fd_fn, "entry");
        let fd_walk = self.context.append_basic_block(fd_fn, "walk");
        self.builder.position_at_end(fd_entry);
        let fd_list = fd_fn.get_first_param().unwrap().into_struct_value();
        let fd_fn_ptr = fd_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fd_init = fd_fn.get_nth_param(2).unwrap().into_int_value();
        let fd_node = self
            .builder
            .build_extract_value(fd_list, 0, "fd_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fd_height = self
            .builder
            .build_extract_value(fd_list, 2, "fd_height")
            .map_err(llvm_err)?
            .into_int_value();
        let fd_acc = self.builder.build_alloca(i64, "fd_acc").map_err(llvm_err)?;
        self.builder
            .build_store(fd_acc, fd_init)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fd_walk);
        self.builder.position_at_end(fd_walk);
        let _ = self
            .builder
            .build_call(
                fd_rec_fn,
                &[
                    fd_node.into(),
                    fd_height.into(),
                    fd_fn_ptr.into(),
                    fd_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let fd_res = self
            .builder
            .build_load(i64, fd_acc, "fd_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fd_res));

        // ---- action_list_any_walk_rec / action_list_any_walk ----
        let ay_rec_fn = self.module.add_function(
            "action_list_any_walk_rec",
            b1.fn_type(&[ptr.into(), i64.into(), ptr.into()], false),
            None,
        );
        let ayr_entry = self.context.append_basic_block(ay_rec_fn, "entry");
        let ayr_true = self.context.append_basic_block(ay_rec_fn, "any_true");
        let ayr_false = self.context.append_basic_block(ay_rec_fn, "any_false");
        let ayr_leaf_hdr = self.context.append_basic_block(ay_rec_fn, "leaf_hdr");
        let ayr_leaf_bdy = self.context.append_basic_block(ay_rec_fn, "leaf_bdy");
        let ayr_leaf_chk = self.context.append_basic_block(ay_rec_fn, "leaf_chk");
        let ayr_leaf_next = self.context.append_basic_block(ay_rec_fn, "leaf_next");
        let ayr_int_hdr = self.context.append_basic_block(ay_rec_fn, "int_hdr");
        let ayr_int_bdy = self.context.append_basic_block(ay_rec_fn, "int_bdy");
        let ayr_int_child = self.context.append_basic_block(ay_rec_fn, "int_child");
        let ayr_int_next = self.context.append_basic_block(ay_rec_fn, "int_next");
        let ayr_concat = self.context.append_basic_block(ay_rec_fn, "concat");
        let ayr_concat_right = self.context.append_basic_block(ay_rec_fn, "concat_right");
        let ayr_normal = self.context.append_basic_block(ay_rec_fn, "normal");
        self.builder.position_at_end(ayr_entry);
        let ayr_node = ay_rec_fn.get_first_param().unwrap().into_pointer_value();
        let ayr_height = ay_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let ayr_fn = ay_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let ayr_neg1 = i64.const_int(-1i64 as u64, true);
        let ayr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, ayr_height, ayr_neg1, "ayr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_is_concat, ayr_concat, ayr_normal);
        self.builder.position_at_end(ayr_concat);
        let ayr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, ayr_node, &[i64.const_int(2, false)], "ayr_ln_p")
                .map_err(llvm_err)
        }?;
        let ayr_left_node = self
            .builder
            .build_load(ptr, ayr_ln_p, "ayr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ayr_lh_p = unsafe {
            self.builder
                .build_gep(i64, ayr_node, &[i64.const_int(4, false)], "ayr_lh_p")
                .map_err(llvm_err)
        }?;
        let ayr_left_h = self
            .builder
            .build_load(i64, ayr_lh_p, "ayr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, ayr_node, &[i64.const_int(5, false)], "ayr_rn_p")
                .map_err(llvm_err)
        }?;
        let ayr_right_node = self
            .builder
            .build_load(ptr, ayr_rn_p, "ayr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ayr_rh_p = unsafe {
            self.builder
                .build_gep(i64, ayr_node, &[i64.const_int(7, false)], "ayr_rh_p")
                .map_err(llvm_err)
        }?;
        let ayr_right_h = self
            .builder
            .build_load(i64, ayr_rh_p, "ayr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_lhit = self
            .builder
            .build_call(
                ay_rec_fn,
                &[ayr_left_node.into(), ayr_left_h.into(), ayr_fn.into()],
                "ayr_lhit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(ayr_lhit, ayr_true, ayr_concat_right);
        self.builder.position_at_end(ayr_concat_right);
        let ayr_rhit = self
            .builder
            .build_call(
                ay_rec_fn,
                &[ayr_right_node.into(), ayr_right_h.into(), ayr_fn.into()],
                "ayr_rhit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&ayr_rhit));
        self.builder.position_at_end(ayr_normal);
        let ayr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, ayr_height, zero, "ayr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_is_leaf, ayr_leaf_hdr, ayr_int_hdr);
        self.builder.position_at_end(ayr_leaf_hdr);
        let ayr_leaf_i8 = self
            .builder
            .build_pointer_cast(ayr_node, ptr, "ayr_leaf_i8")
            .map_err(llvm_err)?;
        let ayr_count_raw = self
            .builder
            .build_load(i32, ayr_leaf_i8, "ayr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_count = self
            .builder
            .build_int_z_extend(ayr_count_raw, i64, "ayr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ayr_leaf_bdy);
        self.builder.position_at_end(ayr_leaf_bdy);
        let ayr_i = self.builder.build_phi(i64, "ayr_i").map_err(llvm_err)?;
        let ayr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                ayr_i.as_basic_value().into_int_value(),
                ayr_count,
                "ayr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_done_leaf, ayr_false, ayr_leaf_chk);
        self.builder.position_at_end(ayr_leaf_chk);
        let ayr_eb = unsafe {
            self.builder
                .build_gep(i8, ayr_leaf_i8, &[i64.const_int(8, false)], "ayr_eb")
                .map_err(llvm_err)?
        };
        let ayr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    ayr_eb,
                    &[ayr_i.as_basic_value().into_int_value()],
                    "ayr_ep",
                )
                .map_err(llvm_err)?
        };
        let ayr_elem = self
            .builder
            .build_load(self.string_type, ayr_ep, "ayr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let ayr_elem_tag = self
            .builder
            .build_extract_value(ayr_elem, 0, "ayr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_pred = self
            .builder
            .build_indirect_call(lambda_fn_ty, ayr_fn, &[ayr_elem_tag.into()], "ayr_pred")
            .map_err(llvm_err)?;
        let ayr_pred_bv = ayr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("any_walk indirect call failed")?;
        let ayr_pred_val = if ayr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(ayr_pred_bv.into_struct_value(), 0, "ayr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            ayr_pred_bv.into_int_value()
        };
        let ayr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, ayr_pred_val, zero, "ayr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_is_true, ayr_true, ayr_leaf_next);
        self.builder.position_at_end(ayr_leaf_next);
        let ayr_next_i = self
            .builder
            .build_int_add(
                ayr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ayr_ni",
            )
            .map_err(llvm_err)?;
        let ayr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        ayr_i.add_incoming(&[(&zero, ayr_leaf_hdr), (&ayr_next_i, ayr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(ayr_leaf_bdy);
        self.builder.position_at_end(ayr_int_hdr);
        let ayr_int_i8 = self
            .builder
            .build_pointer_cast(ayr_node, ptr, "ayr_int_i8")
            .map_err(llvm_err)?;
        let ayr_child_count_raw = self
            .builder
            .build_load(i32, ayr_int_i8, "ayr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_child_count = self
            .builder
            .build_int_z_extend(ayr_child_count_raw, i64, "ayr_cc")
            .map_err(llvm_err)?;
        let ayr_child_h = self
            .builder
            .build_int_sub(ayr_height, i64.const_int(1, false), "ayr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ayr_int_bdy);
        self.builder.position_at_end(ayr_int_bdy);
        let ayr_ci = self.builder.build_phi(i64, "ayr_ci").map_err(llvm_err)?;
        let ayr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                ayr_ci.as_basic_value().into_int_value(),
                ayr_child_count,
                "ayr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_done_int, ayr_false, ayr_int_child);
        self.builder.position_at_end(ayr_int_child);
        let ayr_children_base = unsafe {
            self.builder
                .build_gep(i8, ayr_int_i8, &[i64.const_int(16, false)], "ayr_cb")
                .map_err(llvm_err)?
        };
        let ayr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    ayr_children_base,
                    &[ayr_ci.as_basic_value().into_int_value()],
                    "ayr_cep",
                )
                .map_err(llvm_err)?
        };
        let ayr_child_entry = self
            .builder
            .build_load(self.child_entry_type, ayr_child_ep, "ayr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let ayr_child_ptr = self
            .builder
            .build_extract_value(ayr_child_entry, 0, "ayr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ayr_child_hit = self
            .builder
            .build_call(
                ay_rec_fn,
                &[ayr_child_ptr.into(), ayr_child_h.into(), ayr_fn.into()],
                "ayr_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(ayr_child_hit, ayr_true, ayr_int_next);
        self.builder.position_at_end(ayr_int_next);
        let ayr_next_ci = self
            .builder
            .build_int_add(
                ayr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ayr_nci",
            )
            .map_err(llvm_err)?;
        let ayr_int_next_bb = self.builder.get_insert_block().unwrap();
        ayr_ci.add_incoming(&[(&zero, ayr_int_hdr), (&ayr_next_ci, ayr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(ayr_int_bdy);
        self.builder.position_at_end(ayr_true);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(ayr_false);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        let ay_fn = self.module.add_function(
            "action_list_any_walk",
            b1.fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let ay_entry = self.context.append_basic_block(ay_fn, "entry");
        let ay_walk = self.context.append_basic_block(ay_fn, "walk");
        self.builder.position_at_end(ay_entry);
        let ay_list = ay_fn.get_first_param().unwrap().into_struct_value();
        let ay_fn_ptr = ay_fn.get_nth_param(1).unwrap().into_pointer_value();
        let ay_node = self
            .builder
            .build_extract_value(ay_list, 0, "ay_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ay_height = self
            .builder
            .build_extract_value(ay_list, 2, "ay_height")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(ay_walk);
        self.builder.position_at_end(ay_walk);
        let ay_hit = self
            .builder
            .build_call(
                ay_rec_fn,
                &[ay_node.into(), ay_height.into(), ay_fn_ptr.into()],
                "ay_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&ay_hit));

        // ---- action_list_all_walk_rec / action_list_all_walk ----
        let al_rec_fn = self.module.add_function(
            "action_list_all_walk_rec",
            b1.fn_type(&[ptr.into(), i64.into(), ptr.into()], false),
            None,
        );
        let alr_entry = self.context.append_basic_block(al_rec_fn, "entry");
        let alr_true = self.context.append_basic_block(al_rec_fn, "all_true");
        let alr_false = self.context.append_basic_block(al_rec_fn, "all_false");
        let alr_leaf_hdr = self.context.append_basic_block(al_rec_fn, "leaf_hdr");
        let alr_leaf_bdy = self.context.append_basic_block(al_rec_fn, "leaf_bdy");
        let alr_leaf_chk = self.context.append_basic_block(al_rec_fn, "leaf_chk");
        let alr_leaf_next = self.context.append_basic_block(al_rec_fn, "leaf_next");
        let alr_int_hdr = self.context.append_basic_block(al_rec_fn, "int_hdr");
        let alr_int_bdy = self.context.append_basic_block(al_rec_fn, "int_bdy");
        let alr_int_child = self.context.append_basic_block(al_rec_fn, "int_child");
        let alr_int_next = self.context.append_basic_block(al_rec_fn, "int_next");
        let alr_concat = self.context.append_basic_block(al_rec_fn, "concat");
        let alr_concat_right = self.context.append_basic_block(al_rec_fn, "concat_right");
        let alr_normal = self.context.append_basic_block(al_rec_fn, "normal");
        self.builder.position_at_end(alr_entry);
        let alr_node = al_rec_fn.get_first_param().unwrap().into_pointer_value();
        let alr_height = al_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let alr_fn = al_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let alr_neg1 = i64.const_int(-1i64 as u64, true);
        let alr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, alr_height, alr_neg1, "alr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_is_concat, alr_concat, alr_normal);
        self.builder.position_at_end(alr_concat);
        let alr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, alr_node, &[i64.const_int(2, false)], "alr_ln_p")
                .map_err(llvm_err)
        }?;
        let alr_left_node = self
            .builder
            .build_load(ptr, alr_ln_p, "alr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alr_lh_p = unsafe {
            self.builder
                .build_gep(i64, alr_node, &[i64.const_int(4, false)], "alr_lh_p")
                .map_err(llvm_err)
        }?;
        let alr_left_h = self
            .builder
            .build_load(i64, alr_lh_p, "alr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, alr_node, &[i64.const_int(5, false)], "alr_rn_p")
                .map_err(llvm_err)
        }?;
        let alr_right_node = self
            .builder
            .build_load(ptr, alr_rn_p, "alr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alr_rh_p = unsafe {
            self.builder
                .build_gep(i64, alr_node, &[i64.const_int(7, false)], "alr_rh_p")
                .map_err(llvm_err)
        }?;
        let alr_right_h = self
            .builder
            .build_load(i64, alr_rh_p, "alr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_lok = self
            .builder
            .build_call(
                al_rec_fn,
                &[alr_left_node.into(), alr_left_h.into(), alr_fn.into()],
                "alr_lok",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(alr_lok, alr_concat_right, alr_false);
        self.builder.position_at_end(alr_concat_right);
        let alr_rok = self
            .builder
            .build_call(
                al_rec_fn,
                &[alr_right_node.into(), alr_right_h.into(), alr_fn.into()],
                "alr_rok",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&alr_rok));
        self.builder.position_at_end(alr_normal);
        let alr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, alr_height, zero, "alr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_is_leaf, alr_leaf_hdr, alr_int_hdr);
        self.builder.position_at_end(alr_leaf_hdr);
        let alr_leaf_i8 = self
            .builder
            .build_pointer_cast(alr_node, ptr, "alr_leaf_i8")
            .map_err(llvm_err)?;
        let alr_count_raw = self
            .builder
            .build_load(i32, alr_leaf_i8, "alr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_count = self
            .builder
            .build_int_z_extend(alr_count_raw, i64, "alr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(alr_leaf_bdy);
        self.builder.position_at_end(alr_leaf_bdy);
        let alr_i = self.builder.build_phi(i64, "alr_i").map_err(llvm_err)?;
        let alr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                alr_i.as_basic_value().into_int_value(),
                alr_count,
                "alr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_done_leaf, alr_true, alr_leaf_chk);
        self.builder.position_at_end(alr_leaf_chk);
        let alr_eb = unsafe {
            self.builder
                .build_gep(i8, alr_leaf_i8, &[i64.const_int(8, false)], "alr_eb")
                .map_err(llvm_err)?
        };
        let alr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    alr_eb,
                    &[alr_i.as_basic_value().into_int_value()],
                    "alr_ep",
                )
                .map_err(llvm_err)?
        };
        let alr_elem = self
            .builder
            .build_load(self.string_type, alr_ep, "alr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let alr_elem_tag = self
            .builder
            .build_extract_value(alr_elem, 0, "alr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_pred = self
            .builder
            .build_indirect_call(lambda_fn_ty, alr_fn, &[alr_elem_tag.into()], "alr_pred")
            .map_err(llvm_err)?;
        let alr_pred_bv = alr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("all_walk indirect call failed")?;
        let alr_pred_val = if alr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(alr_pred_bv.into_struct_value(), 0, "alr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            alr_pred_bv.into_int_value()
        };
        let alr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, alr_pred_val, zero, "alr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_is_true, alr_leaf_next, alr_false);
        self.builder.position_at_end(alr_leaf_next);
        let alr_next_i = self
            .builder
            .build_int_add(
                alr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "alr_ni",
            )
            .map_err(llvm_err)?;
        let alr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        alr_i.add_incoming(&[(&zero, alr_leaf_hdr), (&alr_next_i, alr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(alr_leaf_bdy);
        self.builder.position_at_end(alr_int_hdr);
        let alr_int_i8 = self
            .builder
            .build_pointer_cast(alr_node, ptr, "alr_int_i8")
            .map_err(llvm_err)?;
        let alr_child_count_raw = self
            .builder
            .build_load(i32, alr_int_i8, "alr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_child_count = self
            .builder
            .build_int_z_extend(alr_child_count_raw, i64, "alr_cc")
            .map_err(llvm_err)?;
        let alr_child_h = self
            .builder
            .build_int_sub(alr_height, i64.const_int(1, false), "alr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(alr_int_bdy);
        self.builder.position_at_end(alr_int_bdy);
        let alr_ci = self.builder.build_phi(i64, "alr_ci").map_err(llvm_err)?;
        let alr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                alr_ci.as_basic_value().into_int_value(),
                alr_child_count,
                "alr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_done_int, alr_true, alr_int_child);
        self.builder.position_at_end(alr_int_child);
        let alr_children_base = unsafe {
            self.builder
                .build_gep(i8, alr_int_i8, &[i64.const_int(16, false)], "alr_cb")
                .map_err(llvm_err)?
        };
        let alr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    alr_children_base,
                    &[alr_ci.as_basic_value().into_int_value()],
                    "alr_cep",
                )
                .map_err(llvm_err)?
        };
        let alr_child_entry = self
            .builder
            .build_load(self.child_entry_type, alr_child_ep, "alr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let alr_child_ptr = self
            .builder
            .build_extract_value(alr_child_entry, 0, "alr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alr_child_ok = self
            .builder
            .build_call(
                al_rec_fn,
                &[alr_child_ptr.into(), alr_child_h.into(), alr_fn.into()],
                "alr_ok",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(alr_child_ok, alr_int_next, alr_false);
        self.builder.position_at_end(alr_int_next);
        let alr_next_ci = self
            .builder
            .build_int_add(
                alr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "alr_nci",
            )
            .map_err(llvm_err)?;
        let alr_int_next_bb = self.builder.get_insert_block().unwrap();
        alr_ci.add_incoming(&[(&zero, alr_int_hdr), (&alr_next_ci, alr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(alr_int_bdy);
        self.builder.position_at_end(alr_true);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(alr_false);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        let al_fn = self.module.add_function(
            "action_list_all_walk",
            b1.fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let al_entry = self.context.append_basic_block(al_fn, "entry");
        let al_walk = self.context.append_basic_block(al_fn, "walk");
        self.builder.position_at_end(al_entry);
        let al_list = al_fn.get_first_param().unwrap().into_struct_value();
        let al_fn_ptr = al_fn.get_nth_param(1).unwrap().into_pointer_value();
        let al_node = self
            .builder
            .build_extract_value(al_list, 0, "al_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let al_height = self
            .builder
            .build_extract_value(al_list, 2, "al_height")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(al_walk);
        self.builder.position_at_end(al_walk);
        let al_ok = self
            .builder
            .build_call(
                al_rec_fn,
                &[al_node.into(), al_height.into(), al_fn_ptr.into()],
                "al_ok",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&al_ok));

        // ---- action_list_contains({ptr, i64, i64}, {i64, ptr}) -> i1 ----
        let lc_fn = self.module.add_function(
            "action_list_contains",
            b1.fn_type(&[self.list_type.into(), self.string_type.into()], false),
            None,
        );
        let lc_entry = self.context.append_basic_block(lc_fn, "entry");
        let lc_concat = self.context.append_basic_block(lc_fn, "concat");
        let lc_walk = self.context.append_basic_block(lc_fn, "walk");
        self.builder.position_at_end(lc_entry);
        let lc_list = lc_fn.get_first_param().unwrap().into_struct_value();
        let lc_key = lc_fn.get_nth_param(1).unwrap().into_struct_value();
        let lc_node = self
            .builder
            .build_extract_value(lc_list, 0, "lc_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lc_height = self
            .builder
            .build_extract_value(lc_list, 2, "lc_height")
            .map_err(llvm_err)?
            .into_int_value();
        let lc_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lc_height,
                i64.const_int(-1i64 as u64, true),
                "lc_is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lc_is_concat, lc_concat, lc_walk);
        self.builder.position_at_end(lc_concat);
        let lc_flatten_fn = self.module.get_function("action_list_flatten").unwrap();
        let lc_flat = self
            .builder
            .build_call(lc_flatten_fn, &[lc_list.into()], "lc_flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lc_flat_node = self
            .builder
            .build_extract_value(lc_flat, 0, "lc_flat_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lc_flat_h = self
            .builder
            .build_extract_value(lc_flat, 2, "lc_flat_h")
            .map_err(llvm_err)?
            .into_int_value();
        let lc_flat_hit = self
            .builder
            .build_call(
                lc_walk_fn,
                &[lc_flat_node.into(), lc_flat_h.into(), lc_key.into()],
                "lc_flat_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&lc_flat_hit));
        self.builder.position_at_end(lc_walk);
        let lc_hit = self
            .builder
            .build_call(
                lc_walk_fn,
                &[lc_node.into(), lc_height.into(), lc_key.into()],
                "lc_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&lc_hit));

        Ok(())
    }
}
