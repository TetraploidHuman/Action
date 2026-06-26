//! Typed LLVM values produced by expression compilation.

use inkwell::types::{FunctionType, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};

/// The type of value stored inside an enum variant (Some/Ok).
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum InnerType {
    Int,
    Float,
    Str,
}

#[derive(Clone, Copy)]
pub(crate) enum TypedValue<'ctx> {
    Int(IntValue<'ctx>),
    Float(inkwell::values::FloatValue<'ctx>),
    Bool(IntValue<'ctx>),
    Str(PointerValue<'ctx>),
    Fn(PointerValue<'ctx>, FunctionType<'ctx>),
    Closure {
        fn_ptr: PointerValue<'ctx>,
        actual_fn_type: FunctionType<'ctx>,
        closure_ptr: PointerValue<'ctx>,
        closure_ty: StructType<'ctx>,
        alloca: Option<PointerValue<'ctx>>,
        /// Bit i set when capture field i is an RC-managed heap closure cap (not plain fn ptr).
        capture_ptr_rc_mask: u64,
    },
    List(PointerValue<'ctx>),
    Struct(PointerValue<'ctx>, StructType<'ctx>),
    Enum(PointerValue<'ctx>, StructType<'ctx>, InnerType, bool),
    Map(PointerValue<'ctx>),
    Set(PointerValue<'ctx>),
    Task(PointerValue<'ctx>),
    Stream(PointerValue<'ctx>),
    LazyList(PointerValue<'ctx>),
    CString(PointerValue<'ctx>),
    Ptr(PointerValue<'ctx>),
    FileHandle(PointerValue<'ctx>),
    /// Fallible-region intermediate: Int payload + ok flag (i1).
    FallibleInt {
        val: IntValue<'ctx>,
        ok: IntValue<'ctx>,
    },
    /// Fallible-region intermediate: Float payload + ok flag (i1).
    FallibleFloat {
        val: inkwell::values::FloatValue<'ctx>,
        ok: IntValue<'ctx>,
    },
    /// Fallible-region intermediate: Ptr payload + ok flag (i1).
    FalliblePtr {
        val: PointerValue<'ctx>,
        ok: IntValue<'ctx>,
    },
    /// Fallible-region intermediate: String payload (alloca) + ok flag (i1).
    FallibleStr {
        val: PointerValue<'ctx>,
        ok: IntValue<'ctx>,
    },
    /// Fallible-region intermediate: struct payload (alloca) + ok flag (i1).
    FallibleStruct {
        val: PointerValue<'ctx>,
        ty: StructType<'ctx>,
        ok: IntValue<'ctx>,
    },
    Unit,
}

impl<'ctx> TypedValue<'ctx> {
    pub(crate) fn to_bv(&self) -> Option<BasicValueEnum<'ctx>> {
        match self {
            TypedValue::Int(v) => Some(v.as_basic_value_enum()),
            TypedValue::Float(v) => Some(v.as_basic_value_enum()),
            TypedValue::Bool(v) => Some(v.as_basic_value_enum()),
            TypedValue::Str(_) => None,
            TypedValue::Fn(ptr, _) => Some(ptr.as_basic_value_enum()),
            TypedValue::Closure { closure_ptr, .. } => Some(closure_ptr.as_basic_value_enum()),
            TypedValue::List(_) => None,
            TypedValue::Map(_) => None,
            TypedValue::Set(_) => None,
            TypedValue::Task(_) => None,
            TypedValue::Stream(_)
            | TypedValue::LazyList(_)
            | TypedValue::CString(_)
            | TypedValue::FileHandle(_) => None,
            TypedValue::Ptr(v) => Some(v.as_basic_value_enum()),
            TypedValue::Struct(_, _) => None,
            TypedValue::Enum(..) => None,
            TypedValue::FallibleInt { .. } => None,
            TypedValue::FallibleFloat { .. } => None,
            TypedValue::FalliblePtr { .. } => None,
            TypedValue::FallibleStr { .. } => None,
            TypedValue::FallibleStruct { .. } => None,
            TypedValue::Unit => None,
        }
    }
}
