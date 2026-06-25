//! Shared type operations used by the type checker and codegen.

use crate::ast::Type;
use std::collections::HashMap;

/// Mangle a function name by appending parameter types: `add(Int, Float)` → `add_Int_Float`.
pub fn mangle_name(name: &str, param_types: &[Type]) -> String {
    if param_types.is_empty() {
        return name.to_string();
    }
    let parts: Vec<String> = param_types.iter().map(|t| format!("{}", t)).collect();
    format!("{}_{}", name, parts.join("_"))
}

/// Normalize type aliases to canonical names.
pub fn normalize_type_name(name: &str) -> &str {
    match name {
        "Str" => "String",
        "Double" => "Float",
        other => other,
    }
}

/// `Ptr[T]` surface syntax is `Type::Generic(Named("Ptr"), [T])`; registry uses `Type::Ptr(T)`.
fn ptr_pointee(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Ptr(inner) => Some(inner.as_ref()),
        Type::Generic(base, args)
            if matches!(base.as_ref(), Type::Named(n) if n == "Ptr")
                && args.len() == 1 =>
        {
            Some(&args[0])
        }
        _ => None,
    }
}

/// Check if two types are structurally compatible (no type-var binding).
pub fn types_compatible(declared: &Type, inferred: &Type) -> bool {
    match (declared, inferred) {
        (Type::Unit, Type::Unit) => true,
        (Type::Named(a), Type::Named(b)) => normalize_type_name(a) == normalize_type_name(b),
        (Type::Struct(fa), Type::Struct(fb)) => {
            fa.len() == fb.len()
                && fa
                    .iter()
                    .zip(fb.iter())
                    .all(|((na, ta), (nb, tb))| na == nb && types_compatible(ta, tb))
        }
        (Type::Map(ka, va), Type::Map(kb, vb)) => {
            types_compatible(ka, kb) && types_compatible(va, vb)
        }
        (Type::Set(ea), Type::Set(eb)) => types_compatible(ea, eb),
        (Type::Task(ta), Type::Task(tb)) => types_compatible(ta, tb),
        (Type::Stream(sa), Type::Stream(sb)) => types_compatible(sa, sb),
        (Type::LazyList(la), Type::LazyList(lb)) => types_compatible(la, lb),
        (Type::Ptr(pa), Type::Ptr(pb)) => types_compatible(pa, pb),
        (Type::CString, Type::CString) | (Type::FileHandle, Type::FileHandle) => true,
        (Type::CString, Type::Named(n)) | (Type::Named(n), Type::CString) if n == "CString" => true,
        (Type::FileHandle, Type::Named(n)) | (Type::Named(n), Type::FileHandle)
            if n == "FileHandle" =>
        {
            true
        }
        (Type::Generic(ba, ta), Type::Generic(bb, tb)) => {
            ta.len() == tb.len()
                && types_compatible(ba, bb)
                && ta
                    .iter()
                    .zip(tb.iter())
                    .all(|(a, b)| types_compatible(a, b))
        }
        (Type::Function(pa, ra), Type::Function(pb, rb)) => {
            pa.len() == pb.len()
                && pa
                    .iter()
                    .zip(pb.iter())
                    .all(|(a, b)| types_compatible(a, b))
                && types_compatible(ra, rb)
        }
        (Type::Nullable(a), Type::Nullable(b)) => {
            if matches!(b.as_ref(), Type::Named(n) if n == "Nothing") {
                true
            } else {
                types_compatible(a, b)
            }
        }
        (Type::TypeVar(_), _) | (_, Type::TypeVar(_)) => true,
        (Type::InferVar(_), _) | (_, Type::InferVar(_)) => true,
        (_, Type::Nullable(inner)) if matches!(inner.as_ref(), Type::Named(n) if n == "Nothing") => {
            matches!(declared, Type::Nullable(_))
        }
        (Type::Nullable(inner), inferred) if !matches!(inferred, Type::Nullable(_)) => {
            types_compatible(inner, inferred)
        }
        (_, Type::Nullable(_)) => false,
        _ => match (ptr_pointee(declared), ptr_pointee(inferred)) {
            (Some(a), Some(b)) => types_compatible(a, b),
            _ => false,
        },
    }
}

