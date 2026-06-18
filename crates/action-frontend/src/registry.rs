use crate::ast::*;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    pub field_index: HashMap<String, usize>,
}

#[derive(Clone, Debug)]
pub struct EnumInfo {
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<EnumVariantInfo>,
}

#[derive(Clone, Debug)]
pub struct EnumVariantInfo {
    pub name: String,
    pub tag: u32,
    pub params: Vec<EnumVariantParam>,
}

#[derive(Default, Clone)]
pub struct TypeRegistry {
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub type_aliases: HashMap<String, Type>,
    pub variant_to_enum: HashMap<String, String>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        TypeRegistry {
            structs: HashMap::new(),
            enums: HashMap::new(),
            type_aliases: HashMap::new(),
            variant_to_enum: HashMap::new(),
        }
    }

    pub fn register(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::TypeAlias {
                name, definition, ..
            } => {
                if let Type::Struct(fields) = definition {
                    let mut field_index = HashMap::new();
                    for (i, (fname, _)) in fields.iter().enumerate() {
                        field_index.insert(fname.clone(), i);
                    }
                    self.structs.insert(
                        name.clone(),
                        StructInfo {
                            name: name.clone(),
                            fields: fields.clone(),
                            field_index,
                        },
                    );
                }
                self.type_aliases.insert(name.clone(), definition.clone());
            }
            Stmt::Enum {
                name,
                type_params,
                variants,
                ..
            } => {
                let mut enum_variants = Vec::new();
                for (i, v) in variants.iter().enumerate() {
                    self.variant_to_enum.insert(v.name.clone(), name.clone());
                    enum_variants.push(EnumVariantInfo {
                        name: v.name.clone(),
                        tag: i as u32,
                        params: v.params.clone(),
                    });
                }
                self.enums.insert(
                    name.clone(),
                    EnumInfo {
                        name: name.clone(),
                        type_params: type_params.clone(),
                        variants: enum_variants,
                    },
                );
            }
            Stmt::ExternalType { name, .. } => {
                // Register as opaque struct (no fields)
                self.structs.insert(
                    name.clone(),
                    StructInfo {
                        name: name.clone(),
                        fields: vec![],
                        field_index: HashMap::new(),
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }

    /// Find the struct type whose field names match exactly. Returns the struct info if unique.
    pub fn find_struct_by_fields(&self, field_names: &[String]) -> Option<&StructInfo> {
        let matches: Vec<&StructInfo> = self
            .structs
            .values()
            .filter(|s| {
                if s.fields.len() != field_names.len() {
                    return false;
                }
                field_names
                    .iter()
                    .enumerate()
                    .all(|(i, name)| s.fields[i].0 == *name)
            })
            .collect();
        if matches.len() == 1 {
            Some(matches[0])
        } else {
            None
        }
    }

    /// Look up an enum variant by name. Returns (enum_info, variant_info).
    pub fn lookup_variant(&self, variant_name: &str) -> Option<(&EnumInfo, &EnumVariantInfo)> {
        let enum_name = self.variant_to_enum.get(variant_name)?;
        let info = self.enums.get(enum_name)?;
        let variant = info.variants.iter().find(|v| v.name == variant_name)?;
        Some((info, variant))
    }

    pub fn get_struct(&self, name: &str) -> Option<&StructInfo> {
        self.structs.get(name)
    }

    /// Check that a set of when arms covers all variants of the enum they match on.
    pub fn check_when_exhaustive(&self, arms: &[WhenArm]) -> Result<(), String> {
        crate::exhaustive::check_when_exhaustive(self, arms)
    }
}
