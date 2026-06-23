// Submodule: runtime_decl

use super::{llvm_err, CodeGen};
use action_frontend::typecheck::TypeRegistry;
use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::Module;
use std::sync::OnceLock;

// Validated at build time by build.rs (not linked at runtime — see define_runtime).
include!(concat!(env!("OUT_DIR"), "/runtime_bc_embed.rs"));

/// Process-wide cache of LLVM bitcode for the runtime module (List/Map/String/RC etc.).
/// Populated on the first `define_runtime` call; subsequent compilations link this in
/// instead of regenerating thousands of lines of IR.
static RUNTIME_BITCODE: OnceLock<Vec<u8>> = OnceLock::new();

fn link_runtime_bitcode_into<'ctx>(
    module: &Module<'ctx>,
    context: &'ctx Context,
    bitcode: &[u8],
) -> Result<(), String> {
    let buffer = MemoryBuffer::create_from_memory_range_copy(bitcode, "action_runtime.bc");
    let runtime_mod =
        Module::parse_bitcode_from_buffer(&buffer, context).map_err(|e| e.to_string())?;
    module
        .link_in_module(runtime_mod)
        .map_err(|e| e.to_string())
}

impl<'ctx> CodeGen<'ctx> {
    /// Create a global string constant in the LLVM module.
    pub(super) fn make_global_str(
        &self,
        name: &str,
        content: &[u8],
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        let i8 = self.context.i8_type();
        let arr_ty = i8.array_type(content.len() as u32);
        let global = self.add_module_global(arr_ty, name)?;
        let arr = self.context.const_string(content, false);
        global.set_initializer(&arr);
        Ok(global.as_pointer_value())
    }

    pub(super) fn define_runtime(&self) -> Result<(), String> {
        // Build-time embed is validated only; linking duplicates LLVM types from CodeGen::new().
        let _ = EMBEDDED_RUNTIME_BC;
        if let Some(bitcode) = RUNTIME_BITCODE.get() {
            return link_runtime_bitcode_into(&self.module, self.context, bitcode);
        }

        self.define_runtime_generate()?;

        if RUNTIME_BITCODE.get().is_none() {
            let mem = self.module.write_bitcode_to_memory();
            let _ = RUNTIME_BITCODE.set(mem.as_slice().to_vec());
        }
        Ok(())
    }

    /// Emit LLVM bitcode for the Action runtime (build.rs / runtime-bc-emit).
    pub fn generate_runtime_bitcode() -> Result<Vec<u8>, String> {
        let context = Context::create();
        let registry = TypeRegistry::default();
        let cg = CodeGen::new(&context, "action_runtime", registry, None);
        cg.define_runtime_generate()?;
        cg.module
            .verify()
            .map_err(|e| format!("runtime bitcode verify failed: {e}"))?;
        Ok(cg.module.write_bitcode_to_memory().as_slice().to_vec())
    }

    fn define_runtime_generate(&self) -> Result<(), String> {
        self.declare_c_runtime_externs()?;
        self.define_runtime_groups()?;
        self.apply_runtime_fn_attrs();
        Ok(())
    }
}

// ---- Submodules ----
mod declare_groups;
mod define_file_parse;
mod define_lazy_list;
mod define_map;
mod define_math_ms;
mod define_misc;
mod define_print;
mod define_rand;
mod define_str_adv;
mod define_str_basic;
mod define_str_core;
mod define_str_extra;
mod define_str_util;
mod extern_decls;
mod hash_table;
mod list;
