use nodo_core::Schema;
use std::collections::HashMap;

/// Collection of known schemas
///
/// See also: https://mcap.dev/spec/registry#well-known-schema-encodings
#[derive(Default)]
pub struct SchemaSet {
    schemas: HashMap<Schema, &'static [u8]>,
}

impl SchemaSet {
    pub fn insert(&mut self, schema: Schema, def: &'static [u8]) {
        self.schemas.insert(schema, def);
    }

    /// Looks up a schema
    pub fn lookup(&self, schema: &Schema) -> Option<&'static [u8]> {
        self.schemas.get(schema).copied()
    }
}
