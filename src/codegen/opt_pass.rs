// LLVM module-level optimization passes for AOT emission (not MCJIT — JIT applies its own opt).

use super::CodeGen;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{InitializationConfig, Target, TargetMachine};

impl<'ctx> CodeGen<'ctx> {
    /// Run IR passes before AOT object emission when opt >= 1.
    pub fn run_aot_module_passes(&self) -> Result<(), String> {
        if self.opt_level == 0 {
            return Ok(());
        }

        Target::initialize_x86(&InitializationConfig::default());
        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple).map_err(|e| e.to_string())?;
        let cpu = TargetMachine::get_host_cpu_name().to_string();
        let features = TargetMachine::get_host_cpu_features().to_string();
        let opt = match self.opt_level {
            0 => inkwell::OptimizationLevel::None,
            1 => inkwell::OptimizationLevel::Less,
            2 => inkwell::OptimizationLevel::Default,
            _ => inkwell::OptimizationLevel::Aggressive,
        };
        let reloc = super::jit::aot_reloc_mode(&target_triple);
        let target_machine = target
            .create_target_machine(
                &target_triple,
                &cpu,
                &features,
                opt,
                reloc,
                inkwell::targets::CodeModel::Default,
            )
            .ok_or_else(|| "Failed to create target machine for IR passes".to_string())?;

        let passes = match self.opt_level {
            1 => "instcombine,reassociate,simplifycfg,mem2reg",
            2 => "default<O2>",
            _ => "default<O3>",
        };
        self.module
            .run_passes(passes, &target_machine, PassBuilderOptions::create())
            .map_err(|e| e.to_string())
    }

    /// Attach LLVM attributes to hot runtime helpers (helps IPO / inlining in AOT -O2).
    pub(super) fn apply_runtime_fn_attrs(&self) {
        use inkwell::attributes::{Attribute, AttributeLoc};

        let ctx = self.context;
        let nounwind_id = Attribute::get_named_enum_kind_id("nounwind");
        let nounwind = ctx.create_enum_attribute(nounwind_id, 0);

        let nounwind_fns = [
            "action_list_get",
            "action_list_get_cached",
            "action_list_len",
            "action_list_is_empty",
            "action_list_find",
            "action_list_push",
            "action_list_insert",
            "action_list_remove",
            "action_list_concat",
            "action_map_get",
            "action_map_len",
            "action_map_contains_key",
            "action_rc_inc",
            "action_rc_dec",
        ];
        for name in nounwind_fns {
            if let Some(f) = self.module.get_function(name) {
                f.add_attribute(AttributeLoc::Function, nounwind);
            }
        }
    }
}
