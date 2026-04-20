# Decision: Graceful Shutdown

## Problem

No coordinated shutdown signal. ctrl_c killed SSE immediately, ACP
thread had no way to stop, heartbeat task was abandoned.

## Decision

Two-mechanism shutdown coordination:

| Mechanism | Used by | Why |
|-----------|---------|-----|
| `tokio::sync::watch<bool>` | stdio, SSE, heartbeat | Async-friendly, works with `select!` |
| `Arc<AtomicBool>` | ACP thread | ACP uses `LocalSet` (!Send), can't use async watch across threads |

## Shutdown flow

```
ctrl_c → handler sets AtomicBool + sends on watch channel
  → stdio: select! wakes, exits
  → SSE: watch.changed() wakes, exits
  → ACP: supervision loop checks flag on next iteration, exits
  → heartbeat: select! wakes, exits
  → serve() logs "server stopped", returns
```

## What's not done

- No grace period for in-flight requests (dropped immediately)
- No SIGTERM handling (only ctrl_c / SIGINT)
