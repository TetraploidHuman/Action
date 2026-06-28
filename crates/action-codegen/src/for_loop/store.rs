//! For-loop codegen (R4-4).

use inkwell::types::BasicTypeEnum;
use inkwell::values::PointerValue;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn store_value_to_alloca(
        &mut self,
        v: &TypedValue<'ctx>,
        alloca: PointerValue<'ctx>,
    ) -> Result<(), String> {
        match v {
            TypedValue::Str(ptr) => {
                let str_val = self.load_string(*ptr)?;
                self.builder
                    .build_store(alloca, str_val)
                    .map_err(llvm_err)?;
            }
            TypedValue::List(ptr) => {
                let list_val = self.load_list(*ptr)?;
                self.builder
                    .build_store(alloca, list_val)
                    .map_err(llvm_err)?;
            }
            TypedValue::Map(ptr) => {
                let map_val = self.load_list(*ptr)?;
                self.builder
                    .build_store(alloca, map_val)
                    .map_err(llvm_err)?;
            }
            TypedValue::Set(ptr) => {
                let set_val = self.load_list(*ptr)?;
                self.builder
                    .build_store(alloca, set_val)
                    .map_err(llvm_err)?;
            }
            TypedValue::Task(ptr) => {
                self.builder.build_store(alloca, *ptr).map_err(llvm_err)?;
            }
            TypedValue::Stream(ptr) => {
                self.builder.build_store(alloca, *ptr).map_err(llvm_err)?;
            }
            TypedValue::LazyList(ptr) => {
                let ll_val = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "ll_ld")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, ll_val).map_err(llvm_err)?;
            }
            TypedValue::CString(p) | TypedValue::Ptr(p) | TypedValue::FileHandle(p) => {
                self.builder.build_store(alloca, *p).map_err(llvm_err)?;
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "struct_ld")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
            }
            TypedValue::Enum(ptr, ty, ..) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "enum_ld")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
            }
            TypedValue::FallibleInt { val, .. } => {
                self.builder.build_store(alloca, *val).map_err(llvm_err)?;
            }
            TypedValue::FallibleFloat { val, .. } => {
                self.builder.build_store(alloca, *val).map_err(llvm_err)?;
            }
            TypedValue::FalliblePtr { val, .. } => {
                self.builder.build_store(alloca, *val).map_err(llvm_err)?;
            }
            TypedValue::FallibleStr { val, .. } => {
                let loaded = self.load_string(*val)?;
                self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
            }
            TypedValue::FallibleStruct { val, ty, .. } => {
                let bt: inkwell::types::BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *val, "fall_ld")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
            }
            _ => {
                if let Some(bv) = v.to_bv() {
                    self.builder.build_store(alloca, bv).map_err(llvm_err)?;
                }
            }
        }
        Ok(())
    }

    /// Store a TypedValue to an alloca, coercing types when the alloca type differs.
    pub(crate) fn store_typed_value(
        &mut self,
        v: &TypedValue<'ctx>,
        alloca: PointerValue<'ctx>,
        target_ty: BasicTypeEnum<'ctx>,
    ) -> Result<(), String> {
        match (v, target_ty) {
            // Int -> Float coercion
            (TypedValue::Int(iv), BasicTypeEnum::FloatType(_)) => {
                let fv = self
                    .builder
                    .build_signed_int_to_float(*iv, self.f64_ty(), "int2float")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, fv).map_err(llvm_err)?;
            }
            // Float -> Int coercion
            (TypedValue::Float(fv), BasicTypeEnum::IntType(_)) => {
                let iv = self
                    .builder
                    .build_float_to_signed_int(*fv, self.i64_ty(), "float2int")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, iv).map_err(llvm_err)?;
            }
            _ => self.store_value_to_alloca(v, alloca)?,
        }
        Ok(())
    }
}
