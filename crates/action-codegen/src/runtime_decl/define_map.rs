// Submodule: runtime_decl/define_map
//
// Thin wrappers: action_map_* delegates to action_ht_* flat-table runtime.

use super::{llvm_err, CodeGen};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_map(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let list_ty = self.list_type;

        let ht_create = self.module.get_function("action_ht_create").unwrap();
        let ht_insert = self.module.get_function("action_ht_insert").unwrap();
        let ht_get = self.module.get_function("action_ht_get").unwrap();
        let ht_contains = self.module.get_function("action_ht_contains").unwrap();
        let ht_remove = self.module.get_function("action_ht_remove").unwrap();

        // action_map_create(cap) -> action_ht_create(cap)
        let f = self.module.add_function(
            "action_map_create",
            list_ty.fn_type(&[i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        self.builder.position_at_end(entry);
        let cap = f.get_first_param().unwrap();
        let r = self
            .builder
            .build_call(ht_create, &[cap.into()], "r")
            .map_err(llvm_err)?;
        self.builder
            .build_return(Some(&r.try_as_basic_value().unwrap_basic()))
            .map_err(llvm_err)?;

        // action_map_insert(map, key, val) -> action_ht_insert(...)
        let f = self.module.add_function(
            "action_map_insert",
            list_ty.fn_type(&[list_ty.into(), str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        self.builder.position_at_end(entry);
        let map = f.get_first_param().unwrap();
        let key = f.get_nth_param(1).unwrap();
        let val = f.get_nth_param(2).unwrap();
        let r = self
            .builder
            .build_call(ht_insert, &[map.into(), key.into(), val.into()], "r")
            .map_err(llvm_err)?;
        self.builder
            .build_return(Some(&r.try_as_basic_value().unwrap_basic()))
            .map_err(llvm_err)?;

        // action_map_get(map, key) -> action_ht_get(...)
        let f = self.module.add_function(
            "action_map_get",
            str_ty.fn_type(&[list_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        self.builder.position_at_end(entry);
        let map = f.get_first_param().unwrap();
        let key = f.get_nth_param(1).unwrap();
        let r = self
            .builder
            .build_call(ht_get, &[map.into(), key.into()], "r")
            .map_err(llvm_err)?;
        self.builder
            .build_return(Some(&r.try_as_basic_value().unwrap_basic()))
            .map_err(llvm_err)?;

        // action_map_contains(map, key) -> action_ht_contains(...)
        let f = self.module.add_function(
            "action_map_contains",
            b1.fn_type(&[list_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        self.builder.position_at_end(entry);
        let map = f.get_first_param().unwrap();
        let key = f.get_nth_param(1).unwrap();
        let r = self
            .builder
            .build_call(ht_contains, &[map.into(), key.into()], "r")
            .map_err(llvm_err)?;
        self.builder
            .build_return(Some(&r.try_as_basic_value().unwrap_basic()))
            .map_err(llvm_err)?;

        // action_map_remove(map, key) -> action_ht_remove(...)
        let f = self.module.add_function(
            "action_map_remove",
            list_ty.fn_type(&[list_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        self.builder.position_at_end(entry);
        let map = f.get_first_param().unwrap();
        let key = f.get_nth_param(1).unwrap();
        let r = self
            .builder
            .build_call(ht_remove, &[map.into(), key.into()], "r")
            .map_err(llvm_err)?;
        self.builder
            .build_return(Some(&r.try_as_basic_value().unwrap_basic()))
            .map_err(llvm_err)?;

        // action_set_from_list(list) -> action_ht_from_list(list)
        let ht_from_list = self.module.get_function("action_ht_from_list").unwrap();
        let f = self.module.add_function(
            "action_set_from_list",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        self.builder.position_at_end(entry);
        let lst = f.get_first_param().unwrap();
        let r = self
            .builder
            .build_call(ht_from_list, &[lst.into()], "r")
            .map_err(llvm_err)?;
        self.builder
            .build_return(Some(&r.try_as_basic_value().unwrap_basic()))
            .map_err(llvm_err)?;

        Ok(())
    }
}
