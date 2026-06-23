use crate::{llvm_err, CodeGen};
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn extract_ptr(
        &self,
        st: inkwell::values::StructValue<'ctx>,
        idx: u32,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(st, idx, name)
            .map_err(llvm_err)?
            .into_pointer_value())
    }

    pub(crate) fn extract_int(
        &self,
        st: inkwell::values::StructValue<'ctx>,
        idx: u32,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(st, idx, name)
            .map_err(llvm_err)?
            .into_int_value())
    }

    pub(crate) fn load_i64(
        &self,
        a: PointerValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        Ok(self
            .builder
            .build_load(self.i64_ty(), a, name)
            .map_err(llvm_err)?
            .into_int_value())
    }

    /// Load key as fat struct from slot index; normalizes scalar marker kp 1 -> 0.
    pub(crate) fn ht_key_fat_at(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let marker = i64.const_int(Self::HT_SCALAR_MARKER, false);
        let (kt, kp, _, _, _) = self.ht_load_slot(data, slot)?;
        let is_mark = self
            .builder
            .build_int_compare(IntPredicate::EQ, kp, marker, "mk")
            .map_err(llvm_err)?;
        let norm_kp = self
            .builder
            .build_select(is_mark, zero, kp, "nkp")
            .map_err(llvm_err)?
            .into_int_value();
        let kp_p = self
            .builder
            .build_int_to_ptr(norm_kp, ptr, "kp")
            .map_err(llvm_err)?;
        let u = str_ty.get_undef();
        let k1 = self
            .builder
            .build_insert_value(u, kt, 0, "k1")
            .map_err(llvm_err)?;
        self.builder
            .build_insert_value(k1, kp_p, 1, "k2")
            .map_err(llvm_err)
            .map(|v| v.as_basic_value_enum())
    }

    /// Load value as fat struct from slot index.
    pub(crate) fn ht_val_fat_at(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let (_, _, vt, vp, _) = self.ht_load_slot(data, slot)?;
        let vp_p = self
            .builder
            .build_int_to_ptr(vp, ptr, "vp")
            .map_err(llvm_err)?;
        let u = str_ty.get_undef();
        let v1 = self
            .builder
            .build_insert_value(u, vt, 0, "v1")
            .map_err(llvm_err)?;
        self.builder
            .build_insert_value(v1, vp_p, 1, "v2")
            .map_err(llvm_err)
            .map(|v| v.as_basic_value_enum())
    }
}
