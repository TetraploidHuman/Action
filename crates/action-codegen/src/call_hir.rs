// HIR-native call dispatch (release path).

use action_frontend::ast::Literal;
use action_frontend::hir::{HirExpr, HirExprKind, HirStmt};

use super::builtin_dispatch::BuiltinDispatch;
use super::call_arg::CallArg;
use super::{CodeGen, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_call_hir(
        &mut self,
        func: &HirExpr,
        args: &[HirExpr],
        trailing: Option<&Box<HirExpr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match &func.kind {
            HirExprKind::Ident(name) => {
                let call_args = Self::call_args_from_hir(args);
                let trailing_ca = Self::trailing_call_arg_hir(trailing);
                self.dispatch_named_call(name, &call_args, trailing_ca)
            }
            HirExprKind::FunctionRef(_) => {
                let call_args = Self::call_args_from_hir(args);
                let trailing_ca = Self::trailing_call_arg_hir(trailing);
                if let Some(fn_name) = Self::resolve_direct_fn_name_hir(self, func) {
                    return self.compile_direct_function_call_from_call_args(
                        &fn_name, &call_args, trailing_ca,
                    );
                }
                let target = self.compile_hir_expr(func)?;
                self.compile_indirect_call_impl(target, &call_args, trailing_ca)
            }
            HirExprKind::FieldAccess(receiver, method) => {
                self.compile_ufcs_call_hir(receiver, method, args, trailing)
            }
            _ => {
                let target = self.compile_hir_expr(func)?;
                let call_args = Self::call_args_from_hir(args);
                let trailing_ca = Self::trailing_call_arg_hir(trailing);
                self.compile_indirect_call_impl(target, &call_args, trailing_ca)
            }
        }
    }

    /// Shared ident-call router for HIR release path and UFCS fallbacks.
    pub(super) fn dispatch_named_call(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if let Some(scope_var) = self.scope.get(name) {
            if scope_var.kind == ValKind::Fn {
                let target = self.compile_ident(name)?;
                if let Some(result) = self.try_devirtualize_fn_call(&target, args, trailing)? {
                    return Ok(result);
                }
                return self.compile_indirect_call_from_call_args(target, args, trailing);
            }
        }

        if name == "__list" {
            return self.builtin_list(args);
        }
        if name == "lazy_list" {
            return self.builtin_lazy_list(args, trailing);
        }
        if name == "launch" {
            return self.builtin_launch(args, trailing);
        }
        if name == "coroutineScope" {
            return self.builtin_coroutine_scope(args, trailing);
        }
        if name == "delay" {
            return self.builtin_delay(args);
        }
        if name == "withTimeout" {
            return self.builtin_with_timeout(args, trailing);
        }
        if name == "Stream" {
            return self.builtin_stream_create();
        }
        if name == "send" || name == "receive" || name == "close" {
            return self.builtin_stream_op(name, args);
        }
        if name == "cancel" || name == "is_done" || name == "is_cancelled" || name == "wait" {
            return self.builtin_task_op(name, args);
        }

        if matches!(
            name,
            "find"
                | "findIndex"
                | "reduce"
                | "foldRight"
                | "takeWhile"
                | "dropWhile"
                | "sortedBy"
                | "partition"
                | "count"
        ) {
            return self.maybe_builtin_callback_list_call_args(name, args, trailing);
        }

        if let Some(generic_stmt) = self.mono_cache.generic_fun_defs.get(name).cloned() {
            if let HirStmt::Fun { type_params, .. } = &generic_stmt {
                if !type_params.is_empty() {
                    return self.compile_generic_call_from_call_args(
                        &generic_stmt,
                        name,
                        args,
                        trailing,
                    );
                }
            }
        }

        if let Some(def) = action_frontend::builtin::lookup(name) {
            match BuiltinDispatch::for_builtin(def) {
                BuiltinDispatch::Print => return self.builtin_print(name, args),
                BuiltinDispatch::Map => return self.builtin_map(args, trailing),
                BuiltinDispatch::Filter => return self.builtin_filter(args, trailing),
                BuiltinDispatch::Fold => return self.builtin_fold(args, trailing),
                BuiltinDispatch::CallbackList => {
                    let list_arg_idx =
                        BuiltinDispatch::list_operand_index(def, trailing.is_some(), args.len());
                    let is_list_op = list_arg_idx.map_or(false, |idx| {
                        idx < args.len()
                            && matches!(self.compile_call_arg(args[idx]), Ok(TypedValue::List(_)))
                    });
                    if is_list_op {
                        return self.builtin_callback_list(name, args, trailing);
                    }
                }
                BuiltinDispatch::Stdlib => {
                    if trailing.is_some()
                        && (name == "lazyMap" || name == "lazyFilter" || name == "lazyTakeWhile")
                    {
                        let mut new_args = vec![trailing.unwrap()];
                        new_args.extend_from_slice(args);
                        return self.builtin_stdlib(name, &new_args);
                    }
                    return self.builtin_stdlib(name, args);
                }
            }
        }

        if let Some((enum_info, variant)) = self
            .registry
            .lookup_variant(name)
            .map(|(ei, vi)| (ei.clone(), vi.clone()))
        {
            return self.compile_enum_construct(&enum_info, &variant, args);
        }

        if name == "flatMap" {
            return self.maybe_builtin_flat_map_call_args(args, trailing);
        }
        if matches!(name, "mapFilter" | "mapMapValues" | "mapFold") {
            return self.builtin_callback_map(name, args, trailing);
        }

        if let Some(overloads) = self.overloaded_functions.get(name).cloned() {
            return self.compile_overloaded_call_from_call_args(name, &overloads, args, trailing);
        }

        // Stdlib builtins before runtime LLVM symbols (abs/min/max/pow share names with lib helpers).
        match self.dispatch_stdlib_ident(name, args, trailing) {
            Ok(v) => return Ok(v),
            Err(e) if e.starts_with("Unknown builtin:") => {}
            Err(e) => return Err(e),
        }

        if self.module.get_function(name).is_some() {
            return self.compile_direct_function_call_from_call_args(name, args, trailing);
        }

        Err(format!("Unknown function or builtin: {}", name))
    }

    pub(super) fn resolve_user_fn_llvm_name(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<String, String> {
        if let Some(overloads) = self.overloaded_functions.get(name).cloned() {
            let arg_vals: Vec<TypedValue<'ctx>> = args
                .iter()
                .map(|a| self.compile_call_arg(*a))
                .collect::<Result<_, _>>()?;
            let arg_type_names: Vec<String> = arg_vals
                .iter()
                .map(|v| self.typed_value_type_name(v))
                .collect();
            let mangled = if arg_type_names.is_empty() {
                name.to_string()
            } else {
                format!("{}_{}", name, arg_type_names.join("_"))
            };
            if overloads.iter().any(|(_, mn)| mn == &mangled) {
                return Ok(mangled);
            }
        }
        if self.module.get_function(name).is_some() {
            return Ok(name.to_string());
        }
        Err(format!("Function '{}' not found", name))
    }

    fn dispatch_stdlib_ident(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if trailing.is_some()
            && (name == "lazyMap" || name == "lazyFilter" || name == "lazyTakeWhile")
        {
            let mut new_args = vec![trailing.unwrap()];
            new_args.extend_from_slice(args);
            return self.builtin_stdlib(name, &new_args);
        }
        self.builtin_stdlib(name, args)
    }

    fn compile_ufcs_call_hir(
        &mut self,
        receiver: &HirExpr,
        method: &str,
        args: &[HirExpr],
        trailing: Option<&Box<HirExpr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if method == "filter" {
            if let Some((map_fn, inner)) = Self::extract_map_call_args_hir(receiver) {
                let filter_fn = if let Some(lam) = trailing {
                    self.compile_hir_expr(lam)?
                } else if args.len() == 1 {
                    self.compile_hir_expr(&args[0])?
                } else {
                    return Err(
                        "filter with trailing lambda expects 0 args; filter(fn, list) expects 2"
                            .to_string(),
                    );
                };
                return self.fused_map_filter_hir(map_fn, inner, filter_fn);
            }
            if let Some((flat_fn, inner)) = Self::extract_flatmap_call_args_hir(receiver) {
                let filter_fn = if let Some(lam) = trailing {
                    self.compile_hir_expr(lam)?
                } else if args.len() == 1 {
                    self.compile_hir_expr(&args[0])?
                } else {
                    return Err(
                        "filter with trailing lambda expects 0 args; filter(fn, list) expects 2"
                            .to_string(),
                    );
                };
                return self.fused_flatmap_filter_hir(flat_fn, inner, filter_fn);
            }
        }
        if method == "map" {
            if let Some((filter_fn, inner)) = Self::extract_filter_call_args_hir(receiver) {
                let map_fn = if let Some(lam) = trailing {
                    self.compile_hir_expr(lam)?
                } else if args.len() == 1 {
                    self.compile_hir_expr(&args[0])?
                } else {
                    return Err("map with trailing lambda expects 0 args".to_string());
                };
                if let Some((map_inner, base_list)) = Self::extract_map_call_args_hir(inner) {
                    let filter_fn_val = self.compile_hir_expr(filter_fn)?;
                    if let Some(lam) = trailing {
                        if Self::is_identity_lambda_hir(lam) {
                            return self.fused_map_filter_hir(map_inner, base_list, filter_fn_val);
                        }
                        let map_fn_val = self.compile_hir_expr(lam)?;
                        return self.fused_map_filter_map_hir(
                            map_inner,
                            base_list,
                            filter_fn_val,
                            map_fn_val,
                        );
                    }
                }
                let inner_val = self.compile_hir_expr(inner)?;
                if matches!(inner_val, TypedValue::LazyList(_)) {
                    return self.fused_lazy_filter_map_hir(filter_fn, inner, map_fn);
                }
                return self.fused_filter_map_hir(filter_fn, inner, map_fn);
            }
        }
        if method == "takeWhile" {
            if let Some((map_fn, inner)) = Self::extract_map_call_args_hir(receiver) {
                let tw_fn = if let Some(lam) = trailing {
                    self.compile_hir_expr(lam)?
                } else if args.len() == 1 {
                    self.compile_hir_expr(&args[0])?
                } else {
                    return Err("takeWhile with trailing lambda expects 0 args".to_string());
                };
                return self.fused_map_take_while_hir(map_fn, inner, tw_fn);
            }
        }
        if method == "fold" {
            if let Some((map_lam, base_list)) = Self::extract_map_call_args_hir(receiver) {
                if args.len() == 1 {
                    let fold_lam = trailing.ok_or(
                        "fold on map receiver expects trailing lambda: lst.map{}.fold(init){}",
                    )?;
                    return self.fused_map_fold_hir(
                        map_lam,
                        fold_lam.as_ref(),
                        base_list,
                        &args[0],
                    );
                }
            }
            if let Some((filter_lam, base_list)) = Self::extract_filter_call_args_hir(receiver) {
                if args.len() == 1 {
                    let fold_lam = trailing.ok_or(
                        "fold on filter receiver expects trailing lambda: lst.filter{}.fold(init){}",
                    )?;
                    return self.fused_filter_fold_hir(
                        filter_lam,
                        fold_lam.as_ref(),
                        base_list,
                        &args[0],
                    );
                }
            }
        }
        if method == "reduce" {
            if let Some((map_lam, base_list)) = Self::extract_map_call_args_hir(receiver) {
                let reduce_lam = trailing
                    .ok_or("reduce on map receiver expects trailing lambda: lst.map{}.reduce{}")?;
                if !args.is_empty() {
                    return Err("reduce on map receiver does not take positional args".to_string());
                }
                return self.fused_map_reduce_hir(map_lam, reduce_lam.as_ref(), base_list);
            }
        }

        let call_args = Self::call_args_from_hir(args);
        let trailing_ca = Self::trailing_call_arg_hir(trailing);
        self.compile_ufcs_method(CallArg::hir(receiver), method, &call_args, trailing_ca)
    }

    pub(super) fn ufcs_forward_call(
        &mut self,
        name: &str,
        receiver: CallArg<'_>,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let mut all_args = vec![receiver];
        all_args.extend_from_slice(args);
        self.dispatch_named_call(name, &all_args, trailing)
    }

    fn compile_direct_function_call_from_call_args(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if self.in_fallible_region() && self.is_fallible_user_fn(name) {
            return self.compile_fallible_user_call(name, args, trailing);
        }

        if name == "fib" && args.len() == 1 && trailing.is_none() {
            let CallArg::Hir(hir) = &args[0];
            if let HirExprKind::Literal(Literal::Int(n)) = &hir.kind {
                if *n >= 0 && *n <= 92 {
                    let v = Self::consteval_fib(*n as u64);
                    return Ok(TypedValue::Int(self.i64_ty().const_int(v, false)));
                }
            }
        }

        if name == "fact" && args.len() == 2 && trailing.is_none() {
            let CallArg::Hir(n_hir) = &args[0];
            let CallArg::Hir(acc_hir) = &args[1];
            if let (
                HirExprKind::Literal(Literal::Int(n)),
                HirExprKind::Literal(Literal::Int(acc)),
            ) = (&n_hir.kind, &acc_hir.kind)
            {
                if *n >= 0 && *n <= 20 && *acc >= 0 {
                    let v = Self::consteval_fact(*n as u64, *acc as u64);
                    return Ok(TypedValue::Int(self.i64_ty().const_int(v, false)));
                }
            }
        }

        if name == "apply" && args.len() == 2 && trailing.is_none() {
            let CallArg::Hir(hir) = &args[0];
            if let Some(fn_name) = Self::resolve_direct_fn_name_hir(self, hir) {
                return self.compile_direct_function_call_from_call_args(
                    &fn_name,
                    &[args[1]],
                    None,
                );
            }
            if let HirExprKind::Lambda { .. } = &hir.kind {
                if let Ok(lam_val) = self.compile_hir_expr(hir) {
                    if let Some(result) =
                        self.try_devirtualize_unary_lambda_call(&lam_val, args[1])?
                    {
                        return Ok(result);
                    }
                }
            }
            if let Ok(fn_val) = self.compile_call_arg(args[0]) {
                if let Some(fn_name) = self.fn_ptr_to_module_name(&fn_val) {
                    return self.compile_direct_function_call_from_call_args(
                        &fn_name,
                        &[args[1]],
                        None,
                    );
                }
                if let Some(result) = self.try_devirtualize_unary_lambda_call(&fn_val, args[1])? {
                    return Ok(result);
                }
            }
        }

        let fn_val = self
            .module
            .get_function(name)
            .ok_or_else(|| format!("Function '{}' not found", name))?;
        let fn_type = fn_val.get_type();
        let param_tys = fn_type.get_param_types();
        let mut ca = Vec::new();
        let mut direct_arg_vals = Vec::new();
        for (i, a) in args.iter().enumerate() {
            let av = self.compile_call_arg(*a)?;
            let bv = self.typed_value_to_bv(&av);
            let casted = self.coerce_arg(bv, param_tys.get(i))?;
            ca.push(casted.into());
            direct_arg_vals.push(av);
        }
        if let Some(lam) = trailing {
            let bv = self.compile_and_load_call_arg(lam)?;
            let casted = self.coerce_arg(bv, param_tys.get(args.len()))?;
            ca.push(casted.into());
        }
        let cc = self
            .builder
            .build_call(fn_val, &ca, "")
            .map_err(super::llvm_err)?;
        for av in &direct_arg_vals {
            self.rc_free_intermediate(av)?;
        }
        let ast_ret = self.mono_cache.fun_return_types.get(name).cloned();
        match cc.try_as_basic_value().basic() {
            Some(bv) => self.unpack_call_return(bv, fn_type.get_return_type(), ast_ret.as_ref()),
            None => Ok(TypedValue::Unit),
        }
    }

    fn compile_overloaded_call_from_call_args(
        &mut self,
        name: &str,
        overloads: &[(Vec<action_frontend::ast::Type>, String)],
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let arg_vals: Vec<TypedValue<'ctx>> = args
            .iter()
            .map(|a| self.compile_call_arg(*a))
            .collect::<Result<_, _>>()?;
        let arg_type_names: Vec<String> = arg_vals
            .iter()
            .map(|v| self.typed_value_type_name(v))
            .collect();
        let mangled = if arg_type_names.is_empty() {
            name.to_string()
        } else {
            format!("{}_{}", name, arg_type_names.join("_"))
        };
        let fn_name = overloads
            .iter()
            .find(|(_, mn)| mn == &mangled)
            .map(|(_, mn)| mn.as_str())
            .ok_or_else(|| {
                format!(
                    "No matching overload of '{}' for argument types: {:?}",
                    name, arg_type_names
                )
            })?;
        self.compile_direct_function_call_from_call_args(fn_name, args, trailing)
    }

    fn maybe_builtin_flat_map_call_args(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let list_arg_idx = if trailing.is_some() {
            Some(0)
        } else if args.len() >= 2 {
            Some(1)
        } else {
            None
        };
        let is_list_op = list_arg_idx.map_or(false, |idx| {
            idx < args.len() && matches!(self.compile_call_arg(args[idx]), Ok(TypedValue::List(_)))
        });
        if is_list_op {
            return self.builtin_flat_map_list(args, trailing);
        }
        self.dispatch_named_call("flatMap", args, trailing)
    }

    fn maybe_builtin_callback_list_call_args(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let list_arg_idx = if name == "foldRight" {
            if trailing.is_some() && args.len() >= 2 {
                Some(0)
            } else if args.len() >= 3 {
                Some(1)
            } else {
                None
            }
        } else if trailing.is_some() {
            Some(0)
        } else if args.len() >= 2 {
            if matches!(self.compile_call_arg(args[0]), Ok(TypedValue::List(_))) {
                Some(0)
            } else {
                Some(1)
            }
        } else {
            None
        };
        let is_list_op = list_arg_idx.map_or(false, |idx| {
            idx < args.len() && matches!(self.compile_call_arg(args[idx]), Ok(TypedValue::List(_)))
        });
        if is_list_op && name == "takeWhile" {
            if let Some(list_arg) = list_arg_idx.map(|i| args[i]) {
                let CallArg::Hir(list_hir) = list_arg;
                if let Some((map_fn, inner)) = Self::extract_map_call_args_hir(list_hir) {
                    let tw_fn = if let Some(lam) = trailing {
                        self.compile_call_arg(lam)?
                    } else {
                        self.compile_call_arg(args[0])?
                    };
                    return self.fused_map_take_while_hir(map_fn, inner, tw_fn);
                }
            }
        }
        if is_list_op {
            return self.builtin_callback_list(name, args, trailing);
        }
        self.dispatch_named_call(name, args, trailing)
    }

    pub(super) fn compile_indirect_call_from_call_args(
        &mut self,
        target: TypedValue<'ctx>,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_indirect_call_impl(target, args, trailing)
    }

    pub(super) fn compile_indirect_call_impl(
        &mut self,
        target: TypedValue<'ctx>,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if let Some(result) = self.try_devirtualize_fn_call(&target, args, trailing)? {
            return Ok(result);
        }
        use inkwell::types::BasicMetadataTypeEnum;
        use inkwell::values::BasicMetadataValueEnum;
        match target {
            TypedValue::Fn(fn_ptr, fn_type) => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                for a in args {
                    let av = self.compile_call_arg(*a)?;
                    ca.push(self.typed_value_to_bv(&av).into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    ca.push(self.compile_and_load_call_arg(lam)?.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect")
                    .map_err(super::llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Closure {
                fn_ptr,
                actual_fn_type,
                closure_ptr,
                closure_ty,
                alloca,
                capture_ptr_rc_mask,
            } => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                ca.push(closure_ptr.into());
                for a in args {
                    let av = self.compile_call_arg(*a)?;
                    ca.push(self.typed_value_to_bv(&av).into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    ca.push(self.compile_and_load_call_arg(lam)?.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(actual_fn_type, fn_ptr, &ca, "indirect_closure")
                    .map_err(super::llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                if alloca.is_none() {
                    self.rc_inc(closure_ptr)?;
                    self.rc_dec_closure_captures(
                        closure_ptr,
                        closure_ty,
                        capture_ptr_rc_mask,
                        &[],
                    )?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, actual_fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Int(iv) => {
                let total_args = args.len() + trailing.map_or(0, |_| 1);
                let param_tys: Vec<BasicMetadataTypeEnum<'ctx>> =
                    (0..total_args).map(|_| self.i64_ty().into()).collect();
                let ret_ty = self.fat_return_type;
                let fn_type = ret_ty.fn_type(&param_tys, false);
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_ptr = self
                    .builder
                    .build_int_to_ptr(iv, ptr_type, "fn_ptr_cast")
                    .map_err(super::llvm_err)?;
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                for a in args {
                    let av = self.compile_call_arg(*a)?;
                    ca.push(self.typed_value_to_bv(&av).into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    ca.push(self.compile_and_load_call_arg(lam)?.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect_untyped")
                    .map_err(super::llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(
                        bv,
                        Some(inkwell::types::BasicTypeEnum::StructType(ret_ty)),
                    ),
                    None => Ok(TypedValue::Unit),
                }
            }
            _ => Err("Call target is not a function".to_string()),
        }
    }

    pub(super) fn compile_indirect_call_with_precompiled_args(
        &mut self,
        target: TypedValue<'ctx>,
        args: &[TypedValue<'ctx>],
        trailing: Option<TypedValue<'ctx>>,
    ) -> Result<TypedValue<'ctx>, String> {
        use inkwell::types::BasicMetadataTypeEnum;
        use inkwell::values::BasicMetadataValueEnum;
        match target {
            TypedValue::Fn(fn_ptr, fn_type) => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                for av in args {
                    ca.push(self.typed_value_to_bv(av).into());
                }
                if let Some(av) = &trailing {
                    ca.push(self.typed_value_to_bv(av).into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect")
                    .map_err(super::llvm_err)?;
                for av in args {
                    self.rc_free_intermediate(av)?;
                }
                if let Some(av) = &trailing {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Closure {
                fn_ptr,
                actual_fn_type,
                closure_ptr,
                closure_ty,
                alloca,
                capture_ptr_rc_mask,
            } => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                ca.push(closure_ptr.into());
                for av in args {
                    ca.push(self.typed_value_to_bv(av).into());
                }
                if let Some(av) = &trailing {
                    ca.push(self.typed_value_to_bv(av).into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(actual_fn_type, fn_ptr, &ca, "indirect_closure")
                    .map_err(super::llvm_err)?;
                for av in args {
                    self.rc_free_intermediate(av)?;
                }
                if let Some(av) = &trailing {
                    self.rc_free_intermediate(av)?;
                }
                if alloca.is_none() {
                    self.rc_inc(closure_ptr)?;
                    self.rc_dec_closure_captures(
                        closure_ptr,
                        closure_ty,
                        capture_ptr_rc_mask,
                        &[],
                    )?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, actual_fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Int(iv) => {
                let total_args = args.len() + trailing.as_ref().map_or(0, |_| 1);
                let param_tys: Vec<BasicMetadataTypeEnum<'ctx>> =
                    (0..total_args).map(|_| self.i64_ty().into()).collect();
                let ret_ty = self.fat_return_type;
                let fn_type = ret_ty.fn_type(&param_tys, false);
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_ptr = self
                    .builder
                    .build_int_to_ptr(iv, ptr_type, "fn_ptr_cast")
                    .map_err(super::llvm_err)?;
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                for av in args {
                    ca.push(self.typed_value_to_bv(av).into());
                }
                if let Some(av) = &trailing {
                    ca.push(self.typed_value_to_bv(av).into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect_untyped")
                    .map_err(super::llvm_err)?;
                for av in args {
                    self.rc_free_intermediate(av)?;
                }
                if let Some(av) = &trailing {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(
                        bv,
                        Some(inkwell::types::BasicTypeEnum::StructType(ret_ty)),
                    ),
                    None => Ok(TypedValue::Unit),
                }
            }
            _ => Err("Call target is not a function".to_string()),
        }
    }

    fn try_devirtualize_fn_call(
        &mut self,
        target: &TypedValue<'ctx>,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        if trailing.is_some() {
            return Ok(None);
        }
        if let Some(fn_name) = self.fn_ptr_to_module_name(target) {
            return Ok(Some(self.compile_direct_function_call_from_call_args(
                &fn_name, args, None,
            )?));
        }
        if args.len() == 1 {
            if let Some(result) = self.try_devirtualize_unary_lambda_call(target, args[0])? {
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    fn try_devirtualize_unary_lambda_call(
        &mut self,
        target: &TypedValue<'ctx>,
        arg: CallArg<'_>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let Some(dl) = self.try_direct_lambda(target.clone()) else {
            return Ok(None);
        };
        let av = self.compile_call_arg(arg)?;
        let iv = match av {
            TypedValue::Int(i) => i,
            _ => return Ok(None),
        };
        let cc = self.emit_direct_lambda_call(&dl, iv, "apply_direct")?;
        self.rc_free_intermediate(&av)?;
        if cc.is_int_value() {
            return Ok(Some(TypedValue::Int(cc.into_int_value())));
        }
        Ok(Some(self.unpack_fat_return(cc, None)?))
    }

    fn fn_ptr_to_module_name(&self, val: &TypedValue<'ctx>) -> Option<String> {
        let fn_ptr = match val {
            TypedValue::Fn(p, _) => *p,
            TypedValue::Closure { fn_ptr, .. } => *fn_ptr,
            _ => return None,
        };
        for f in self.module.get_functions() {
            if f.as_global_value().as_pointer_value() == fn_ptr {
                let name = f.get_name().to_str().ok()?;
                if name.starts_with(".lambda_") {
                    continue;
                }
                return Some(name.to_string());
            }
        }
        None
    }

    fn consteval_fib(n: u64) -> u64 {
        if n <= 1 {
            return n;
        }
        let mut a = 0u64;
        let mut b = 1u64;
        for _ in 2..=n {
            let c = a.wrapping_add(b);
            a = b;
            b = c;
        }
        b
    }

    fn consteval_fact(n: u64, acc: u64) -> u64 {
        if n <= 1 {
            return acc;
        }
        let mut n = n;
        let mut acc = acc;
        while n > 1 {
            acc = acc.wrapping_mul(n);
            n -= 1;
        }
        acc
    }

    fn resolve_direct_fn_name_hir(
        codegen: &CodeGen<'_>,
        hir: &action_frontend::hir::HirExpr,
    ) -> Option<String> {
        use action_frontend::hir::HirExprKind;
        match &hir.kind {
            HirExprKind::Ident(name) => {
                if codegen.module.get_function(name).is_some() {
                    Some(name.clone())
                } else {
                    None
                }
            }
            HirExprKind::FunctionRef(name) => {
                if codegen.module.get_function(name).is_some() {
                    Some(name.clone())
                } else {
                    let resolved = name.replace("::", "_").replace('.', "_");
                    if codegen.module.get_function(&resolved).is_some() {
                        Some(resolved)
                    } else {
                        None
                    }
                }
            }
            _ => None,
        }
    }
}
