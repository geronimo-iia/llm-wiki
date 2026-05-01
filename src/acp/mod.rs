mod graph;
mod helpers;
mod ingest;
mod lint;
mod research;
mod server;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use agent_client_protocol::schema::{ContentBlock, PromptRequest};

pub use server::serve_acp;

// ── Session ───────────────────────────────────────────────────────────────────

/// An active ACP session tracking identity and execution state.
pub struct AcpSession {
    /// Unique session identifier assigned at creation.
    pub id: String,
    /// Optional human-readable label for the session.
    pub label: Option<String>,
    /// Wiki name associated with the session, if any.
    pub wiki: Option<String>,
    /// Unix timestamp (milliseconds) when the session was created.
    pub created_at: u64,
    /// ID of the currently executing tool run, if any.
    pub active_run: Option<String>,
}

type Sessions = Arc<Mutex<HashMap<String, AcpSession>>>;

// ── Dispatch ──────────────────────────────────────────────────────────────────

/// Parse `llm-wiki:<workflow> <text>` or fall back to keyword matching.
pub fn dispatch_workflow(prompt: &str) -> (&str, &str) {
    if let Some(rest) = prompt.strip_prefix("llm-wiki:") {
        let rest = rest.trim_start();
        if let Some(pos) = rest.find(char::is_whitespace) {
            let workflow = &rest[..pos];
            let text = rest[pos..].trim_start();
            return (workflow, text);
        }
        return (rest, "");
    }
    ("research", prompt)
}

fn extract_prompt_text(req: &PromptRequest) -> String {
    req.prompt
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Convenience alias for ACP step return values.
pub type StepResult<T = ()> = std::result::Result<T, agent_client_protocol::schema::Error>;

/// Generate a unique tool-run ID from workflow name, step name, and current timestamp.
pub fn make_tool_id(workflow: &str, step: &str) -> String {
    format!(
        "{workflow}-{step}-{}",
        chrono::Utc::now().timestamp_millis()
    )
}
