use std::collections::HashMap;

use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, STORED, STRING,
};

/// Hardcoded tantivy schema for Phase 1.
///
/// Phase 2 replaces this with a dynamic schema derived from the type registry.
pub struct IndexSchema {
    pub schema: Schema,
    pub fields: HashMap<String, Field>,
}

impl IndexSchema {
    pub fn build(tokenizer: &str) -> Self {
        let mut builder = Schema::builder();

        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer(tokenizer)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let text_opts = TextOptions::default()
            .set_indexing_options(text_indexing)
            .set_stored();

        let mut fields = HashMap::new();

        let f = |name: &str, field: Field, map: &mut HashMap<String, Field>| {
            map.insert(name.to_string(), field);
        };

        f("slug", builder.add_text_field("slug", STRING | STORED), &mut fields);
        f("uri", builder.add_text_field("uri", STRING | STORED), &mut fields);
        f("title", builder.add_text_field("title", text_opts.clone()), &mut fields);
        f("summary", builder.add_text_field("summary", text_opts.clone()), &mut fields);
        f("body", builder.add_text_field("body", text_opts.clone()), &mut fields);
        f("type", builder.add_text_field("type", STRING | STORED), &mut fields);
        f("status", builder.add_text_field("status", STRING | STORED), &mut fields);
        f("tags", builder.add_text_field("tags", text_opts), &mut fields);
        // Multi-valued keyword field for body [[wiki-links]]
        f("body_links", builder.add_text_field("body_links", STRING | STORED), &mut fields);

        IndexSchema {
            schema: builder.build(),
            fields,
        }
    }

    pub fn field(&self, name: &str) -> Field {
        self.fields[name]
    }
}
