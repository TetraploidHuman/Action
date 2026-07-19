use crate::ast::*;
use std::collections::HashSet;

use super::registry::TypeRegistry;

pub fn check_when_exhaustive(registry: &TypeRegistry, arms: &[WhenArm]) -> Result<(), String> {
    let mut covered: HashSet<String> = HashSet::new();
    let mut enum_name: Option<String> = None;
    let mut has_wildcard = false;

    for arm in arms {
        collect_pattern_coverage(
            registry,
            &arm.pattern,
            &mut covered,
            &mut enum_name,
            &mut has_wildcard,
        );
    }

    if has_wildcard || enum_name.is_none() {
        return Ok(());
    }

    let info = registry
        .enums
        .get(enum_name.as_ref().unwrap())
        .ok_or_else(|| format!("Unknown enum type: {}", enum_name.as_ref().unwrap()))?;

    let mut missing: Vec<&str> = Vec::new();
    for v in &info.variants {
        if !covered.contains(&v.name) {
            missing.push(&v.name);
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        let msg = missing
            .iter()
            .map(|n| format!("'{}'", n))
            .collect::<Vec<_>>()
            .join(", ");
        Err(format!(
            "Non-exhaustive when: enum '{}' is missing variant(s): {}. Add them or add an else branch.",
            info.name, msg
        ))
    }
}

fn collect_pattern_coverage(
    registry: &TypeRegistry,
    pattern: &Pattern,
    covered: &mut HashSet<String>,
    enum_name: &mut Option<String>,
    has_wildcard: &mut bool,
) {
    match pattern {
        Pattern::Wildcard | Pattern::Variable(_) => {
            *has_wildcard = true;
        }
        Pattern::Constructor {
            name,
            args,
            named_fields,
        } => {
            if let Some(en) = registry.variant_to_enum.get(name.as_str()) {
                if enum_name.is_none() {
                    *enum_name = Some(en.clone());
                }
                // Only known variants count as coverage (M66: unknowns → E014 elsewhere).
                covered.insert(name.clone());
            }
            for sub in args {
                collect_pattern_coverage(registry, sub, covered, enum_name, has_wildcard);
            }
            for (_, sub) in named_fields {
                collect_pattern_coverage(registry, sub, covered, enum_name, has_wildcard);
            }
        }
        Pattern::Or(patterns) => {
            for p in patterns {
                collect_pattern_coverage(registry, p, covered, enum_name, has_wildcard);
            }
        }
        Pattern::Tuple(patterns) => {
            for p in patterns {
                collect_pattern_coverage(registry, p, covered, enum_name, has_wildcard);
            }
        }
        _ => {} // Literal, Range, IsType — not relevant for enum exhaustiveness
    }
}
