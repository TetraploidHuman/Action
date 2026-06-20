// Submodule: builtins_list

use super::call_arg::CallArg;
use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn builtin_list(
        &mut self,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        let len = self.i64_ty().const_int(args.len() as u64, false);
        let cc = self.call_rt("action_list_create", &[len.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let list_alloca = self
            .builder
            .build_alloca(self.list_type, "list_tmp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(list_alloca, list_bv)
            .map_err(llvm_err)?;

        for arg in args {
            let v = self.compile_call_arg(*arg)?;
            // action_list_push handles rc_inc of the element data_ptr internally
            let elem_fat = self.to_fat_struct(&v)?;
            let list_val = self.load_list(list_alloca)?;
            let cc = self.call_rt("action_list_push", &[list_val.into(), elem_fat.into()])?;
            let new_list = cc.try_as_basic_value().basic().ok_or("list_push failed")?;
            self.builder
                .build_store(list_alloca, new_list)
                .map_err(llvm_err)?;
        }

        Ok(TypedValue::List(list_alloca))
    }

    /// lazy_list(seed) - create a lazy list with a seed value
    /// lazy_list(seed) { fn } - create a lazy list with seed and step function
    pub(super) fn builtin_lazy_list(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if args.is_empty() {
            return Err("lazy_list expects at least 1 argument (seed)".to_string());
        }
        let seed = self.compile_call_arg(args[0])?;
        let seed_i64 = match &seed {
            TypedValue::Int(v) => *v,
            _ => return Err("lazy_list: seed must be an Int".to_string()),
        };

        // Compile step function if provided
        let (step_fn_ptr, state, take_count) = if let Some(lam) = trailing {
            let step_fn_val = self.compile_lambda_for_lazy_call_arg(lam)?;
            // -1 means "infinite" — only bounded by explicit take()
            (
                step_fn_val,
                seed_i64,
                self.i64_ty().const_int(-1_i64 as u64, true),
            )
        } else {
            // No step function: only the seed element
            (
                self.ptr_ty().const_null(),
                self.i64_ty().const_int(0, false),
                self.i64_ty().const_int(0, false),
            )
        };

        // Build LazyList struct: {head_val: i64, step_fn: i8*, state: i64, take_count: i64, map_fn: i8*}
        let ll_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "ll")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, seed_i64, 0, "ll_head")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn_ptr, 1, "ll_fn")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state, 2, "ll_state")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, take_count, 3, "ll_tc")
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

    /// Compile a lambda CallArg for use as a lazy list step function.
    fn compile_lambda_for_lazy_call_arg(
        &mut self,
        lam: CallArg<'_>,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match lam {
            CallArg::Hir(h) => match &h.kind {
                action_frontend::hir::HirExprKind::Lambda { params, body, .. } => {
                    if params.is_empty() {
                        return Err("lazy_list step function expects 1 parameter".to_string());
                    }
                    let fn_val = self.compile_lambda_hir(params, body)?;
                    match fn_val {
                        TypedValue::Fn(ptr, _) => Ok(ptr),
                        TypedValue::Closure { fn_ptr, .. } => Ok(fn_ptr),
                        _ => Err("lazy_list: step function compilation failed".to_string()),
                    }
                }
                _ => Err("lazy_list: expected lambda body".to_string()),
            },
        }
    }

    /// Compile a lambda for use as a lazy list step function.
    /// Returns a function pointer that can be called with (i64 state) -> next_i64.

    // ---- LazyList operations ----

    /// If the value is a LazyList, convert it to a List and return the list alloca pointer.
    /// If it's already a List, return the pointer directly.
    pub(super) fn ensure_list_ptr(
        &self,
        val: &TypedValue<'ctx>,
        prefix: &str,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match val {
            TypedValue::LazyList(_) => {
                let list_sv = self.convert_lazylist_to_list(val)?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, &format!("{}_list", prefix))
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, list_sv)
                    .map_err(llvm_err)?;
                Ok(alloca)
            }
            TypedValue::List(p) => Ok(*p),
            _ => Err(format!("{}: argument must be a List or LazyList", prefix)),
        }
    }

    /// Convert a LazyList to a List struct value (i.e., the loaded StructValue of the list).
    /// This forces evaluation via runtime `action_lazylist_to_list`.
    pub(super) fn convert_lazylist_to_list(
        &self,
        ll_val: &TypedValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let ll_ptr = match ll_val {
            TypedValue::LazyList(p) => *p,
            _ => return Err("convert_lazylist_to_list: expected LazyList".to_string()),
        };
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, ll_ptr, "ll_conv")
            .map_err(llvm_err)?;
        let cc = self.call_rt("action_lazylist_to_list", &[ll_sv.into()])?;
        cc.try_as_basic_value()
            .basic()
            .ok_or_else(|| "action_lazylist_to_list returned void".to_string())
            .map(|bv| bv.into_struct_value())
    }

    /// Create a fat struct {i64, i8*} from an i64 value (using string_type to match list_push expectations)
    pub(super) fn make_int_fat(
        &self,
        val: inkwell::values::IntValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let undef = self.string_type.get_undef();
        let null_ptr = self.ptr_ty().const_null();
        let aggregate = self
            .builder
            .build_insert_value(undef, val, 0, "fat_v")
            .map_err(llvm_err)?;
        let aggregate2 = self
            .builder
            .build_insert_value(aggregate, null_ptr, 1, "fat_p")
            .map_err(llvm_err)?;
        Ok(aggregate2.into_struct_value())
    }
}
