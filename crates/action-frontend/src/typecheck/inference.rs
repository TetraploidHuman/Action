//! Hindley-Milner style type inference (Algorithm W subset).

use crate::ast::Type;
use crate::types::normalize_type_name;
use std::collections::HashMap;

/// Solver state: substitution map for internal inference variables.
pub struct InferenceEngine {
    subst: HashMap<u32, Type>,
    next_var: u32,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            subst: HashMap::new(),
            next_var: 0,
        }
    }

    pub fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::InferVar(id)
    }

    /// Apply substitution recursively.
    pub fn resolve(&self, ty: &Type) -> Type {
        match ty {
            Type::InferVar(id) => {
                if let Some(t) = self.subst.get(id) {
                    let resolved = self.resolve(t);
                    // path compression would mutate subst; keep simple
                    resolved
                } else {
                    ty.clone()
                }
            }
            Type::Generic(base, params) => Type::Generic(
                Box::new(self.resolve(base)),
                params.iter().map(|p| self.resolve(p)).collect(),
            ),
            Type::Function(params, ret) => Type::Function(
                params.iter().map(|p| self.resolve(p)).collect(),
                Box::new(self.resolve(ret)),
            ),
            Type::Struct(fields) => Type::Struct(
                fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.resolve(t)))
                    .collect(),
            ),
            Type::Map(k, v) => Type::Map(Box::new(self.resolve(k)), Box::new(self.resolve(v))),
            Type::Set(t) => Type::Set(Box::new(self.resolve(t))),
            Type::Task(t) => Type::Task(Box::new(self.resolve(t))),
            Type::Stream(s) => Type::Stream(Box::new(self.resolve(s))),
            Type::LazyList(l) => Type::LazyList(Box::new(self.resolve(l))),
            Type::Ptr(inner) => Type::Ptr(Box::new(self.resolve(inner))),
            other => other.clone(),
        }
    }

    fn occurs(&self, id: u32, ty: &Type) -> bool {
        match self.resolve(ty) {
            Type::InferVar(v) => v == id,
            Type::Generic(base, params) => {
                self.occurs(id, &base) || params.iter().any(|p| self.occurs(id, p))
            }
            Type::Function(params, ret) => {
                params.iter().any(|p| self.occurs(id, p)) || self.occurs(id, &ret)
            }
            Type::Struct(fields) => fields.iter().any(|(_, t)| self.occurs(id, t)),
            Type::Map(k, v) => self.occurs(id, &k) || self.occurs(id, &v),
            Type::Set(t)
            | Type::Task(t)
            | Type::Stream(t)
            | Type::LazyList(t)
            | Type::Ptr(t) => self.occurs(id, &t),
            _ => false,
        }
    }

    /// Unify two types; binds inference variables.
    pub fn unify(&mut self, t1: &Type, t2: &Type) -> Result<(), String> {
        let t1 = self.resolve(t1);
        let t2 = self.resolve(t2);
        match (&t1, &t2) {
            (Type::InferVar(a), Type::InferVar(b)) if a == b => Ok(()),
            (Type::InferVar(id), other) | (other, Type::InferVar(id)) => {
                let other = if matches!(other, Type::InferVar(_)) {
                    self.resolve(other)
                } else {
                    other.clone()
                };
                if self.occurs(*id, &other) {
                    return Err("Infinite type in inference".to_string());
                }
                self.subst.insert(*id, other);
                Ok(())
            }
            (Type::Named(a), Type::Named(b)) => {
                if normalize_type_name(a) == normalize_type_name(b) {
                    Ok(())
                } else {
                    Err(format!("Type mismatch: {} vs {}", a, b))
                }
            }
            (Type::Unit, Type::Unit) => Ok(()),
            (Type::Generic(ba, ta), Type::Generic(bb, tb)) => {
                if ta.len() != tb.len() {
                    return Err("Generic argument count mismatch".to_string());
                }
                self.unify(ba, bb)?;
                for (a, b) in ta.iter().zip(tb.iter()) {
                    self.unify(a, b)?;
                }
                Ok(())
            }
            (Type::Function(pa, ra), Type::Function(pb, rb)) => {
                if pa.len() != pb.len() {
                    return Err("Function arity mismatch".to_string());
                }
                for (a, b) in pa.iter().zip(pb.iter()) {
                    self.unify(a, b)?;
                }
                self.unify(ra, rb)
            }
            (Type::Struct(fa), Type::Struct(fb)) => {
                if fa.len() != fb.len() {
                    return Err("Struct field count mismatch".to_string());
                }
                for ((na, ta), (nb, tb)) in fa.iter().zip(fb.iter()) {
                    if na != nb {
                        return Err(format!("Struct field name mismatch: {} vs {}", na, nb));
                    }
                    self.unify(ta, tb)?;
                }
                Ok(())
            }
            (Type::Map(ka, va), Type::Map(kb, vb)) => {
                self.unify(ka, kb)?;
                self.unify(va, vb)
            }
            (Type::Set(ea), Type::Set(eb)) => self.unify(ea, eb),
            (Type::Task(ta), Type::Task(tb)) => self.unify(ta, tb),
            (Type::Stream(sa), Type::Stream(sb)) => self.unify(sa, sb),
            (Type::LazyList(la), Type::LazyList(lb)) => self.unify(la, lb),
            (Type::Ptr(pa), Type::Ptr(pb)) => self.unify(pa, pb),
            (Type::TypeVar(name), other) | (other, Type::TypeVar(name)) => {
                // Source-level generic vars: compatible if same name or first bind
                if let Type::TypeVar(n2) = other {
                    if name == n2 {
                        return Ok(());
                    }
                }
                Err(format!(
                    "Cannot unify type variable {} with {}",
                    name, other
                ))
            }
            (Type::CString, Type::CString) | (Type::FileHandle, Type::FileHandle) => Ok(()),
            _ => Err(format!("Type mismatch: {} vs {}", t1, t2)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unify_infer_with_int() {
        let mut eng = InferenceEngine::new();
        let v = eng.fresh_var();
        eng.unify(&v, &Type::Named("Int".into())).unwrap();
        assert_eq!(eng.resolve(&v), Type::Named("Int".into()));
    }

    #[test]
    fn unify_function_types() {
        let mut eng = InferenceEngine::new();
        let a = eng.fresh_var();
        let b = eng.fresh_var();
        let f1 = Type::Function(vec![a.clone()], Box::new(b.clone()));
        let f2 = Type::Function(
            vec![Type::Named("Int".into())],
            Box::new(Type::Named("Bool".into())),
        );
        eng.unify(&f1, &f2).unwrap();
        assert_eq!(eng.resolve(&a), Type::Named("Int".into()));
        assert_eq!(eng.resolve(&b), Type::Named("Bool".into()));
    }
}