/// Unify an expected type (may contain type variables) with an actual concrete type.
pub fn unify(
    expected: &Type,
    actual: &Type,
    type_map: &mut HashMap<String, Type>,
) -> Result<(), String> {
    match (expected, actual) {
        (Type::TypeVar(name), _) => {
            if let Some(existing) = type_map.get(name) {
                if types_compatible(existing, actual) {
                    Ok(())
                } else {
                    Err(format!(
                        "Conflicting type inference for '{}': {} vs {}",
                        name, existing, actual
                    ))
                }
            } else {
                type_map.insert(name.clone(), actual.clone());
                Ok(())
            }
        }
        (Type::InferVar(_), _) | (_, Type::InferVar(_)) => {
            // Generic call-site inference uses source TypeVar only; InferVar resolved earlier
            Ok(())
        }
        (Type::Named(a), Type::Named(b)) => {
            if normalize_type_name(a) == normalize_type_name(b) {
                Ok(())
            } else {
                Err(format!("Type mismatch: {} vs {}", a, b))
            }
        }
        (Type::Generic(ba, ta), Type::Generic(bb, tb)) => {
            if ta.len() != tb.len() {
                return Err("Generic argument count mismatch".to_string());
            }
            unify(ba, bb, type_map)?;
            for (a, b) in ta.iter().zip(tb.iter()) {
                unify(a, b, type_map)?;
            }
            Ok(())
        }
        (Type::Nullable(a), Type::Nullable(b)) => unify(a, b, type_map),
        (Type::Function(pa, ra), Type::Function(pb, rb)) => {
            if pa.len() != pb.len() {
                return Err("Function arity mismatch".to_string());
            }
            for (a, b) in pa.iter().zip(pb.iter()) {
                unify(a, b, type_map)?;
            }
            unify(ra, rb, type_map)
        }
        (Type::Struct(fa), Type::Struct(fb)) => {
            if fa.len() != fb.len() {
                return Err("Struct field count mismatch".to_string());
            }
            for ((na, ta), (nb, tb)) in fa.iter().zip(fb.iter()) {
                if na != nb {
                    return Err(format!("Struct field name mismatch: {} vs {}", na, nb));
                }
                unify(ta, tb, type_map)?;
            }
            Ok(())
        }
        (Type::Map(ka, va), Type::Map(kb, vb)) => {
            unify(ka, kb, type_map)?;
            unify(va, vb, type_map)
        }
        (Type::Set(ea), Type::Set(eb)) => unify(ea, eb, type_map),
        (Type::Task(ta), Type::Task(tb)) => unify(ta, tb, type_map),
        (Type::Stream(sa), Type::Stream(sb)) => unify(sa, sb, type_map),
        (Type::LazyList(la), Type::LazyList(lb)) => unify(la, lb, type_map),
        (Type::Ptr(pa), Type::Ptr(pb)) => unify(pa, pb, type_map),
        (Type::Unit, Type::Unit) => Ok(()),
        (Type::Nullable(inner), _) if !matches!(actual, Type::Nullable(_)) => {
            unify(inner, actual, type_map)
        }
        (_, Type::Nullable(inner)) if matches!(inner.as_ref(), Type::Named(n) if n == "Nothing") => {
            Ok(())
        }
        _ => Err(format!("Type mismatch: {} vs {}", expected, actual)),
    }
}

/// Infer type arguments for a generic call by unifying parameter types with argument types.
pub fn infer_type_args(
    param_tys: &[Type],
    arg_tys: &[Type],
) -> Result<HashMap<String, Type>, String> {
    let mut type_map = HashMap::new();
    for (param_ty, arg_ty) in param_tys.iter().zip(arg_tys.iter()) {
        unify(param_ty, arg_ty, &mut type_map)?;
    }
    Ok(type_map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Type;

    #[test]
    fn mangle_name_empty_params() {
        assert_eq!(mangle_name("foo", &[]), "foo");
    }

    #[test]
    fn mangle_name_with_params() {
        assert_eq!(
            mangle_name(
                "add",
                &[Type::Named("Int".into()), Type::Named("Float".into())]
            ),
            "add_Int_Float"
        );
    }

    #[test]
    fn types_compatible_null_literal() {
        let int_null = Type::Nullable(Box::new(Type::Named("Int".into())));
        let nothing_null = Type::Nullable(Box::new(Type::Named("Nothing".into())));
        assert!(types_compatible(&int_null, &nothing_null));
    }

    #[test]
    fn normalize_type_aliases() {
        assert_eq!(normalize_type_name("Str"), "String");
        assert_eq!(normalize_type_name("Double"), "Float");
    }
}
