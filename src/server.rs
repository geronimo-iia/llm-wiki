use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use rmcp::model::{Annotated, RawResource};
use rmcp::service::{Peer, RoleServer};
use rmcp::transport::SseServer;
use rmcp::ServiceExt;

use crate::config::{self, GlobalConfig, WikiEntry};
use crate::markdown;
use crate::search;
use crate::spaces;

pub const INSTRUCTIONS: &str = include_str!("assets/instructions.md");

#[derive(Clone)]
pub struct WikiServer {
    pub wikis: Arc<Vec<WikiEntry>>,
    pub global: Arc<GlobalConfig>,
    pub config_path: PathBuf,
    pub instructions: Arc<String>,
    pub peer: Arc<Mutex<Option<Peer<RoleServer>>>>,
}

impl WikiServer {
    pub fn new(global: GlobalConfig, config_path: PathBuf) -> Result<Self> {
        let wikis = spaces::load_all(&global);

        let mut full_instructions = INSTRUCTIONS.to_string();
        if let Some(default_entry) = wikis.iter().find(|w| w.name == global.global.default_wiki) {
            let schema_path = PathBuf::from(&default_entry.path).join("schema.md");
            if schema_path.exists() {
                if let Ok(schema) = std::fs::read_to_string(&schema_path) {
                    full_instructions.push_str("\n\n---\n\n# schema.md\n\n");
                    full_instructions.push_str(&schema);
                }
            }
        }

        Ok(Self {
            wikis: Arc::new(wikis),
            global: Arc::new(global),
            config_path,
            instructions: Arc::new(full_instructions),
            peer: Arc::new(Mutex::new(None)),
        })
    }

    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    pub fn default_wiki_name(&self) -> &str {
        &self.global.global.default_wiki
    }

    pub fn index_path_for(wiki_name: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home)
            .join(".wiki")
            .join("indexes")
            .join(wiki_name)
    }

    pub fn list_wiki_resources(&self) -> Vec<rmcp::model::Resource> {
        let mut resources = Vec::new();
        for entry in self.wikis.iter() {
            let wiki_root = PathBuf::from(&entry.path).join("wiki");
            if !wiki_root.exists() {
                continue;
            }
            for file in walkdir::WalkDir::new(&wiki_root)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = file.path();
                if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let slug = markdown::slug_for(path, &wiki_root);
                let uri = format!("wiki://{}/{}", entry.name, slug);
                resources.push(Annotated {
                    raw: RawResource {
                        uri,
                        name: slug,
                        description: None,
                        mime_type: Some("text/markdown".into()),
                        size: None,
                    },
                    annotations: None,
                });
            }
        }
        resources
    }
}

pub async fn serve_stdio(server: WikiServer) -> Result<()> {
    let transport = rmcp::transport::io::stdio();
    let service = server
        .serve(transport)
        .await
        .map_err(|e| anyhow::anyhow!("failed to start MCP stdio server: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP stdio server error: {e}"))?;
    Ok(())
}

pub async fn serve_sse(server: WikiServer, port: u16) -> Result<()> {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let sse_server = SseServer::serve(addr)
        .await
        .map_err(|e| anyhow::anyhow!("failed to start SSE server on {addr}: {e}"))?;
    tracing::info!(%addr, "SSE server listening");
    let _ct = sse_server.with_service(move || server.clone());
    tokio::signal::ctrl_c().await?;
    Ok(())
}

pub async fn serve(
    global: GlobalConfig,
    config_path: PathBuf,
    sse: bool,
    sse_port: u16,
    acp: bool,
    dry_run: bool,
) -> Result<()> {
    let wikis = spaces::load_all(&global);
    let wiki_count = wikis.len();

    let resolved_default = {
        let wiki_cfg =
            if let Some(entry) = wikis.iter().find(|w| w.name == global.global.default_wiki) {
                config::load_wiki(&PathBuf::from(&entry.path)).unwrap_or_default()
            } else {
                config::WikiConfig::default()
            };
        config::resolve(&global, &wiki_cfg)
    };

    for entry in &wikis {
        let repo_root = PathBuf::from(&entry.path);
        let index_path = WikiServer::index_path_for(&entry.name);
        if let Ok(status) = search::index_status(&entry.name, &index_path, &repo_root) {
            if status.stale && resolved_default.index.auto_rebuild {
                let wiki_root = repo_root.join("wiki");
                if let Err(e) =
                    search::rebuild_index(&wiki_root, &index_path, &entry.name, &repo_root)
                {
                    tracing::warn!(wiki = %entry.name, error = %e, "index rebuild failed");
                }
            } else if status.stale {
                tracing::warn!(
                    wiki = %entry.name,
                    "index stale — run `wiki index rebuild --wiki {}`",
                    entry.name,
                );
            }
        }
    }

    let mut transports = vec!["stdio".to_string()];
    if sse {
        transports.push(format!("sse :{sse_port}"));
    }
    if acp {
        transports.push("acp".to_string());
    }

    tracing::info!(
        wikis = wiki_count,
        transports = %transports.join("] ["),
        "server started",
    );

    if dry_run {
        return Ok(());
    }

    let server = WikiServer::new(global.clone(), config_path)?;

    if acp {
        let global_arc = Arc::new((*server.global).clone());
        // ACP uses LocalSet (not Send), so run it on a dedicated thread
        let acp_thread = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build ACP runtime");
            rt.block_on(crate::acp::serve_acp(global_arc))
        });

        if sse {
            serve_sse(server, sse_port).await?;
        } else {
            serve_stdio(server).await?;
        }

        acp_thread
            .join()
            .map_err(|_| anyhow::anyhow!("ACP thread panicked"))??;
        Ok(())
    } else if sse {
        serve_sse(server, sse_port).await
    } else {
        serve_stdio(server).await
    }
}
