// Submodule: builtins_ffi

use inkwell::values::BasicValue;
use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_to_cstring_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_call_arg(arg)?;
        self.builtin_to_cstring_value(val)
    }

    pub(crate) fn builtin_from_cstring_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_call_arg(arg)?;
        self.builtin_from_cstring_value(val)
    }

    pub(crate) fn builtin_is_null_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_call_arg(arg)?;
        self.builtin_is_null_value(val)
    }

    pub(crate) fn builtin_deref_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_call_arg(arg)?;
        self.builtin_deref_value(val)
    }

    pub(crate) fn builtin_to_cstring_value(
        &mut self,
        val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match val {
            TypedValue::Str(ptr) => {
                let str_val = self.load_string(ptr)?;
                let len = self
                    .builder
                    .build_extract_value(str_val, 0, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let data = self
                    .builder
                    .build_extract_value(str_val, 1, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                // allocate len + 1 bytes for null-terminated copy
                let size = self
                    .builder
                    .build_int_add(len, self.i64_ty().const_int(1, false), "cstr_size")
                    .map_err(llvm_err)?;
                let cstr = self.call_rt("malloc", &[size.into()])?;
                let cstr_ptr = cstr
                    .try_as_basic_value()
                    .basic()
                    .ok_or("malloc failed")?
                    .into_pointer_value();
                // memcpy the string data (dest, src, len)
                let _ = self
                    .builder
                    .build_memcpy(cstr_ptr, 1, data, 1, len)
                    .map_err(llvm_err)?;
                // null terminate
                let null_pos = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), cstr_ptr, &[len], "null_pos")
                }
                .map_err(llvm_err)?;
                self.builder
                    .build_store(null_pos, self.context.i8_type().const_int(0, false))
                    .map_err(llvm_err)?;
                Ok(TypedValue::CString(cstr_ptr))
            }
            _ => Err("toCString: argument must be a String".to_string()),
        }
    }

    pub(crate) fn builtin_from_cstring_value(
        &mut self,
        val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match val {
            TypedValue::CString(ptr) | TypedValue::Ptr(ptr) | TypedValue::FileHandle(ptr) => {
                if matches!(val, TypedValue::FileHandle(_)) {
                    return Err("fromCString: cannot convert FileHandle to string".to_string());
                }
                // Null check: return empty string for null pointers
                let null_ptr = self
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .const_zero();
                let is_null = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, ptr, null_ptr, "is_null")
                    .map_err(llvm_err)?;
                let is_null_bb = self.context.append_basic_block(
                    self.builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("no function")?,
                    "fcs_null",
                );
                let ok_bb = self.context.append_basic_block(
                    self.builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("no function")?,
                    "fcs_ok",
                );
                let merge_bb = self.context.append_basic_block(
                    self.builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("no function")?,
                    "fcs_merge",
                );
                let _ = self
                    .builder
                    .build_conditional_branch(is_null, is_null_bb, ok_bb);

                // Null path: return empty string ""
                self.builder.position_at_end(is_null_bb);
                let empty_str = self
                    .builder
                    .build_alloca(self.string_type, "empty")
                    .map_err(llvm_err)?;
                let empty_undef = self.string_type.get_undef();
                let e1 = self
                    .builder
                    .build_insert_value(empty_undef, self.i64_ty().const_int(0, false), 0, "e_len")
                    .map_err(llvm_err)?;
                // Allocate a zero byte for the empty string data
                let zero_byte = self
                    .builder
                    .build_alloca(self.context.i8_type(), "zero_byte")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(zero_byte, self.context.i8_type().const_int(0, false))
                    .map_err(llvm_err)?;
                let e2 = self
                    .builder
                    .build_insert_value(e1, zero_byte, 1, "e_ptr")
                    .map_err(llvm_err)?;
                self.builder.build_store(empty_str, e2).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);

                // OK path: strlen + allocate
                self.builder.position_at_end(ok_bb);
                let len_val = self.call_rt("strlen", &[ptr.into()])?;
                let len = len_val
                    .try_as_basic_value()
                    .basic()
                    .ok_or("strlen failed")?
                    .into_int_value();
                let str_struct = self.call_rt("action_string_create", &[ptr.into(), len.into()])?;
                let str_val = str_struct
                    .try_as_basic_value()
                    .basic()
                    .ok_or("string_create failed")?;
                let ok_alloca = self
                    .builder
                    .build_alloca(self.string_type, "from_cstr")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(ok_alloca, str_val)
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);

                // Merge with phi
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(
                        self.context.ptr_type(inkwell::AddressSpace::default()),
                        "fcs_phi",
                    )
                    .map_err(llvm_err)?;
                phi.add_incoming(&[
                    (&empty_str.as_basic_value_enum(), is_null_bb),
                    (&ok_alloca.as_basic_value_enum(), ok_bb),
                ]);
                let result_alloca = phi.as_basic_value().into_pointer_value();
                Ok(TypedValue::Str(result_alloca))
            }
            _ => Err("fromCString: argument must be a CString or Ptr".to_string()),
        }
    }

    pub(crate) fn builtin_is_null_value(
        &mut self,
        val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        if !self.in_unsafe {
            return Err("isNull can only be used inside an unsafe block".to_string());
        }
        match val {
            TypedValue::Ptr(p) | TypedValue::CString(p) | TypedValue::FileHandle(p) => {
                let null_ptr = self
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .const_zero();
                let is_null = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, p, null_ptr, "isNull")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_null))
            }
            _ => Err("isNull: argument must be a Ptr, CString, or FileHandle".to_string()),
        }
    }

    pub(crate) fn builtin_deref_value(
        &mut self,
        val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        if !self.in_unsafe {
            return Err("deref can only be used inside an unsafe block".to_string());
        }
        match val {
            TypedValue::Ptr(p) => {
                // Load as i64 (most common FFI use case)
                let loaded = self
                    .builder
                    .build_load(self.i64_ty(), p, "deref")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Int(loaded.into_int_value()))
            }
            _ => Err("deref: argument must be a Ptr".to_string()),
        }
    }

    pub(crate) fn builtin_http_request_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
        c: CallArg<'_>,
        d: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let method_val = self.compile_call_arg(a)?;
        let url_val = self.compile_call_arg(b)?;
        let headers_val = self.compile_call_arg(c)?;
        let body_val = self.compile_call_arg(d)?;
        self.builtin_http_request_values(method_val, url_val, headers_val, body_val)
    }

    pub(crate) fn builtin_http_request_values(
        &mut self,
        method_val: TypedValue<'ctx>,
        url_val: TypedValue<'ctx>,
        headers_val: TypedValue<'ctx>,
        body_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let method_cstr = self.builtin_to_cstring_value(method_val)?;
        let url_cstr = self.builtin_to_cstring_value(url_val)?;
        let headers_cstr = self.builtin_to_cstring_value(headers_val)?;
        let body_cstr = self.builtin_to_cstring_value(body_val)?;

        let method_ptr = match method_cstr {
            TypedValue::CString(p) => p,
            _ => return Err("httpRequest: method must be String".to_string()),
        };
        let url_ptr = match url_cstr {
            TypedValue::CString(p) => p,
            _ => return Err("httpRequest: url must be String".to_string()),
        };
        let headers_ptr = match headers_cstr {
            TypedValue::CString(p) => p,
            _ => return Err("httpRequest: headers must be String".to_string()),
        };
        let body_ptr = match body_cstr {
            TypedValue::CString(p) => p,
            _ => return Err("httpRequest: body must be String".to_string()),
        };

        // Use strlen to get body length (safe since we just null-terminated it)
        let body_len_val = self.call_rt("strlen", &[body_ptr.into()])?;
        let body_len = body_len_val
            .try_as_basic_value()
            .basic()
            .ok_or("strlen failed")?
            .into_int_value();

        // Call action_http_request(method, url, headers, body, body_len)
        let req_fn = self
            .module
            .get_function("action_http_request")
            .ok_or("action_http_request not found")?;
        let call_result = self
            .builder
            .build_call(
                req_fn,
                &[
                    method_ptr.into(),
                    url_ptr.into(),
                    headers_ptr.into(),
                    body_ptr.into(),
                    body_len.into(),
                ],
                "http_result",
            )
            .map_err(llvm_err)?;
        let result_ptr = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("call failed")?
            .into_pointer_value();

        // Free temp CStrings
        let free_fn = self
            .module
            .get_function("free")
            .ok_or("free not found in module")?;
        for ptr in &[method_ptr, url_ptr, headers_ptr, body_ptr] {
            let _ = self.builder.build_call(free_fn, &[(*ptr).into()], "");
        }

        // Parse "STATUS\nBODY" -> HttpResponse { status: Int, body: String }
        let res_len_val = self.call_rt("strlen", &[result_ptr.into()])?;
        let res_len = res_len_val
            .try_as_basic_value()
            .basic()
            .ok_or("strlen failed")?
            .into_int_value();

        let strchr_fn = self
            .module
            .get_function("strchr")
            .ok_or("strchr not found")?;
        let nl_byte = self.context.i8_type().const_int(b'\n' as u64, false);
        let nl_call = self
            .builder
            .build_call(strchr_fn, &[result_ptr.into(), nl_byte.into()], "nl")
            .map_err(llvm_err)?;
        let nl_ptr = nl_call
            .try_as_basic_value()
            .basic()
            .ok_or("strchr failed")?
            .into_pointer_value();

        let null_ptr = self.ptr_ty().const_null();
        let has_nl = self
            .builder
            .build_int_compare(IntPredicate::NE, nl_ptr, null_ptr, "has_nl")
            .map_err(llvm_err)?;

        let status_line_len_raw = self
            .builder
            .build_ptr_diff(
                self.context.i8_type(),
                nl_ptr,
                result_ptr,
                "status_len",
            )
            .map_err(llvm_err)?;
        let status_line_len = self
            .builder
            .build_select(has_nl, status_line_len_raw, res_len, "status_line_len")
            .map_err(llvm_err)?
            .into_int_value();

        let status_sv = self.call_rt(
            "action_string_create",
            &[result_ptr.into(), status_line_len.into()],
        )?;
        let status_st = status_sv
            .try_as_basic_value()
            .basic()
            .ok_or("status string failed")?
            .into_struct_value();
        let parse_res = self.call_rt("action_parse_int", &[status_st.into()])?;
        let parse_st = parse_res
            .try_as_basic_value()
            .basic()
            .ok_or("parse failed")?
            .into_struct_value();
        let status_val = self
            .builder
            .build_extract_value(parse_st, 0, "status")
            .map_err(llvm_err)?
            .into_int_value();

        let one = self.i64_ty().const_int(1, false);
        let body_start_gep = unsafe {
            self.builder
                .build_gep(self.context.i8_type(), nl_ptr, &[one], "body_start")
                .map_err(llvm_err)?
        };
        let body_start = self
            .builder
            .build_select(has_nl, body_start_gep, result_ptr, "body_ptr")
            .map_err(llvm_err)?;
        let skip = self
            .builder
            .build_int_add(status_line_len, one, "skip")
            .map_err(llvm_err)?;
        let body_len_raw = self
            .builder
            .build_int_sub(res_len, skip, "body_len")
            .map_err(llvm_err)?;
        let body_len = self
            .builder
            .build_select(has_nl, body_len_raw, res_len, "body_len_safe")
            .map_err(llvm_err)?;

        let body_sv = self.call_rt(
            "action_string_create",
            &[body_start.into(), body_len.into()],
        )?;
        let body_st = body_sv
            .try_as_basic_value()
            .basic()
            .ok_or("body string failed")?
            .into_struct_value();
        let body_alloca = self
            .builder
            .build_alloca(self.string_type, "http_body")
            .map_err(llvm_err)?;
        self.builder
            .build_store(body_alloca, body_st)
            .map_err(llvm_err)?;

        let http_resp_ty = self
            .type_layout
            .named_structs
            .get("HttpResponse")
            .copied()
            .ok_or("HttpResponse type not registered")?;
        let resp_undef = http_resp_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(resp_undef, status_val, 0, "hr_status")
            .map_err(llvm_err)?;
        let loaded_body = self.load_string(body_alloca)?;
        let r2 = self
            .builder
            .build_insert_value(r1, loaded_body, 1, "hr_body")
            .map_err(llvm_err)?;
        let resp_alloca = self
            .builder
            .build_alloca(http_resp_ty, "http_response")
            .map_err(llvm_err)?;
        self.builder
            .build_store(resp_alloca, r2)
            .map_err(llvm_err)?;

        let http_free_fn = self
            .module
            .get_function("action_http_free")
            .ok_or("action_http_free not found")?;
        let _ = self
            .builder
            .build_call(http_free_fn, &[result_ptr.into()], "");

        Ok(TypedValue::Struct(resp_alloca, http_resp_ty))
    }
}
