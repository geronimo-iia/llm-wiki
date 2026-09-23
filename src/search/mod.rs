//! Search and list operations over a Tantivy index.
//!
//! - [`search`] — BM25 full-text search against a single wiki index.
//! - [`search_all`] — cross-wiki search, merges and re-ranks results.
//! - [`list`] — paginated page listing with sort and filter.
//! - [`render_search_llms`] / [`render_list_llms`] — LLM-optimized markdown renderers.

#![allow(unreachable_pub)]

mod facets;
mod fn_search;
mod list;
mod options;
mod render;
mod search_all;
mod types;

pub use fn_search::search;
pub use list::list;
pub use options::{ListOptions, SearchOptions, SortField, SortOrder, TagsMode};
pub use render::{render_list_llms, render_search_llms};
pub use search_all::search_all;
pub use types::{FacetCounts, PageList, PageRef, PageSummary, SearchResult};

#[cfg(test)]
mod tests {
    use tantivy::Index;
    use tantivy::query::QueryParser;
    use tantivy::schema::{SchemaBuilder, TEXT};

    /// `parse_query` fails on bare field specifiers like `title:attention` when
    /// `title` is not a registered query field. `parse_query_lenient` must
    /// succeed and return a usable query rather than propagating the error.
    #[test]
    fn parse_query_lenient_fallback_on_field_specifier() {
        let mut builder = SchemaBuilder::new();
        let body = builder.add_text_field("body", TEXT);
        let schema = builder.build();
        let index = Index::create_in_ram(schema);
        let parser = QueryParser::for_index(&index, vec![body]);

        // `title:attention` fails strict parse (title not in query fields)
        assert!(parser.parse_query("title:attention").is_err());
        // lenient parse must succeed
        let (query, _errors) = parser.parse_query_lenient("title:attention");
        // the returned query must be usable (searcher.search won't panic)
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let count = searcher.search(&query, &tantivy::collector::Count).unwrap();
        assert_eq!(count, 0); // empty index — just verifying no panic
    }

    /// Type-filter query: indexing two docs with different types and filtering on one
    /// must return only the matching doc. Mirrors the BooleanQuery + TermQuery path
    /// in the production `search()` function.
    #[test]
    fn type_filter_excludes_non_matching_type() {
        use tantivy::doc;
        use tantivy::query::{BooleanQuery, Occur, TermQuery};
        use tantivy::schema::{IndexRecordOption, STRING, SchemaBuilder, TEXT};
        use tantivy::{Index, Term};

        let mut builder = SchemaBuilder::new();
        let f_body = builder.add_text_field("body", TEXT);
        let f_type = builder.add_text_field("type", STRING);
        let schema = builder.build();
        let index = Index::create_in_ram(schema.clone());

        let mut writer = index.writer(15_000_000).unwrap();
        writer
            .add_document(doc!(f_body => "attention mechanism", f_type => "concept"))
            .unwrap();
        writer
            .add_document(doc!(f_body => "mixtral paper", f_type => "source"))
            .unwrap();
        writer.commit().unwrap();

        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let parser = QueryParser::for_index(&index, vec![f_body]);
        let base = parser.parse_query("attention").unwrap();

        // Filter: type must be "concept"
        let type_term = Term::from_field_text(f_type, "concept");
        let filtered = BooleanQuery::new(vec![
            (Occur::Must, base),
            (
                Occur::Must,
                Box::new(TermQuery::new(type_term, IndexRecordOption::Basic)),
            ),
        ]);

        let count = searcher
            .search(&filtered, &tantivy::collector::Count)
            .unwrap();
        assert_eq!(count, 1, "type filter must return only the concept doc");
    }
}
