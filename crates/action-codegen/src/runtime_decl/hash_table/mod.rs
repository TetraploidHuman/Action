// Submodule: runtime_decl/hash_table (R5-1)
//
// Open-addressing hash table for Map/Set with Robin-Hood probing.
// 40-byte entries: key_tag, key_ptr, val_tag, val_ptr, dist (probe distance from ideal slot).
// Struct { ptr data, i64 len, i64 cap } — reuses list_type; len = occupied count, cap = slot count.

mod accessors;
mod from_list;
mod hash_rehash;
mod helpers;
mod insert;
mod query;
mod rc_dec;
mod remove;

use crate::{llvm_err, CodeGen};

impl<'ctx> CodeGen<'ctx> {
    const HT_ENTRY_I64S: u64 = 5;
    const HT_ENTRY_BYTES: u64 = 40;
    pub(crate) const HT_SCALAR_MARKER: u64 = 1;
    pub(crate) const HT_TOMBSTONE: u64 = 2;
    const HT_MIN_CAP: u64 = 8;
    const HT_LOAD_NUM: u64 = 7;
    const HT_LOAD_DEN: u64 = 8;
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;
    const GOLDEN: u64 = 0x9e3779b97f4a7c15;

    pub(super) fn define_hash_table(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let malloc_rc = self.module.get_function("action_malloc_rc").unwrap();
        let memset = self.module.get_function("memset").unwrap();
        let memcpy = self.module.get_function("memcpy").unwrap();
        let seq_fn = self.module.get_function("action_string_eq").unwrap();
        let rc_dec = self.module.get_function("action_rc_dec").unwrap();
        let free_fn = self.module.get_function("free").unwrap();
        let zero = i64.const_int(0, false);
        let i32z = self.context.i32_type().const_int(0, false);

        self.define_ht_hash_str()?;
        self.define_ht_rehash(seq_fn, malloc_rc, memset)?;
        self.define_ht_bulk_copy_active_slots(seq_fn)?;

        // action_ht_create(cap_hint) -> {ptr, i64, i64}
        let cr = self.module.add_function(
            "action_ht_create",
            self.list_type.fn_type(&[i64.into()], false),
            None,
        );
        let cr_e = self.context.append_basic_block(cr, "entry");
        self.builder.position_at_end(cr_e);
        let hint = cr.get_first_param().unwrap().into_int_value();
        let load_num = i64.const_int(Self::HT_LOAD_NUM, false);
        let load_den = i64.const_int(Self::HT_LOAD_DEN, false);
        let one = i64.const_int(1, false);
        // Scale element-count hint to slot capacity for 75% max load factor.
        let scaled = self
            .builder
            .build_int_add(
                self.builder
                    .build_int_unsigned_div(
                        self.builder
                            .build_int_mul(hint, load_den, "h4")
                            .map_err(llvm_err)?,
                        load_num,
                        "hs",
                    )
                    .map_err(llvm_err)?,
                one,
                "scaled",
            )
            .map_err(llvm_err)?;
        let cap = self.ht_round_cap_pow2(scaled)?;
        let dsz = self
            .builder
            .build_int_mul(cap, i64.const_int(Self::HT_ENTRY_BYTES, false), "dsz")
            .map_err(llvm_err)?;
        let data = self
            .builder
            .build_call(malloc_rc, &[dsz.into()], "d")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(memset, &[data.into(), i32z.into(), dsz.into()], "");
        let r = self.ht_pack(data, zero, cap)?;
        self.builder.build_return(Some(&r)).map_err(llvm_err)?;

        // action_ht_len
        let ln = self.module.add_function(
            "action_ht_len",
            i64.fn_type(&[self.list_type.into()], false),
            None,
        );
        let ln_e = self.context.append_basic_block(ln, "entry");
        self.builder.position_at_end(ln_e);
        let lnv = ln.get_first_param().unwrap().into_struct_value();
        self.builder
            .build_return(Some(
                &self
                    .builder
                    .build_extract_value(lnv, 1, "len")
                    .map_err(llvm_err)?,
            ))
            .map_err(llvm_err)?;

        self.define_ht_insert(seq_fn, memcpy)?;
        self.define_ht_get_contains(seq_fn)?;
        self.define_ht_remove(seq_fn, memcpy)?;
        self.define_ht_rc_dec(rc_dec, free_fn)?;
        self.define_ht_from_list()?;

        Ok(())
    }
}
