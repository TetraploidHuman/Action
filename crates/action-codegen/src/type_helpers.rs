// Submodule: type_helpers — type inference, type conversion helpers
//
// Extracted from misc.rs.
//

use action_frontend::ast::*;
use action_frontend::hir::{HirExpr, HirExprKind};
use action_frontend::types::{collection_kind_from_type, collection_kind_from_type_name, normalize_type_name, CollectionKind};
use inkwell::types::{BasicType, BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum};

use super::{llvm_err, CodeGen, TypedValue, ValKind};
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::types::FunctionType;

pub(crate) fn val_kind_for_collection(kind: CollectionKind) -> ValKind {
    match kind {
        CollectionKind::List => ValKind::List,
        CollectionKind::Map => ValKind::Map,
        CollectionKind::Set => ValKind::Set,
    }
}

fn codegen_unsupported_type(ty: &Type) -> String {
    format!("codegen: unsupported type '{ty}' (no LLVM layout mapping)")
}

impl<'ctx> CodeGen<'ctx> {
    /// Map a fully-resolved AST type to LLVM layout. Unknown types are errors, not Int.
    pub(super) fn llvm_layout_for_type(
        &mut self,
        ty: &Type,
    ) -> Result<BasicTypeEnum<'ctx>, String> {
        if collection_kind_from_type(ty).is_some() {
            return Ok(self.collection_layout_type());
        }
        match ty {
            Type::Named(n) => {
                match normalize_type_name(n) {
                    "Int" | "Char" | "Unit" => Ok(self.i64_ty().into()),
                    "Float" => Ok(self.f64_ty().into()),
                    "Bool" => Ok(self.bool_ty().into()),
                    "String" => Ok(self.string_type.into()),
                    "LazyList" => Ok(self.lazylist_type.into()),
                    "Task" => Ok(self.task_type.into()),
                    "Stream" => Ok(self.ptr_ty().into()),
                    "Ptr" | "CString" | "FileHandle" => Ok(self.ptr_ty().into()),
                    name => {
                        if let Some(st) = self.type_layout.named_structs.get(name) {
                            Ok((*st).into())
                        } else if let Some(et) = self.type_layout.enum_types.get(name) {
                            Ok((*et).into())
                        } else {
                            Err(codegen_unsupported_type(ty))
                        }
                    }
                }
            }
            Type::Struct(fields) => {
                let field_tys: Vec<BasicTypeEnum> = fields
                    .iter()
                    .map(|(_, fty)| self.llvm_layout_for_type(fty))
                    .collect::<Result<_, _>>()?;
                Ok(self.context.struct_type(&field_tys, false).into())
            }
            Type::Function(_, _) => Ok(self.ptr_ty().into()),
            Type::Map(_, _) | Type::Set(_) => Ok(self.collection_layout_type()),
            Type::Task(_) => Ok(self.task_type.into()),
            Type::Stream(_) => Ok(self.ptr_ty().into()),
            Type::LazyList(_) => Ok(self.lazylist_type.into()),
            Type::CString | Type::Ptr(_) | Type::FileHandle => Ok(self.ptr_ty().into()),
            Type::Unit => Ok(self.i64_ty().into()),
            Type::Generic(base, _) => match base.as_ref() {
                Type::Named(n) => match normalize_type_name(n) {
                    "Task" => Ok(self.task_type.into()),
                    "Stream" => Ok(self.ptr_ty().into()),
                    "LazyList" => Ok(self.lazylist_type.into()),
                    "Ptr" => Ok(self.ptr_ty().into()),
                    name if collection_kind_from_type_name(name).is_some() => {
                        Ok(self.collection_layout_type())
                    }
                    _ => Err(codegen_unsupported_type(ty)),
                },
                _ => Err(codegen_unsupported_type(ty)),
            },
            Type::TypeVar(name) => Err(format!(
                "codegen: unresolved type variable '{name}' (instantiate before codegen)"
            )),
            // Ephemeral HM metavariable; HIR may still carry ?N after typecheck.
            Type::InferVar(_) => Ok(self.i64_ty().into()),
        }
    }

    pub(super) fn ast_type_to_basic_type(
        &mut self,
        ty: &Type,
    ) -> Result<BasicTypeEnum<'ctx>, String> {
        self.llvm_layout_for_type(ty)
    }

    pub(super) fn ast_type_to_llvm(
        &mut self,
        ty: Option<&Type>,
    ) -> Result<inkwell::types::BasicMetadataTypeEnum<'ctx>, String> {
        match ty {
            None => Ok(self.i64_ty().into()),
            Some(t) => self.llvm_layout_for_type(t).map(|b| b.into()),
        }
    }

    fn val_kind_for_ast_type(&self, ty: &Type) -> Result<ValKind, String> {
        if let Some(kind) = collection_kind_from_type(ty) {
            return Ok(val_kind_for_collection(kind));
        }
        match ty {
            Type::Named(n) => match normalize_type_name(n) {
                "Int" | "Char" | "Unit" => Ok(ValKind::Int),
                "Float" => Ok(ValKind::Float),
                "Bool" => Ok(ValKind::Bool),
                "String" => Ok(ValKind::Str),
                "LazyList" => Ok(ValKind::LazyList),
                "Task" => Ok(ValKind::Task),
                "Stream" => Ok(ValKind::Stream),
                "Ptr" | "CString" | "FileHandle" => Ok(ValKind::Ptr),
                name => {
                    if self.type_layout.named_structs.contains_key(name) {
                        Ok(ValKind::Struct)
                    } else if self.type_layout.enum_types.contains_key(name) {
                        Ok(ValKind::Enum)
                    } else {
                        Err(codegen_unsupported_type(ty))
                    }
                }
            },
            Type::Function(_, _) => Ok(ValKind::Fn),
            Type::Map(_, _) => Ok(ValKind::Map),
            Type::Set(_) => Ok(ValKind::Set),
            Type::Task(_) => Ok(ValKind::Task),
            Type::Stream(_) => Ok(ValKind::Stream),
            Type::LazyList(_) => Ok(ValKind::LazyList),
            Type::Ptr(_) | Type::CString | Type::FileHandle => Ok(ValKind::Ptr),
            Type::Struct(_) => Ok(ValKind::Struct),
            Type::Unit => Ok(ValKind::Int),
            Type::Generic(base, _) => match base.as_ref() {
                Type::Named(n) => match normalize_type_name(n) {
                    "Float" => Ok(ValKind::Float),
                    "Bool" => Ok(ValKind::Bool),
                    "String" => Ok(ValKind::Str),
                    "Task" => Ok(ValKind::Task),
                    "Stream" => Ok(ValKind::Stream),
                    "LazyList" => Ok(ValKind::LazyList),
                    "Ptr" => Ok(ValKind::Ptr),
                    name if collection_kind_from_type_name(name).is_some() => {
                        Ok(val_kind_for_collection(
                            collection_kind_from_type_name(name).unwrap(),
                        ))
                    }
                    _ => Err(codegen_unsupported_type(ty)),
                },
                _ => Err(codegen_unsupported_type(ty)),
            },
            Type::TypeVar(name) => Err(format!(
                "codegen: unresolved type variable '{name}' (instantiate before codegen)"
            )),
            Type::InferVar(_) => Ok(ValKind::Int),
        }
    }

    pub(super) fn param_val_kind(&self, ty: Option<&Type>) -> Result<ValKind, String> {
        match ty {
            None => Ok(ValKind::Int),
            Some(t) => self.val_kind_for_ast_type(t),
        }
    }
    /// LLVM struct type for List / Map / Set (shared `{ptr, len, height}` layout).
    pub(super) fn collection_layout_type(&self) -> BasicTypeEnum<'ctx> {
        self.list_type.into()
    }

    pub(super) fn to_fat_struct(
        &mut self,
        val: &TypedValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match val {
            TypedValue::Str(ptr) => Ok(self.load_string(*ptr)?.into()),
            TypedValue::Enum(ptr, _ty, ..) => {
                let bt: BasicTypeEnum = self.string_type.into();
                Ok(self
                    .builder
                    .build_load(bt, *ptr, "fat_enum")
                    .map_err(llvm_err)?)
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                Ok(self
                    .builder
                    .build_load(bt, *ptr, "fat_struct")
                    .map_err(llvm_err)?)
            }
            TypedValue::List(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(6, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            TypedValue::Map(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(7, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            TypedValue::Set(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(8, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            TypedValue::Task(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(9, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            TypedValue::Stream(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(10, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            _ => {
                // Scalar value: wrap in {scalar, null}
                let bv = val
                    .to_bv()
                    .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
                // Coerce Float/Bool to i64 (field 0 of string_type is i64)
                let coerced: BasicValueEnum = match bv {
                    BasicValueEnum::FloatValue(fv) => {
                        let tmp = self
                            .builder
                            .build_alloca(self.f64_ty(), "ftmp")
                            .map_err(llvm_err)?;
                        self.builder.build_store(tmp, fv).map_err(llvm_err)?;
                        let casted = self
                            .builder
                            .build_pointer_cast(tmp, self.ptr_ty(), "fcast")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_load(self.i64_ty(), casted, "fbits")
                            .map_err(llvm_err)?
                    }
                    BasicValueEnum::IntValue(iv) if iv.get_type().get_bit_width() == 1 => self
                        .builder
                        .build_int_z_extend(iv, self.i64_ty(), "b2i")
                        .map_err(llvm_err)?
                        .as_basic_value_enum(),
                    _ => bv,
                };
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, coerced, 0, "wrap0")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, self.ptr_ty().const_zero(), 1, "wrap1")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers extracted from mod.rs: type helpers, RC helpers, type inference, name mangling
// ---------------------------------------------------------------------------
impl<'ctx> CodeGen<'ctx> {
    pub fn set_opt_level(&mut self, level: u8) {
        self.opt_level = level.min(3);
    }

    /// Convert Int or Float TypedValue to FloatValue (Int gets converted via sitofp).
    pub(super) fn typed_to_float(
        &self,
        val: &TypedValue<'ctx>,
    ) -> Result<inkwell::values::FloatValue<'ctx>, String> {
        match val {
            TypedValue::Float(fv) => Ok(*fv),
            TypedValue::Int(iv) => self
                .builder
                .build_signed_int_to_float(*iv, self.f64_ty(), "i2f")
                .map_err(|e| format!("LLVM error: {}", e)),
            _ => Err("Expected Int or Float".to_string()),
        }
    }

    pub(super) fn i64_ty(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i64_type()
    }
    pub(super) fn i32_ty(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i32_type()
    }
    pub(super) fn f64_ty(&self) -> inkwell::types::FloatType<'ctx> {
        self.context.f64_type()
    }
    pub(super) fn bool_ty(&self) -> inkwell::types::IntType<'ctx> {
        self.context.bool_type()
    }
    pub(super) fn void_ty(&self) -> inkwell::types::VoidType<'ctx> {
        self.context.void_type()
    }
    pub(super) fn ptr_ty(&self) -> inkwell::types::PointerType<'ctx> {
        self.context.ptr_type(inkwell::AddressSpace::default())
    }

    /// Compute the store size (bytes) of an LLVM type for x86-64 ABI.
    pub(super) fn type_store_size(&self, ty: BasicTypeEnum<'ctx>) -> u64 {
        match ty {
            BasicTypeEnum::IntType(it) => {
                let bw = it.get_bit_width() as u64;
                (bw + 7) / 8
            }
            BasicTypeEnum::FloatType(_) => 8,
            BasicTypeEnum::PointerType(_) => 8,
            BasicTypeEnum::StructType(st) => self.struct_store_size(st),
            BasicTypeEnum::ArrayType(at) => {
                let elem_size = self.type_store_size(at.get_element_type());
                let len = at.len() as u64;
                elem_size * len
            }
            _ => 8,
        }
    }

    fn struct_store_size(&self, st: StructType<'ctx>) -> u64 {
        let fields = st.get_field_types();
        if fields.is_empty() {
            return 0;
        }
        let mut max_align: u64 = 1;
        let mut offset: u64 = 0;
        for field in &fields {
            let f_size = self.type_store_size(*field);
            let f_align = self.type_alignment(*field);
            max_align = max_align.max(f_align);
            offset = (offset + f_align - 1) / f_align * f_align;
            offset += f_size;
        }
        (offset + max_align - 1) / max_align * max_align
    }

    fn type_alignment(&self, ty: BasicTypeEnum<'ctx>) -> u64 {
        match ty {
            BasicTypeEnum::IntType(it) => {
                let bw = it.get_bit_width() as u64;
                ((bw + 7) / 8).min(8)
            }
            BasicTypeEnum::FloatType(_) => 8,
            BasicTypeEnum::PointerType(_) => 8,
            BasicTypeEnum::StructType(st) => {
                let fields = st.get_field_types();
                if fields.is_empty() {
                    return 1;
                }
                fields
                    .iter()
                    .map(|f| self.type_alignment(*f))
                    .max()
                    .unwrap_or(8)
            }
            BasicTypeEnum::ArrayType(at) => self.type_alignment(at.get_element_type()),
            _ => 8,
        }
    }

    fn infer_hir_when_type(&self, w: &action_frontend::hir::HirWhen) -> Type {
        use action_frontend::hir::HirWhenKind;
        match &w.kind {
            HirWhenKind::OneLine {
                then_expr,
                else_expr,
                ..
            } => {
                let t = self.infer_hir_expr_type(then_expr);
                if matches!(t, Type::Unit) {
                    self.infer_hir_expr_type(else_expr)
                } else {
                    t
                }
            }
            HirWhenKind::ValueMatch { arms, .. } | HirWhenKind::ConditionChain { arms } => arms
                .first()
                .map(|a| self.infer_hir_expr_type(&a.body))
                .unwrap_or(Type::Unit),
        }
    }

    /// Infer expression type at codegen time from HIR, using scope for idents.
    /// HIR `expr.ty` can be stale for prelude/stdlib idents (e.g. param `x` tagged as
    /// `list` during lowering); scope matches frontend type inference behavior.
    pub(super) fn infer_hir_expr_type(&self, expr: &HirExpr) -> Type {
        match &expr.kind {
            HirExprKind::Ident(name) => {
                if let Some(sv) = self.scope.get(name) {
                    if let Some(ref ast_type) = sv.ast_type {
                        return ast_type.clone();
                    }
                    match sv.kind {
                        ValKind::Enum => {
                            for enum_name in self.type_layout.enum_types.keys() {
                                if sv.ty
                                    == (*self.type_layout.enum_types.get(enum_name).unwrap()).into()
                                {
                                    return Type::Named(enum_name.clone());
                                }
                            }
                            Type::Named("Int".into())
                        }
                        ValKind::Str => Type::Named("String".into()),
                        ValKind::List => Type::Named("List".into()),
                        ValKind::Map => Type::Named("Map".into()),
                        ValKind::Set => Type::Named("Set".into()),
                        ValKind::Fn => Type::Named("Int".into()),
                        _ => Type::Named("Int".into()),
                    }
                } else if self.registry.lookup_variant(name).is_some() {
                    let enum_name = self
                        .registry
                        .variant_to_enum
                        .get(name)
                        .cloned()
                        .unwrap_or_default();
                    Type::Named(enum_name)
                } else {
                    expr.ty.clone()
                }
            }
            HirExprKind::Literal(lit) => match lit {
                Literal::String(_) => Type::Named("String".into()),
                Literal::Int(_) => Type::Named("Int".into()),
                Literal::Float(_) => Type::Named("Float".into()),
                Literal::Bool(_) => Type::Named("Bool".into()),
                Literal::Char(_) => Type::Named("Char".into()),
                Literal::Unit => Type::Unit,
            },
            HirExprKind::Unary(_, inner) => self.infer_hir_expr_type(inner),
            HirExprKind::Binary(lhs, op, _) if *op == BinaryOp::Add => {
                if matches!(
                    self.infer_hir_expr_type(lhs),
                    Type::Named(ref n) if n == "String"
                ) {
                    Type::Named("String".into())
                } else {
                    Type::Named("Int".into())
                }
            }
            HirExprKind::Block(stmts) => stmts
                .last()
                .and_then(|s| match s {
                    action_frontend::hir::HirStmt::Expr { expr, .. } => {
                        Some(self.infer_hir_expr_type(expr))
                    }
                    _ => None,
                })
                .unwrap_or(Type::Unit),
            HirExprKind::When(w) => self.infer_hir_when_type(w),
            HirExprKind::Continue | HirExprKind::Break => Type::Unit,
            HirExprKind::For(_) => Type::Unit,
            HirExprKind::OrBlock { fallback, .. } => self.infer_hir_expr_type(fallback),
            _ => expr.ty.clone(),
        }
    }

    pub(super) fn build_fn_type(
        &mut self,
        ret_ast: Option<&Type>,
        name: &str,
        param_tys: &[BasicMetadataTypeEnum<'ctx>],
    ) -> Result<FunctionType<'ctx>, String> {
        match ret_ast {
            None => Ok(if name == "main" {
                self.void_ty().fn_type(param_tys, false)
            } else {
                self.fat_return_type.fn_type(param_tys, false)
            }),
            Some(Type::Unit) => Ok(self.void_ty().fn_type(param_tys, false)),
            Some(ty) => {
                let ret_ty = self.llvm_layout_for_type(ty)?;
                Ok(ret_ty.fn_type(param_tys, false))
            }
        }
    }

    pub(super) fn build_fallible_fn_type(
        &mut self,
        ret_ast: &Type,
        param_tys: &[BasicMetadataTypeEnum<'ctx>],
    ) -> Result<inkwell::types::FunctionType<'ctx>, String> {
        let payload = self.llvm_layout_for_type(ret_ast)?;
        let st = self
            .context
            .struct_type(&[payload, self.bool_ty().into()], false);
        Ok(st.fn_type(param_tys, false))
    }

    /// Mangle a function name by appending param types: add(Int,Float) → add_Int_Float
    pub(super) fn mangle_name(name: &str, param_types: &[Type]) -> String {
        action_frontend::types::mangle_name(name, param_types)
    }

    /// Map a TypedValue to a type name string for overload resolution.
    pub(super) fn typed_value_type_name(&self, v: &TypedValue<'ctx>) -> String {
        match v {
            TypedValue::Int(_) => "Int".to_string(),
            TypedValue::Float(_) => "Float".to_string(),
            TypedValue::Bool(_) => "Bool".to_string(),
            TypedValue::Str(_) => "String".to_string(),
            TypedValue::Fn(_, _) | TypedValue::Closure { .. } => "Fn".to_string(),
            TypedValue::List(_) => "List".to_string(),
            TypedValue::Struct(_, st) => {
                // Try to find the named struct type
                for (name, ty) in &self.type_layout.named_structs {
                    if *ty == *st {
                        return name.clone();
                    }
                }
                "Struct".to_string()
            }
            TypedValue::Enum(..) => {
                // Enum types are anonymous {i64, ptr} — for overload resolution
                // we use the registry to find the enum name
                "Enum".to_string()
            }
            TypedValue::Map(_) => "Map".to_string(),
            TypedValue::Set(_) => "Set".to_string(),
            TypedValue::Task(_) => "Task".to_string(),
            TypedValue::Stream(_) => "Stream".to_string(),
            TypedValue::LazyList(_) => "LazyList".to_string(),
            TypedValue::CString(_) => "CString".to_string(),
            TypedValue::Ptr(_) => "Ptr".to_string(),
            TypedValue::FileHandle(_) => "FileHandle".to_string(),
            TypedValue::Unit => "Unit".to_string(),
            TypedValue::FallibleInt { .. } => "Int".to_string(),
            TypedValue::FallibleFloat { .. } => "Float".to_string(),
            TypedValue::FalliblePtr { .. } => "Ptr".to_string(),
            TypedValue::FallibleStr { .. } => "String".to_string(),
            TypedValue::FallibleStruct { .. } => "Struct".to_string(),
        }
    }
    // ---- TypedValue helpers ----
}
impl<'ctx> TypedValue<'ctx> {
    pub(super) fn get_type_for_alloca(
        &self,
        cg: &CodeGen<'ctx>,
    ) -> inkwell::types::BasicTypeEnum<'ctx> {
        match self {
            TypedValue::CString(_) | TypedValue::Ptr(_) | TypedValue::FileHandle(_) => {
                cg.ptr_ty().into()
            }
            TypedValue::Struct(_, ty) => (*ty).into(),
            TypedValue::Enum(_, ty, ..) => (*ty).into(),
            TypedValue::Unit => cg.i64_ty().into(),
            TypedValue::Int(_) => cg.i64_ty().into(),
            TypedValue::Float(_) => cg.f64_ty().into(),
            TypedValue::Bool(_) => cg.bool_ty().into(),
            TypedValue::Str(_) => cg.string_type.into(),
            TypedValue::Fn(_, _) | TypedValue::Closure { .. } => cg.ptr_ty().into(),
            TypedValue::List(_) | TypedValue::Map(_) | TypedValue::Set(_) => cg.list_type.into(),
            TypedValue::Task(_) | TypedValue::Stream(_) => cg.ptr_ty().into(),
            TypedValue::LazyList(_) => cg.lazylist_type.into(),
            TypedValue::FallibleInt { .. } => cg.i64_ty().into(),
            TypedValue::FallibleFloat { .. } => cg.f64_ty().into(),
            TypedValue::FalliblePtr { .. } => cg.ptr_ty().into(),
            TypedValue::FallibleStr { .. } => cg.string_type.into(),
            TypedValue::FallibleStruct { ty, .. } => (*ty).into(),
        }
    }

    /// The actual LLVM type of the value (not the alloca pointer type).
    /// For strings this returns the {i64, ptr} struct, not ptr.
    pub(super) fn get_value_type(&self, cg: &CodeGen<'ctx>) -> inkwell::types::BasicTypeEnum<'ctx> {
        match self {
            TypedValue::Str(_) => cg.string_type.into(),
            TypedValue::List(_) | TypedValue::Map(_) | TypedValue::Set(_) => cg.list_type.into(),
            TypedValue::Stream(_) => cg.stream_type.into(),
            TypedValue::Task(_) => cg.task_type.into(),
            TypedValue::LazyList(_) => cg.lazylist_type.into(),
            TypedValue::Struct(_, ty) => (*ty).into(),
            TypedValue::Enum(_, ty, ..) => (*ty).into(),
            TypedValue::Bool(_) => cg.i64_ty().into(),
            TypedValue::Int(_) => cg.i64_ty().into(),
            TypedValue::Float(_) => cg.f64_ty().into(),
            TypedValue::CString(_) | TypedValue::Ptr(_) | TypedValue::FileHandle(_) => {
                cg.ptr_ty().into()
            }
            TypedValue::Unit | TypedValue::Fn(_, _) | TypedValue::Closure { .. } => {
                cg.ptr_ty().into()
            }
            TypedValue::FallibleInt { .. } => cg.i64_ty().into(),
            TypedValue::FallibleFloat { .. } => cg.f64_ty().into(),
            TypedValue::FalliblePtr { .. } => cg.ptr_ty().into(),
            TypedValue::FallibleStr { .. } => cg.string_type.into(),
            TypedValue::FallibleStruct { ty, .. } => (*ty).into(),
        }
    }

    pub(super) fn val_kind(&self) -> ValKind {
        match self {
            TypedValue::Int(_) => ValKind::Int,
            TypedValue::Float(_) => ValKind::Float,
            TypedValue::Bool(_) => ValKind::Bool,
            TypedValue::Str(_) => ValKind::Str,
            TypedValue::Fn(_, _) | TypedValue::Closure { .. } => ValKind::Fn,
            TypedValue::List(_) => ValKind::List,
            TypedValue::Map(_) => ValKind::Map,
            TypedValue::Set(_) => ValKind::Set,
            TypedValue::Task(_) => ValKind::Task,
            TypedValue::Stream(_) => ValKind::Stream,
            TypedValue::LazyList(_) => ValKind::LazyList,
            TypedValue::CString(_) => ValKind::CString,
            TypedValue::Ptr(_) => ValKind::Ptr,
            TypedValue::FileHandle(_) => ValKind::FileHandle,
            TypedValue::Struct(_, _) => ValKind::Struct,
            TypedValue::Enum(..) => ValKind::Enum,
            TypedValue::Unit => ValKind::Unit,
            TypedValue::FallibleInt { .. } => ValKind::Int,
            TypedValue::FallibleFloat { .. } => ValKind::Float,
            TypedValue::FalliblePtr { .. } => ValKind::Ptr,
            TypedValue::FallibleStr { .. } => ValKind::Str,
            TypedValue::FallibleStruct { .. } => ValKind::Struct,
        }
    }
}
