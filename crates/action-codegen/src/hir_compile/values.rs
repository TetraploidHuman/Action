use crate::{llvm_err, CodeGen, TypedValue};
use action_frontend::ast::*;
use inkwell::types::BasicTypeEnum;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_binary_values(
        &mut self,
        op: BinaryOp,
        left: &TypedValue<'ctx>,
        right: &TypedValue<'ctx>,
        _result_ty: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        match op {
            BinaryOp::Add => self.bin_add(left, right),
            BinaryOp::Sub => self.bin_arith(
                left,
                right,
                "sub",
                |b, l, r| b.build_int_sub(l, r, "sub"),
                |b, l, r| b.build_float_sub(l, r, "sub"),
            ),
            BinaryOp::Mul => self.bin_arith(
                left,
                right,
                "mul",
                |b, l, r| b.build_int_mul(l, r, "mul"),
                |b, l, r| b.build_float_mul(l, r, "mul"),
            ),
            BinaryOp::Div => self.bin_div(left, right),
            BinaryOp::Mod => self.bin_mod(left, right),
            BinaryOp::Pow => self.bin_pow(left, right),
            BinaryOp::Eq => self.compare_eq(left, right),
            BinaryOp::Neq => self.compare_neq(left, right),
            BinaryOp::Lt => self.compare(
                inkwell::IntPredicate::SLT,
                inkwell::FloatPredicate::OLT,
                left,
                right,
            ),
            BinaryOp::Gt => self.compare(
                inkwell::IntPredicate::SGT,
                inkwell::FloatPredicate::OGT,
                left,
                right,
            ),
            BinaryOp::Lte => self.compare(
                inkwell::IntPredicate::SLE,
                inkwell::FloatPredicate::OLE,
                left,
                right,
            ),
            BinaryOp::Gte => self.compare(
                inkwell::IntPredicate::SGE,
                inkwell::FloatPredicate::OGE,
                left,
                right,
            ),
            BinaryOp::BitAnd => {
                self.bin_bitwise(left, right, "and", |b, l, r| b.build_and(l, r, "and"))
            }
            BinaryOp::BitOr => {
                self.bin_bitwise(left, right, "or", |b, l, r| b.build_or(l, r, "or"))
            }
            BinaryOp::BitXor => {
                self.bin_bitwise(left, right, "xor", |b, l, r| b.build_xor(l, r, "xor"))
            }
            BinaryOp::Shl => self.bin_bitwise(left, right, "shl", |b, l, r| {
                b.build_left_shift(l, r, "shl")
            }),
            BinaryOp::Shr => self.bin_bitwise(left, right, "shr", |b, l, r| {
                b.build_right_shift(l, r, false, "shr")
            }),
            BinaryOp::Range | BinaryOp::RangeExclusive => {
                let inclusive = matches!(op, BinaryOp::Range);
                let start_int = match left {
                    TypedValue::Int(v) => *v,
                    _ => return Err("Range start must be integer".into()),
                };
                let end_int = match right {
                    TypedValue::Int(v) => *v,
                    _ => return Err("Range end must be integer".into()),
                };
                let range_ty = self.context.struct_type(
                    &[
                        self.i64_ty().into(),
                        self.i64_ty().into(),
                        self.i64_ty().into(),
                    ],
                    false,
                );
                let alloca = self
                    .builder
                    .build_alloca(range_ty, "range")
                    .map_err(llvm_err)?;
                let sptr = self
                    .builder
                    .build_struct_gep(range_ty, alloca, 0, "r_start")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(sptr, start_int)
                    .map_err(llvm_err)?;
                let eptr = self
                    .builder
                    .build_struct_gep(range_ty, alloca, 1, "r_end")
                    .map_err(llvm_err)?;
                self.builder.build_store(eptr, end_int).map_err(llvm_err)?;
                let iptr = self
                    .builder
                    .build_struct_gep(range_ty, alloca, 2, "r_inc")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(
                        iptr,
                        self.i64_ty()
                            .const_int(if inclusive { 1 } else { 0 }, false),
                    )
                    .map_err(llvm_err)?;
                Ok(TypedValue::Struct(alloca, range_ty))
            }
            BinaryOp::And | BinaryOp::Or | BinaryOp::Is | BinaryOp::In => {
                unreachable!("handled before compile_binary_values")
            }
            BinaryOp::Assign => Err("assign is not a binary operator expression".to_string()),
        }
    }

    pub(crate) fn compile_copy_value(
        &mut self,
        val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match &val {
            TypedValue::Int(_)
            | TypedValue::Float(_)
            | TypedValue::Bool(_)
            | TypedValue::Unit
            | TypedValue::Fn(_, _)
            | TypedValue::Closure { .. }
            | TypedValue::CString(_)
            | TypedValue::Ptr(_)
            | TypedValue::FileHandle(_) => Ok(val),
            TypedValue::Str(ptr) => {
                let loaded = self.load_string(*ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.string_type, "str_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Str(new_alloca))
            }
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "struct_copy_ld")
                    .map_err(llvm_err)?;
                let new_alloca = self
                    .builder
                    .build_alloca(bt, "struct_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Struct(new_alloca, *st))
            }
            TypedValue::Enum(ptr, et, inner_type, rc_managed) => {
                let bt: BasicTypeEnum = (*et).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "enum_copy_ld")
                    .map_err(llvm_err)?;
                let new_alloca = self
                    .builder
                    .build_alloca(bt, "enum_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Enum(new_alloca, *et, *inner_type, *rc_managed))
            }
            TypedValue::List(ptr) => {
                let loaded = self.load_list(*ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "list_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::List(new_alloca))
            }
            TypedValue::Map(ptr) => {
                let loaded = self.load_list(*ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "map_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Map(new_alloca))
            }
            TypedValue::Set(ptr) => {
                let loaded = self.load_list(*ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "set_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Set(new_alloca))
            }
            _ => Err("copy not supported for this type".to_string()),
        }
    }

    // ---- HIR-native helper methods ----

    /// Emit scope cleanup and return the given value.
    pub(crate) fn compile_return_value(&mut self, val: TypedValue<'ctx>) -> Result<(), String> {
        if self.is_scope_variable(&val) {
            self.rc_inc_typed_value(&val)?;
        }
        self.emit_scope_cleanup()?;
        if let Some(bv) = val.to_bv() {
            let _ = self.builder.build_return(Some(&bv));
            return Ok(());
        }
        match &val {
            TypedValue::Str(ptr) => {
                let sv = self.load_string(*ptr)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::Enum(ptr, ty, ..) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "ret_enum")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&loaded));
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "ret_struct")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&loaded));
            }
            TypedValue::Stream(ptr) => {
                let list_field = self
                    .builder
                    .build_struct_gep(self.stream_type, *ptr, 1, "ret_sl2")
                    .map_err(llvm_err)?;
                let sv = self
                    .builder
                    .build_load(self.list_type, list_field, "ret_sv2")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::Task(ptr) => {
                let sv = self
                    .builder
                    .build_load(self.task_type, *ptr, "ret_task")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                let sv = self.load_list(*ptr)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::LazyList(ptr) => {
                let ll_val = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "ret_ll")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&ll_val));
            }
            TypedValue::Nullable(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "ret_nullable")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&loaded));
            }
            _ => {
                let _ = self.builder.build_return(None);
            }
        }
        Ok(())
    }

    /// Emit scope cleanup and return void.
    pub(crate) fn compile_return_void(&mut self) -> Result<(), String> {
        self.emit_scope_cleanup()?;
        let _ = self.builder.build_return(None);
        Ok(())
    }

    /// Compile a unary operation on an already-compiled value.
    pub(crate) fn compile_unary_values(
        &mut self,
        op: UnaryOp,
        val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match op {
            UnaryOp::Neg => match val {
                TypedValue::Int(v) => Ok(TypedValue::Int(
                    self.builder.build_int_neg(v, "neg").map_err(llvm_err)?,
                )),
                TypedValue::Float(v) => Ok(TypedValue::Float(
                    self.builder.build_float_neg(v, "neg").map_err(llvm_err)?,
                )),
                _ => Err("Cannot negate this type".to_string()),
            },
            UnaryOp::Not => match val {
                TypedValue::Bool(v) => Ok(TypedValue::Bool(
                    self.builder.build_not(v, "not").map_err(llvm_err)?,
                )),
                _ => Err("'not' requires boolean operand".to_string()),
            },
            UnaryOp::BitNot => match val {
                TypedValue::Int(v) => Ok(TypedValue::Int(
                    self.builder.build_not(v, "bitnot").map_err(llvm_err)?,
                )),
                _ => Err("'~' requires integer operand".to_string()),
            },
        }
    }

    /// Compile field access on an already-compiled value.
    pub(crate) fn compile_field_access_value(
        &mut self,
        obj_val: TypedValue<'ctx>,
        field: &str,
    ) -> Result<TypedValue<'ctx>, String> {
        // Struct field access: load the struct and extract by field name
        if let TypedValue::Struct(ptr, struct_ty) = &obj_val {
            let bt: BasicTypeEnum = (*struct_ty).into();
            let loaded = self
                .builder
                .build_load(bt, *ptr, "struct_ld")
                .map_err(llvm_err)?
                .into_struct_value();

            // Try numeric index for tuple access: .0, .1, etc.
            if let Ok(idx) = field.parse::<usize>() {
                let field_val = self
                    .builder
                    .build_extract_value(loaded, idx as u32, field)
                    .map_err(llvm_err)?;
                return self.bv_to_typed(field_val);
            }

            let field_names = self.lookup_struct_field_names(*struct_ty);
            let idx = field_names
                .iter()
                .position(|n| n == field)
                .ok_or_else(|| format!("Field '{}' not found on struct", field))?;
            let field_val = self
                .builder
                .build_extract_value(loaded, idx as u32, field)
                .map_err(llvm_err)?;
            return self.bv_to_typed(field_val);
        }

        // Delegate to compile_field_access_on_typed_value for other types
        let val_bt = obj_val.get_type_for_alloca(self);
        self.compile_field_access_on_typed_value(&obj_val, field, val_bt)
    }

    /// Compile range creation from already-compiled start/end values.
    pub(crate) fn compile_range_values(
        &mut self,
        start_val: TypedValue<'ctx>,
        end_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let start_int = match start_val {
            TypedValue::Int(v) => v,
            _ => return Err("Range start must be integer".to_string()),
        };
        let end_int = match end_val {
            TypedValue::Int(v) => v,
            _ => return Err("Range end must be integer".to_string()),
        };
        let range_ty = self.range_type;
        let alloca = self
            .builder
            .build_alloca(range_ty, "range")
            .map_err(llvm_err)?;
        let sptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 0, "r_start")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sptr, start_int)
            .map_err(llvm_err)?;
        let eptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 1, "r_end")
            .map_err(llvm_err)?;
        self.builder.build_store(eptr, end_int).map_err(llvm_err)?;
        let iptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 2, "r_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(iptr, self.i64_ty().const_int(1, false))
            .map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, range_ty))
    }
}
