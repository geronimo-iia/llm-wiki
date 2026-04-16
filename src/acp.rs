/// ACP transport — stub for Phase 6. Full implementation in Phase 7.
pub struct WikiAgent;

pub struct AcpSession {
    pub id: String,
    pub label: Option<String>,
    pub wiki: Option<String>,
    pub created_at: u64,
    pub active_run: Option<String>,
}
