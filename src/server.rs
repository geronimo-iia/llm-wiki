use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use rmcp::transport::SseServer;
use rmcp::ServiceExt;
use tokio::sync::watch;

use crate::config;
use crate::engine::WikiEngine;
use crate::mcp::McpServer;

// ── serve_stdio ───────────────────────────────────────────────────────────────

async fn serve_stdio(server: McpServer, mut shutdown: watch::Receiver<bool>) -> Result<()> {
    let transport = rmcp::transport::io::stdio();
    let service = server
        .serve(transport)
        .await
        .map_err(|e| anyhow::anyhow!("failed to start MCP stdio server: {e}"))?;

    tokio::select! {
        result = service.waiting() => {
            result.map_err(|e| anyhow::anyhow!("MCP stdio server error: {e}"))?;
        }
        _ = shutdown.changed() => {
            tracing::info!("stdio: shutdown signal received");
        }
    }
    Ok(())
}

// ── serve_sse ─────────────────────────────────────────────────────────────────

async fn serve_sse(
    server: McpServer,
    port: u16,
    serve_cfg: &config::ServeConfig,
    mut shutdown: watch::Receiver<bool>,
) -> Result<()> {
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let max_attempts = if serve_cfg.max_restarts == 0 {
        1
    } else {
        serve_cfg.max_restarts
    };
    let mut backoff = std::time::Duration::from_secs(serve_cfg.restart_backoff as u64);
    let max_backoff = std::time::Duration::from_secs(30);

    for attempt in 1..=max_attempts {
        match SseServer::serve(addr).await {
            Ok(sse_server) => {
                tracing::info!(%addr, "SSE server listening");
                let _ct = sse_server.with_service(move || server.clone());
                shutdown.changed().await.ok();
                tracing::info!("SSE: shutdown signal received");
                return Ok(());
            }
            Err(e) => {
                if attempt == max_attempts {
                    return Err(anyhow::anyhow!(
                        "SSE bind failed after {max_attempts} attempts: {e}"
                    ));
                }
                tracing::warn!(
                    %addr,
                    error = %e,
                    attempt,
                    max = max_attempts,
                    "SSE bind failed, retrying",
                );
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
            }
        }
    }
    unreachable!()
}

// ── serve (orchestration) ─────────────────────────────────────────────────────

pub async fn serve(config_path: &std::path::Path, sse_port: Option<u16>, acp: bool) -> Result<()> {
    // 1. Build WikiEngine
    let manager = Arc::new(WikiEngine::build(config_path)?);

    let (wiki_count, serve_cfg, sse_enabled, resolved_port) = {
        let engine = manager.state.read().map_err(|_| anyhow::anyhow!("lock"))?;
        let count = engine.spaces.len();
        let cfg = engine.config.serve.clone();
        let sse = sse_port.is_some() || cfg.sse;
        let port = sse_port.unwrap_or(cfg.sse_port);
        (count, cfg, sse, port)
    };

    // 2. Log startup summary
    let mut transports = vec!["stdio".to_string()];
    if sse_enabled {
        transports.push(format!("sse :{resolved_port}"));
    }
    if acp {
        transports.push("acp".to_string());
    }
    tracing::info!(
        wikis = wiki_count,
        transports = %transports.join("] ["),
        "server started",
    );

    // 3. Shutdown coordination
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let shutdown_flag = Arc::new(AtomicBool::new(false));

    // ctrl_c handler
    let flag_for_signal = shutdown_flag.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("shutdown signal received");
        flag_for_signal.store(true, Ordering::SeqCst);
        let _ = shutdown_tx.send(true);
    });

    // 4. Build MCP server
    let mcp_server = McpServer::new(manager.clone());

    // 5. Heartbeat task
    if serve_cfg.heartbeat_secs > 0 {
        let interval_secs = serve_cfg.heartbeat_secs;
        let mut hb_shutdown = shutdown_rx.clone();
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(interval_secs as u64));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        tracing::debug!("heartbeat");
                    }
                    _ = hb_shutdown.changed() => {
                        break;
                    }
                }
            }
        });
    }

    // 6. Start transports
    if acp {
        let acp_manager = manager.clone();
        let acp_flag = shutdown_flag.clone();
        let max_restarts = serve_cfg.max_restarts;
        let initial_backoff_secs = serve_cfg.restart_backoff;

        let acp_thread = std::thread::spawn(move || {
            let max_backoff = std::time::Duration::from_secs(30);
            let mut backoff = std::time::Duration::from_secs(initial_backoff_secs as u64);
            let mut restarts = 0u32;

            loop {
                if acp_flag.load(Ordering::SeqCst) {
                    tracing::info!("ACP: shutdown signal received");
                    break Ok(());
                }

                let mgr = acp_manager.clone();
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("failed to build ACP runtime");
                match rt.block_on(crate::acp::serve_acp(mgr)) {
                    Ok(()) => break Ok(()),
                    Err(e) => {
                        if acp_flag.load(Ordering::SeqCst) {
                            break Ok(());
                        }
                        restarts += 1;
                        tracing::error!(
                            transport = "acp",
                            error = %e,
                            restart = restarts,
                            max = max_restarts,
                            "transport crashed",
                        );
                        if max_restarts > 0 && restarts >= max_restarts {
                            tracing::error!("ACP max restarts reached, giving up");
                            break Err(e);
                        }
                        std::thread::sleep(backoff);
                        backoff = (backoff * 2).min(max_backoff);
                    }
                }
            }
        });

        if sse_enabled {
            serve_sse(mcp_server, resolved_port, &serve_cfg, shutdown_rx).await?;
        } else {
            serve_stdio(mcp_server, shutdown_rx).await?;
        }

        // Wait briefly for ACP thread to notice the flag
        let _ = acp_thread.join();
    } else if sse_enabled {
        serve_sse(mcp_server, resolved_port, &serve_cfg, shutdown_rx).await?;
    } else {
        serve_stdio(mcp_server, shutdown_rx).await?;
    }

    tracing::info!("server stopped");
    Ok(())
}
