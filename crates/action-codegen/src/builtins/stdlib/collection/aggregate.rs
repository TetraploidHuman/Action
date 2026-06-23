// Submodule: builtins_stdlib_collection/aggregate

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};
use action_frontend::ast::Type;
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn collection_dispatch_aggregate(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
            "randShuffle" => {
                if args.len() != 1 {
                    return Err("randShuffle expects 1 argument (list)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_rand_shuffle", &[lv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("randShuffle failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "shuffled")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("randShuffle: argument must be a list".to_string()),
                }
            }
            "sorted" => {
                if args.len() != 1 {
                    return Err("sorted expects 1 argument (list)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let is_float = match args[0] {
                            CallArg::Hir(h) => matches!(
                                &h.ty,
                                Type::Generic(base, params)
                                    if matches!(base.as_ref(), Type::Named(n) if n == "List")
                                        && params.first().map(|t| matches!(t, Type::Named(n) if n == "Float")).unwrap_or(false)
                            ),
                        };
                        let cc = if is_float {
                            let cmp_fn = self
                                .module
                                .get_function("action_float_bits_gt")
                                .ok_or("action_float_bits_gt not found")?;
                            self.call_rt(
                                "action_list_sorted_by",
                                &[
                                    lv.into(),
                                    cmp_fn.as_global_value().as_pointer_value().into(),
                                ],
                            )?
                        } else {
                            self.call_rt("action_list_sorted", &[lv.into()])?
                        };
                        let result = cc.try_as_basic_value().basic().ok_or("sorted failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "sorted")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("sorted: argument must be a list".to_string()),
                }
            }
            "sum" => {
                if args.len() != 1 {
                    return Err("sum expects 1 argument (list)".to_string());
                }
                let list_val = self.compile_call_arg(args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("sum: argument must be a list".to_string()),
                };
                let list = self.load_list(list_ptr)?;
                let result = self.list_sum_from_loaded(list)?;
                Ok(Some(TypedValue::Int(result)))
            }
            "product" => {
                if args.len() != 1 {
                    return Err("product expects 1 argument (list)".to_string());
                }
                let list_val = self.compile_call_arg(args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("product: argument must be a list".to_string()),
                };
                let list = self.load_list(list_ptr)?;
                let len = self.list_len_val(list)?;
                let data = self.list_data_ptr(list)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                let prod_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "prod")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(prod_a, self.i64_ty().const_int(1, false))
                    .map_err(llvm_err)?;
                let i_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "i")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(i_a, self.i64_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                let hdr = self.context.append_basic_block(current, "prod_hdr");
                let bdy = self.context.append_basic_block(current, "prod_bdy");
                let ext = self.context.append_basic_block(current, "prod_ext");
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(hdr);
                let iv = self
                    .builder
                    .build_load(self.i64_ty(), i_a, "iv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, iv, len, "cond")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(cond, bdy, ext);
                self.builder.position_at_end(bdy);
                let ep = unsafe {
                    self.builder
                        .build_gep(self.string_type, data, &[iv], "ep")
                        .map_err(llvm_err)
                }?;
                let ev = self
                    .builder
                    .build_load(self.string_type, ep, "ev")
                    .map_err(llvm_err)?;
                let etag = self
                    .builder
                    .build_extract_value(ev.into_struct_value(), 0, "etag")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur = self
                    .builder
                    .build_load(self.i64_ty(), prod_a, "cur")
                    .map_err(llvm_err)?
                    .into_int_value();
                let new_prod = self
                    .builder
                    .build_int_mul(cur, etag, "new_prod")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(prod_a, new_prod)
                    .map_err(llvm_err)?;
                let ni = self
                    .builder
                    .build_int_add(iv, self.i64_ty().const_int(1, false), "ni")
                    .map_err(llvm_err)?;
                self.builder.build_store(i_a, ni).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(ext);
                let result = self
                    .builder
                    .build_load(self.i64_ty(), prod_a, "result")
                    .map_err(llvm_err)?;
                Ok(Some(TypedValue::Int(result.into_int_value())))
            }
            "digits" => {
                // digits(n) -> List<Int>: decimal digits of abs(n), MSD first. 0 -> [0].
                if args.len() != 1 {
                    return Err("digits expects 1 argument (int)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                let n = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("digits: argument must be an int".to_string()),
                };
                let ten = self.i64_ty().const_int(10, false);
                let zero = self.i64_ty().const_int(0, false);
                let one = self.i64_ty().const_int(1, false);
                // abs_n = n < 0 ? -n : n
                let neg = self.builder.build_int_neg(n, "neg").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, n, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_n = self
                    .builder
                    .build_select(is_neg, neg, n, "abs_n")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_zero = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, n, zero, "is0")
                    .map_err(llvm_err)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                // Count digits via repeated division
                let dc_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "dc")
                    .map_err(llvm_err)?;
                self.builder.build_store(dc_a, zero).map_err(llvm_err)?;
                let tmp_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "tmp")
                    .map_err(llvm_err)?;
                self.builder.build_store(tmp_a, abs_n).map_err(llvm_err)?;
                let cnt_hdr = self.context.append_basic_block(current, "dc_hdr");
                let cnt_bdy = self.context.append_basic_block(current, "dc_bdy");
                let cnt_ext = self.context.append_basic_block(current, "dc_ext");
                let _ = self.builder.build_unconditional_branch(cnt_hdr);
                self.builder.position_at_end(cnt_hdr);
                let tv = self
                    .builder
                    .build_load(self.i64_ty(), tmp_a, "tv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let gt0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGT, tv, zero, "gt0")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(gt0, cnt_bdy, cnt_ext);
                self.builder.position_at_end(cnt_bdy);
                let dv = self
                    .builder
                    .build_load(self.i64_ty(), dc_a, "dv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let nd = self
                    .builder
                    .build_int_add(dv, one, "nd")
                    .map_err(llvm_err)?;
                self.builder.build_store(dc_a, nd).map_err(llvm_err)?;
                let nt = self
                    .builder
                    .build_int_signed_div(tv, ten, "nt")
                    .map_err(llvm_err)?;
                self.builder.build_store(tmp_a, nt).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(cnt_hdr);
                self.builder.position_at_end(cnt_ext);
                let ndigits = self
                    .builder
                    .build_load(self.i64_ty(), dc_a, "nd")
                    .map_err(llvm_err)?
                    .into_int_value();
                // 0 -> 1 digit
                let final_dc = self
                    .builder
                    .build_select(is_zero, one, ndigits, "fdc")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Create result list with capacity = final_dc
                let cc = self.call_rt("action_list_create", &[final_dc.into()])?;
                let res_bv = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("list_create failed")?;
                let res_a = self
                    .builder
                    .build_alloca(self.list_type, "digits_res")
                    .map_err(llvm_err)?;
                self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
                // Compute 10^(ndigits-1) iteratively
                let pow_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "pow10")
                    .map_err(llvm_err)?;
                self.builder.build_store(pow_a, one).map_err(llvm_err)?;
                let pi_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "pi")
                    .map_err(llvm_err)?;
                self.builder.build_store(pi_a, one).map_err(llvm_err)?;
                let pow_hdr = self.context.append_basic_block(current, "pow_hdr");
                let pow_bdy = self.context.append_basic_block(current, "pow_bdy");
                let pow_ext = self.context.append_basic_block(current, "pow_ext");
                let _ = self.builder.build_unconditional_branch(pow_hdr);
                self.builder.position_at_end(pow_hdr);
                let piv = self
                    .builder
                    .build_load(self.i64_ty(), pi_a, "piv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let plt = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, piv, final_dc, "plt")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(plt, pow_bdy, pow_ext);
                self.builder.position_at_end(pow_bdy);
                let pv = self
                    .builder
                    .build_load(self.i64_ty(), pow_a, "pv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let npv = self
                    .builder
                    .build_int_mul(pv, ten, "npv")
                    .map_err(llvm_err)?;
                self.builder.build_store(pow_a, npv).map_err(llvm_err)?;
                let npi = self
                    .builder
                    .build_int_add(piv, one, "npi")
                    .map_err(llvm_err)?;
                self.builder.build_store(pi_a, npi).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(pow_hdr);
                self.builder.position_at_end(pow_ext);
                let pow10 = self
                    .builder
                    .build_load(self.i64_ty(), pow_a, "pow10")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Extract digits MSD-first: for i in 0..ndigits { d = (abs_n / pow10) % 10; push; pow10 /= 10 }
                self.builder.build_store(tmp_a, abs_n).map_err(llvm_err)?;
                let di_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "di")
                    .map_err(llvm_err)?;
                self.builder.build_store(di_a, zero).map_err(llvm_err)?;
                let p10_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "p10")
                    .map_err(llvm_err)?;
                self.builder.build_store(p10_a, pow10).map_err(llvm_err)?;
                let fill_hdr = self.context.append_basic_block(current, "fill_hdr");
                let fill_bdy = self.context.append_basic_block(current, "fill_bdy");
                let fill_ext = self.context.append_basic_block(current, "fill_ext");
                let _ = self.builder.build_unconditional_branch(fill_hdr);
                self.builder.position_at_end(fill_hdr);
                let div = self
                    .builder
                    .build_load(self.i64_ty(), di_a, "div")
                    .map_err(llvm_err)?
                    .into_int_value();
                let flt = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, div, final_dc, "flt")
                    .map_err(llvm_err)?;
                let _ = self
                    .builder
                    .build_conditional_branch(flt, fill_bdy, fill_ext);
                self.builder.position_at_end(fill_bdy);
                let cur_pow = self
                    .builder
                    .build_load(self.i64_ty(), p10_a, "cur_pow")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur_n = self
                    .builder
                    .build_load(self.i64_ty(), tmp_a, "cur_n")
                    .map_err(llvm_err)?
                    .into_int_value();
                let q = self
                    .builder
                    .build_int_signed_div(cur_n, cur_pow, "q")
                    .map_err(llvm_err)?;
                let digit = self
                    .builder
                    .build_int_signed_rem(q, ten, "digit")
                    .map_err(llvm_err)?;
                // Build fat struct {digit, null} and push
                let undef = self.string_type.get_undef();
                let d1 = self
                    .builder
                    .build_insert_value(undef, digit, 0, "d1")
                    .map_err(llvm_err)?;
                let d2 = self
                    .builder
                    .build_insert_value(d1, self.ptr_ty().const_zero(), 1, "d2")
                    .map_err(llvm_err)?;
                let rl = self
                    .builder
                    .build_load(self.list_type, res_a, "rl")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let rp = self.call_rt(
                    "action_list_push",
                    &[rl.into(), d2.as_basic_value_enum().into()],
                )?;
                self.builder
                    .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
                    .map_err(llvm_err)?;
                // Advance: i++, pow10 /= 10
                let ndi = self
                    .builder
                    .build_int_add(div, one, "ndi")
                    .map_err(llvm_err)?;
                self.builder.build_store(di_a, ndi).map_err(llvm_err)?;
                let np10 = self
                    .builder
                    .build_int_signed_div(cur_pow, ten, "np10")
                    .map_err(llvm_err)?;
                self.builder.build_store(p10_a, np10).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(fill_hdr);
                self.builder.position_at_end(fill_ext);
                Ok(Some(TypedValue::List(res_a)))
            }
            _ => Ok(None),
        }
    }
}
