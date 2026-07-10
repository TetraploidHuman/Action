//! Expression codegen (R4-3).

use action_frontend::ast::*;
use action_frontend::types::collection_kind_from_type;
use inkwell::types::BasicTypeEnum;
use inkwell::values::BasicValueEnum;

use super::{llvm_err, CodeGen, InnerType, TypedValue, ValKind};
use crate::type_helpers::val_kind_for_collection;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn unpack_fat_return(
        &mut self,
        bv: BasicValueEnum<'ctx>,
        ret_ty: Option<BasicTypeEnum<'ctx>>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.unpack_call_return(bv, ret_ty, None)
    }

    /// Unpack a direct/indirect call result, using AST return type when the LLVM
    /// type is the shared `{ptr,i64,i64}` layout so List/Map/Set dispatch correctly.
    pub(crate) fn unpack_call_return(
        &mut self,
        bv: BasicValueEnum<'ctx>,
        llvm_ret: Option<BasicTypeEnum<'ctx>>,
        ast_ret: Option<&Type>,
    ) -> Result<TypedValue<'ctx>, String> {
        if let Some(rt) = llvm_ret {
            if let BasicTypeEnum::StructType(fat_ty) = rt {
                if fat_ty == self.fat_return_type {
                    if let BasicValueEnum::StructValue(sv) = bv {
                        let alloca = self
                            .builder
                            .build_alloca(fat_ty, "fat_unpack")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, sv).map_err(llvm_err)?;
                        self.last_fat_ret = Some((alloca, fat_ty));
                        let gep0 = self
                            .builder
                            .build_struct_gep(fat_ty, alloca, 0, "val_gep")
                            .map_err(llvm_err)?;
                        let val = self
                            .builder
                            .build_load(self.i64_ty(), gep0, "val")
                            .map_err(llvm_err)?
                            .into_int_value();
                        return Ok(TypedValue::Int(val));
                    }
                }
            }
        }
        if let BasicValueEnum::StructValue(sv) = bv {
            if sv.get_type() == self.list_type {
                if let Some(kind) = ast_ret.and_then(Self::heap_collection_kind) {
                    let alloca = self
                        .builder
                        .build_alloca(self.list_type, "call_heap_ret")
                        .map_err(llvm_err)?;
                    self.builder.build_store(alloca, sv).map_err(llvm_err)?;
                    return Ok(match kind {
                        ValKind::Map => TypedValue::Map(alloca),
                        ValKind::Set => TypedValue::Set(alloca),
                        _ => TypedValue::List(alloca),
                    });
                }
            }
        }
        self.bv_to_typed(bv)
    }

    pub(crate) fn heap_collection_kind(t: &Type) -> Option<ValKind> {
        collection_kind_from_type(t).map(val_kind_for_collection)
    }

    pub(crate) fn bv_to_typed(
        &mut self,
        val: BasicValueEnum<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match val {
            BasicValueEnum::IntValue(v) if v.get_type().get_bit_width() == 1 => {
                Ok(TypedValue::Bool(v))
            }
            BasicValueEnum::IntValue(v) => Ok(TypedValue::Int(v)),
            BasicValueEnum::FloatValue(v) => Ok(TypedValue::Float(v)),
            BasicValueEnum::PointerValue(v) => Ok(TypedValue::Ptr(v)),
            BasicValueEnum::StructValue(v) => {
                let st = v.get_type();
                let alloca = self
                    .builder
                    .build_alloca(st, "struct_tmp2")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, v).map_err(llvm_err)?;
                if st == self.fat_return_type {
                    // Fat return from untyped lambda/function: extract field 0 as Int.
                    // Also save the full alloca for possible enum bitcast later.
                    self.last_fat_ret = Some((alloca, st));
                    let gep0 = self
                        .builder
                        .build_struct_gep(st, alloca, 0, "fv_gep")
                        .map_err(llvm_err)?;
                    let val = self
                        .builder
                        .build_load(self.i64_ty(), gep0, "fv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    Ok(TypedValue::Int(val))
                } else if st == self.string_type {
                    // Named __action_str type — must check before enum_types since
                    // enum types are anonymous {i64, ptr} which used to collide.
                    Ok(TypedValue::Str(alloca))
                } else if st == self.list_type {
                    // List, Map, Set all share list_type. Default to List.
                    Ok(TypedValue::List(alloca))
                } else if st == self.lazylist_type {
                    Ok(TypedValue::LazyList(alloca))
                } else if self.type_layout.enum_types.values().any(|et| *et == st) {
                    // Matches a registered enum type (anonymous {i64,ptr})
                    let (inner_type, rc_managed) = self
                        .last_enum_inner
                        .take()
                        .unwrap_or((InnerType::Int, false));
                    Ok(TypedValue::Enum(alloca, st, inner_type, rc_managed))
                } else {
                    Ok(TypedValue::Struct(alloca, st))
                }
            }
            _ => Ok(TypedValue::Unit),
        }
    }

    /// Infer ValKind from a BasicValueEnum (used for destructuring, where types are not annotated)
    pub(crate) fn bv_kind(&self, val: &BasicValueEnum<'ctx>) -> ValKind {
        match val {
            BasicValueEnum::IntValue(v) if v.get_type().get_bit_width() == 1 => ValKind::Bool,
            BasicValueEnum::IntValue(_) => ValKind::Int,
            BasicValueEnum::FloatValue(_) => ValKind::Float,
            BasicValueEnum::StructValue(v) => {
                let st = v.get_type();
                if st == self.string_type {
                    ValKind::Str
                } else if st == self.list_type {
                    ValKind::List
                } else if self.type_layout.enum_types.values().any(|et| *et == st) {
                    ValKind::Enum
                } else {
                    ValKind::Struct
                }
            }
            _ => ValKind::Int,
        }
    }
}
