// Submodule: builtins_conversion

use crate::ast::*;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// toList(lazy_or_set) - convert a LazyList or Set to a List
    pub(super) fn builtin_to_list(&mut self, expr: &Expr) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_expr(expr)?;
        match val {
            TypedValue::LazyList(_) => {
                let list_sv = self.convert_lazylist_to_list(&val)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "toList")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, list_sv)
                    .map_err(llvm_err)?;
                Ok(TypedValue::List(new_alloca))
            }
            TypedValue::Set(ptr) => {
                let list_val = self.load_list(ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "to_list_s")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, list_val)
                    .map_err(llvm_err)?;
                Ok(TypedValue::List(new_alloca))
            }
            TypedValue::List(_) => Ok(val),
            _ => Err("toList: argument must be a LazyList or Set".to_string()),
        }
    }

    /// toLazyList(list) - convert a List to a LazyList
    pub(super) fn builtin_to_lazy_list(&mut self, expr: &Expr) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_expr(expr)?;
        match val {
            TypedValue::List(ptr) => {
                // Load list, extract first element as head
                let list_sv = self.load_list(ptr)?;
                let data = self
                    .builder
                    .build_extract_value(list_sv, 0, "toll_data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let len = self
                    .builder
                    .build_extract_value(list_sv, 1, "toll_len")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Load first element (fat struct) from data[0]
                let first_fat_ptr = unsafe {
                    self.builder
                        .build_gep(
                            self.fat_return_type,
                            data,
                            &[self.i64_ty().const_int(0, false)],
                            "toll_gep",
                        )
                        .map_err(llvm_err)
                }?;
                let first_fat = self
                    .builder
                    .build_load(self.fat_return_type, first_fat_ptr, "toll_fat")
                    .map_err(llvm_err)?;
                let head_val = self
                    .builder
                    .build_extract_value(first_fat.into_struct_value(), 0, "toll_head")
                    .map_err(llvm_err)?
                    .into_int_value();

                // Store data pointer as i64 in state field so round-trip toList can recover all elements
                let data_as_i64 = self
                    .builder
                    .build_ptr_to_int(data, self.i64_ty(), "data_i64")
                    .map_err(llvm_err)?;

                // Create LazyList with head, no step fn, state = data_ptr, take_count = len
                let ll_alloca = self
                    .builder
                    .build_alloca(self.lazylist_type, "to_ll")
                    .map_err(llvm_err)?;
                let undef = self.lazylist_type.get_undef();
                let v0 = self
                    .builder
                    .build_insert_value(undef, head_val, 0, "ll_h")
                    .map_err(llvm_err)?;
                let v1 = self
                    .builder
                    .build_insert_value(v0, self.ptr_ty().const_null(), 1, "ll_fn")
                    .map_err(llvm_err)?;
                let v2 = self
                    .builder
                    .build_insert_value(v1, data_as_i64, 2, "ll_s")
                    .map_err(llvm_err)?;
                let v3 = self
                    .builder
                    .build_insert_value(v2, len, 3, "ll_tc")
                    .map_err(llvm_err)?;
                let v4 = self
                    .builder
                    .build_insert_value(v3, self.ptr_ty().const_null(), 4, "ll_map")
                    .map_err(llvm_err)?;
                let v5 = self
                    .builder
                    .build_insert_value(v4, self.ptr_ty().const_null(), 5, "ll_filt")
                    .map_err(llvm_err)?;
                self.builder.build_store(ll_alloca, v5).map_err(llvm_err)?;
                Ok(TypedValue::LazyList(ll_alloca))
            }
            TypedValue::LazyList(_) => Ok(val),
            _ => Err("toLazyList: argument must be a List".to_string()),
        }
    }
}
