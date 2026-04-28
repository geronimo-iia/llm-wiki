//! Git-backed wiki engine. Full-text search, typed pages, concept graph,
//! MCP and ACP transports. The CLI is the primary interface; this crate also
//! exposes the engine internals for embedding or testing.

/// ACP (Agent Client Protocol) transport and session handling.
pub mod acp;
/// CLI argument structs and subcommand enums.
pub mod cli;
/// Global and per-wiki configuration types and loaders.
pub mod config;
/// Embedded default JSON schemas and body templates.
pub mod default_schemas;
/// Central wiki engine — mounts spaces and manages indexes.
pub mod engine;
/// Frontmatter parsing, scaffolding, and serialization helpers.
pub mod frontmatter;
/// Git commit, history, and change-detection helpers.
pub mod git;
/// Concept graph construction, community detection, and renderers.
pub mod graph;
/// Tantivy index lifecycle manager for a single wiki space.
pub mod index_manager;
/// Tantivy schema builder and field classification.
pub mod index_schema;
/// File ingestion, validation, and optional redaction.
pub mod ingest;
/// Wikilink and cross-wiki link extraction and classification.
pub mod links;
/// Markdown page read/write, asset, and scaffolding helpers.
pub mod markdown;
/// MCP server and tool handlers.
pub mod mcp;
/// High-level operations called by CLI and server handlers.
pub mod ops;
/// Full-text BM25 search and paginated list operations.
pub mod search;
/// HTTP and stdio server entry points.
pub mod server;
/// Slug validation, resolution, and URI parsing.
pub mod slug;
/// Builds SpaceTypeRegistry and IndexSchema from schema files.
pub mod space_builder;
/// Wiki space creation, registration, and management.
pub mod spaces;
/// Per-wiki type registry — schema compilation and validation.
pub mod type_registry;
/// Filesystem watcher for auto-ingest on file save.
pub mod watch;
