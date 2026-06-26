//! Lexical scope for codegen: variable bindings and their LLVM representations.

use super::typed_value::InnerType;
use action_frontend::ast::Type;
use action_frontend::hir::HirExpr;
use inkwell::types::{BasicTypeEnum, FunctionType, StructType};
use inkwell::values::PointerValue;
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum ValKind {
    Int,
    Float,
    Bool,
    Str,
    Fn,
    List,
    Map,
    Set,
    Task,
    Stream,
    LazyList,
    CString,
    Ptr,
    FileHandle,
    Struct,
    Enum,
    Unit,
}

#[derive(Clone)]
pub(crate) struct ScopeVar<'ctx> {
    pub(crate) ptr: PointerValue<'ctx>,
    pub(crate) ty: BasicTypeEnum<'ctx>,
    pub(crate) kind: ValKind,
    pub(crate) fn_type: Option<FunctionType<'ctx>>,
    pub(crate) mutable: bool,
    pub(crate) lazy_flag: Option<PointerValue<'ctx>>,
    pub(crate) lazy_init_expr: Option<HirExpr>,
    pub(crate) ast_type: Option<Type>,
    pub(crate) enum_inner_type: Option<InnerType>,
    pub(crate) enum_data_rc_managed: bool,
    pub(crate) is_closure: bool,
    pub(crate) closure_ty: Option<StructType<'ctx>>,
    pub(crate) closure_fn_ptr: Option<PointerValue<'ctx>>,
    pub(crate) actual_fn_type: Option<FunctionType<'ctx>>,
    pub(crate) closure_capture_ptr_rc_mask: u64,
}

#[derive(Clone)]
pub(crate) struct Scope<'ctx> {
    variables: HashMap<String, ScopeVar<'ctx>>,
    pub(crate) parent: Option<Box<Scope<'ctx>>>,
}

impl<'ctx> Scope<'ctx> {
    pub(crate) fn new() -> Self {
        Scope {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub(crate) fn with_parent(parent: Scope<'ctx>) -> Self {
        Scope {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<&ScopeVar<'ctx>> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }

    pub(crate) fn set(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        kind: ValKind,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type: None,
                mutable: false,
                lazy_flag: None,
                lazy_init_expr: None,
                ast_type: None,
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
                closure_capture_ptr_rc_mask: 0,
            },
        );
    }

    pub(crate) fn set_with_fn_type(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        kind: ValKind,
        fn_type: Option<FunctionType<'ctx>>,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type,
                mutable: false,
                lazy_flag: None,
                lazy_init_expr: None,
                ast_type: None,
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
                closure_capture_ptr_rc_mask: 0,
            },
        );
    }

    pub(crate) fn set_mutable(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        kind: ValKind,
        fn_type: Option<FunctionType<'ctx>>,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type,
                mutable: true,
                lazy_flag: None,
                lazy_init_expr: None,
                ast_type: None,
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
                closure_capture_ptr_rc_mask: 0,
            },
        );
    }

    pub(crate) fn set_lazy(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        kind: ValKind,
        flag: PointerValue<'ctx>,
        init_expr: HirExpr,
        ast_type: Option<Type>,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type: None,
                mutable: false,
                lazy_flag: Some(flag),
                lazy_init_expr: Some(init_expr),
                ast_type,
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
                closure_capture_ptr_rc_mask: 0,
            },
        );
    }

    pub(crate) fn set_with_ast_type(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
        kind: ValKind,
        fn_type: Option<FunctionType<'ctx>>,
        ast_type: Type,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type,
                mutable: false,
                lazy_flag: None,
                lazy_init_expr: None,
                ast_type: Some(ast_type),
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
                closure_capture_ptr_rc_mask: 0,
            },
        );
    }

    pub(crate) fn set_closure_info(
        &mut self,
        name: &str,
        closure_ty: StructType<'ctx>,
        closure_fn_ptr: PointerValue<'ctx>,
        actual_fn_type: FunctionType<'ctx>,
        capture_ptr_rc_mask: u64,
    ) {
        if let Some(var) = self.variables.get_mut(name) {
            var.is_closure = true;
            var.closure_ty = Some(closure_ty);
            var.closure_fn_ptr = Some(closure_fn_ptr);
            var.actual_fn_type = Some(actual_fn_type);
            var.fn_type = None;
            var.closure_capture_ptr_rc_mask = capture_ptr_rc_mask;
        }
    }

    pub(crate) fn set_fn_type(
        &mut self,
        name: &str,
        fn_type: Option<FunctionType<'ctx>>,
    ) {
        if let Some(var) = self.variables.get_mut(name) {
            var.fn_type = fn_type;
            var.is_closure = false;
            var.closure_ty = None;
            var.closure_fn_ptr = None;
            var.actual_fn_type = None;
            var.closure_capture_ptr_rc_mask = 0;
        }
    }

    pub(crate) fn set_enum_inner_type(&mut self, name: &str, inner_type: InnerType) {
        if let Some(var) = self.variables.get_mut(name) {
            var.enum_inner_type = Some(inner_type);
        }
    }

    pub(crate) fn set_enum_data_rc_managed(&mut self, name: &str, managed: bool) {
        if let Some(var) = self.variables.get_mut(name) {
            var.enum_data_rc_managed = managed;
        }
    }

    pub(crate) fn remove_var(&mut self, name: &str) {
        self.variables.remove(name);
    }

    pub(crate) fn local_variables(&self) -> &HashMap<String, ScopeVar<'ctx>> {
        &self.variables
    }
}
