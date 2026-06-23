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
