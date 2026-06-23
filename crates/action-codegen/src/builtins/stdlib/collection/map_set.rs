// Submodule: builtins_stdlib_collection/map_set

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn collection_dispatch_map_set(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
            "mapKeys" => {
                if args.len() != 1 {
                    return Err("mapKeys expects 1 argument (map)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_keys", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapKeys failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "keys")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("mapKeys: argument must be a map".to_string()),
                }
            }
            "mapValues" => {
                if args.len() != 1 {
                    return Err("mapValues expects 1 argument (map)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_values", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapValues failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "values")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("mapValues: argument must be a map".to_string()),
                }
            }
            "mapEntries" => {
                if args.len() != 1 {
                    return Err("mapEntries expects 1 argument (map)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_entries", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapEntries failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "entries")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("mapEntries: argument must be a map".to_string()),
                }
            }
            "mapUnion" => {
                if args.len() != 2 {
                    return Err("map.union expects 2 arguments (map1, map2)".to_string());
                }
                let v1 = self.compile_call_arg(args[0])?;
                let v2 = self.compile_call_arg(args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Map(mp1), TypedValue::Map(mp2)) => {
                        let mv1 = self.load_list(*mp1)?;
                        let mv2 = self.load_list(*mp2)?;
                        let cc = self.call_rt("action_map_union", &[mv1.into(), mv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("map.union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "mapUnion")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::Map(alloca)))
                    }
                    _ => Err("map.union: arguments must be maps".to_string()),
                }
            }
            "setUnion" => {
                if args.len() != 2 {
                    return Err("set.union expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_call_arg(args[0])?;
                let v2 = self.compile_call_arg(args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc = self.call_rt("action_set_union", &[sv1.into(), sv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("set.union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "union")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::Set(alloca)))
                    }
                    _ => Err("set.union: arguments must be sets".to_string()),
                }
            }
            "setIntersection" => {
                if args.len() != 2 {
                    return Err("set.intersection expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_call_arg(args[0])?;
                let v2 = self.compile_call_arg(args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc =
                            self.call_rt("action_set_intersection", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.intersection failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "intersection")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::Set(alloca)))
                    }
                    _ => Err("set.intersection: arguments must be sets".to_string()),
                }
            }
            "setDifference" => {
                if args.len() != 2 {
                    return Err("set.difference expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_call_arg(args[0])?;
                let v2 = self.compile_call_arg(args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc =
                            self.call_rt("action_set_difference", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.difference failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "difference")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::Set(alloca)))
                    }
                    _ => Err("set.difference: arguments must be sets".to_string()),
                }
            }
            "setIsSubset" => {
                if args.len() != 2 {
                    return Err("set.isSubset expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_call_arg(args[0])?;
                let v2 = self.compile_call_arg(args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc = self.call_rt("action_set_is_subset", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.isSubset failed")?
                            .into_int_value();
                        Ok(Some(TypedValue::Bool(result)))
                    }
                    _ => Err("set.isSubset: arguments must be sets".to_string()),
                }
            }
            _ => Ok(None),
        }
    }
}
