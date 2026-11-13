use std::cmp::Reverse;
use std::collections::HashMap;

use anyhow::Result;
use tantivy::{
    Searcher,
    collector::{Collector, MultiCollector, SegmentCollector},
};

// ── Facet collection ──────────────────────────────────────────────────────────

pub(super) struct KeywordFacetCollector {
    pub field_name: String,
    pub top_n: usize,
}

pub(super) struct KeywordFacetSegmentCollector {
    column: Option<tantivy::columnar::StrColumn>,
    buf: String,
    counts: HashMap<String, u64>,
}

impl Collector for KeywordFacetCollector {
    type Fruit = HashMap<String, u64>;
    type Child = KeywordFacetSegmentCollector;

    fn for_segment(
        &self,
        _segment_local_id: u32,
        reader: &tantivy::SegmentReader,
    ) -> tantivy::Result<Self::Child> {
        let column = reader.fast_fields().str(&self.field_name).ok().flatten();
        Ok(KeywordFacetSegmentCollector {
            column,
            buf: String::new(),
            counts: HashMap::new(),
        })
    }

    fn requires_scoring(&self) -> bool {
        false
    }

    fn merge_fruits(
        &self,
        fruits: Vec<HashMap<String, u64>>,
    ) -> tantivy::Result<HashMap<String, u64>> {
        let mut merged: HashMap<String, u64> = HashMap::new();
        for f in fruits {
            for (k, v) in f {
                *merged.entry(k).or_insert(0) += v;
            }
        }
        if self.top_n > 0 && merged.len() > self.top_n {
            let mut entries: Vec<_> = merged.into_iter().collect();
            entries.sort_by_key(|e| Reverse(e.1));
            entries.truncate(self.top_n);
            return Ok(entries.into_iter().collect());
        }
        Ok(merged)
    }
}

impl SegmentCollector for KeywordFacetSegmentCollector {
    type Fruit = HashMap<String, u64>;

    fn collect(&mut self, doc: tantivy::DocId, _score: tantivy::Score) {
        let Some(col) = &self.column else { return };
        for ord in col.term_ords(doc) {
            self.buf.clear();
            if col.ord_to_str(ord, &mut self.buf).unwrap_or(false) && !self.buf.is_empty() {
                *self.counts.entry(self.buf.clone()).or_insert(0) += 1;
            }
        }
    }

    fn harvest(self) -> HashMap<String, u64> {
        self.counts
    }
}

pub(super) fn collect_facets(
    searcher: &Searcher,
    query: &dyn tantivy::query::Query,
    fields: &[(&str, usize)],
) -> Result<Vec<HashMap<String, u64>>> {
    if fields.is_empty() {
        return Ok(Vec::new());
    }
    let mut multi = MultiCollector::new();
    let handles: Vec<_> = fields
        .iter()
        .map(|(name, top_n)| {
            multi.add_collector(KeywordFacetCollector {
                field_name: name.to_string(),
                top_n: *top_n,
            })
        })
        .collect();
    let mut fruits = searcher.search(query, &multi)?;
    Ok(handles
        .into_iter()
        .map(|h| h.extract(&mut fruits))
        .collect())
}
