use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::AllQuery;
use tantivy::schema::Value;

use crate::engine::EngineState;
use crate::index_schema::IndexSchema;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Options controlling the wiki export operation.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Name of the wiki to export.
    pub wiki: String,
    /// Output path — resolved against wiki root if relative.
    pub path: Option<String>,
    /// Output format: llms-txt, llms-full, or JSON.
    pub format: ExportFormat,
    /// Whether to include archived pages (default: false).
    pub include_archived: bool,
}

/// Supported wiki export formats.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ExportFormat {
    /// Compact llms.txt listing with titles and summaries.
    #[default]
    LlmsTxt,
    /// Full llms.txt with complete page bodies.
    LlmsFull,
    /// JSON array of page entries with metadata and bodies.
    Json,
}

impl ExportFormat {
    /// Return the canonical string representation of the format.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportFormat::LlmsTxt => "llms-txt",
            ExportFormat::LlmsFull => "llms-full",
            ExportFormat::Json => "json",
        }
    }

    /// Parse a format string; falls back to `LlmsTxt` for unrecognised input.
    pub fn parse(s: &str) -> Self {
        match s {
            "llms-full" => ExportFormat::LlmsFull,
            "json" => ExportFormat::Json,
            _ => ExportFormat::LlmsTxt,
        }
    }
}

/// Summary of a completed wiki export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReport {
    /// Absolute path of the written output file.
    pub path: String,
    /// Number of pages written to the output.
    pub pages_written: usize,
    /// Total bytes written.
    pub bytes: usize,
    /// Name of the format used (e.g. `"llms-txt"`).
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageEntry {
    slug: String,
    uri: String,
    title: String,
    r#type: String,
    status: String,
    confidence: f64,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

// ── export ────────────────────────────────────────────────────────────────────

/// Export a wiki to a file in the requested format.
pub fn export(engine: &EngineState, options: &ExportOptions) -> Result<ExportReport> {
    let space = engine.space(&options.wiki)?;
    let wiki_root = &space.wiki_root;

    let resolved_path = resolve_path(options.path.as_deref(), wiki_root);

    let searcher = space.index_manager.searcher()?;
    let is = &space.index_schema;

    let pages = collect_pages(&searcher, is, &options.wiki, options.include_archived)?;

    let need_bodies = matches!(options.format, ExportFormat::LlmsFull | ExportFormat::Json);
    let pages = if need_bodies {
        load_bodies(pages, wiki_root)?
    } else {
        pages
    };

    let content = match options.format {
        ExportFormat::LlmsTxt => render_llms_txt(&pages, &options.wiki),
        ExportFormat::LlmsFull => render_llms_full(&pages, &options.wiki),
        ExportFormat::Json => {
            serde_json::to_string_pretty(&pages).context("failed to serialize pages to JSON")?
        }
    };

    if let Some(parent) = resolved_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    std::fs::write(&resolved_path, &content)
        .with_context(|| format!("failed to write export to {}", resolved_path.display()))?;

    Ok(ExportReport {
        path: resolved_path.to_string_lossy().to_string(),
        pages_written: pages.len(),
        bytes: content.len(),
        format: options.format.as_str().to_string(),
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn resolve_path(path: Option<&str>, wiki_root: &Path) -> PathBuf {
    let p = path.unwrap_or("llms.txt");
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        pb
    } else {
        wiki_root.join(pb)
    }
}

fn collect_pages(
    searcher: &tantivy::Searcher,
    is: &IndexSchema,
    wiki_name: &str,
    include_archived: bool,
) -> Result<Vec<PageEntry>> {
    let f_slug = is.field("slug");
    let f_title = is.field("title");
    let f_type = is.field("type");
    let f_status = is.field("status");
    let f_confidence = is.try_field("confidence");
    let f_summary = is.try_field("summary");

    let top_docs = searcher.search(&AllQuery, &TopDocs::with_limit(100_000).order_by_score())?;

    let mut pages = Vec::new();
    for (_score, doc_addr) in &top_docs {
        let doc: tantivy::TantivyDocument = searcher.doc(*doc_addr)?;

        let slug = doc
            .get_first(f_slug)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }

        let status = doc
            .get_first(f_status)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !include_archived && status == "archived" {
            continue;
        }

        let title = doc
            .get_first(f_title)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let page_type = doc
            .get_first(f_type)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let confidence = f_confidence
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let summary = f_summary
            .and_then(|f| doc.get_first(f))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();

        let uri = format!("wiki://{wiki_name}/{slug}");

        pages.push(PageEntry {
            slug,
            uri,
            title,
            r#type: page_type,
            status,
            confidence,
            summary,
            body: None,
        });
    }

    // Sort: group by type (count desc), within group by confidence desc then title asc
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for p in &pages {
        *type_counts.entry(p.r#type.clone()).or_insert(0) += 1;
    }
    pages.sort_by(|a, b| {
        let ca = type_counts.get(&a.r#type).copied().unwrap_or(0);
        let cb = type_counts.get(&b.r#type).copied().unwrap_or(0);
        cb.cmp(&ca)
            .then(a.r#type.cmp(&b.r#type))
            .then(
                b.confidence
                    .partial_cmp(&a.confidence)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.title.cmp(&b.title))
    });

    Ok(pages)
}

fn load_bodies(mut pages: Vec<PageEntry>, wiki_root: &Path) -> Result<Vec<PageEntry>> {
    for page in &mut pages {
        let path = wiki_root.join(format!("{}.md", page.slug));
        if path.exists() {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            // Strip frontmatter (between --- delimiters)
            let body = strip_frontmatter(&raw);
            page.body = Some(body.to_string());
        }
    }
    Ok(pages)
}

fn strip_frontmatter(content: &str) -> &str {
    if !content.starts_with("---") {
        return content;
    }
    // Find second --- after the opening
    if let Some(rest) = content[3..].find("\n---") {
        let end = 3 + rest + 4; // skip past the closing ---
        // Skip past optional newline after ---
        let end = if content[end..].starts_with('\n') {
            end + 1
        } else {
            end
        };
        &content[end..]
    } else {
        content
    }
}

// ── Renderers ─────────────────────────────────────────────────────────────────

fn render_llms_txt(pages: &[PageEntry], wiki_name: &str) -> String {
    let mut out = format!("# {wiki_name}\n\n");
    out.push_str(&format!("{} pages\n\n", pages.len()));

    let mut current_type = "";
    for page in pages {
        if page.r#type != current_type {
            current_type = &page.r#type;
            let count = pages.iter().filter(|p| p.r#type == current_type).count();
            out.push_str(&format!("## {} ({})\n\n", current_type, count));
        }
        if page.summary.is_empty() {
            out.push_str(&format!("- [{}]({})\n", page.title, page.uri));
        } else {
            out.push_str(&format!(
                "- [{}]({}): {}\n",
                page.title, page.uri, page.summary
            ));
        }
    }
    out
}

fn render_llms_full(pages: &[PageEntry], wiki_name: &str) -> String {
    let mut out = format!("# {wiki_name}\n\n");
    out.push_str(&format!("{} pages\n\n", pages.len()));

    for page in pages {
        out.push_str("---\n\n");
        out.push_str(&format!("# [{}]({})\n\n", page.title, page.uri));
        if !page.summary.is_empty() {
            out.push_str(&format!("_{}_\n\n", page.summary));
        }
        if let Some(ref body) = page.body {
            out.push_str(body.trim());
            out.push_str("\n\n");
        }
    }
    out
}
