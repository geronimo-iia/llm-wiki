# ACP Manual Test Matrix

Connect Zed to `llm-wiki serve --acp`. One session per block.

## research
| Prompt | Expected |
|--------|----------|
| `what is sparse routing?` | search tool call + read tool call + page body + slug list |
| `llm-wiki:research scaling laws` | same via explicit prefix |
| `llm-wiki:research zzz-no-match` | "No results found" message |

## lint
| Prompt | Expected |
|--------|----------|
| `llm-wiki:lint` | tool call + summary line + one line per finding |
| `llm-wiki:lint orphan` | only orphan findings |
| `llm-wiki:lint stale,broken-link` | stale + broken-link findings only |

## graph
| Prompt | Expected |
|--------|----------|
| `llm-wiki:graph` | tool call + "N nodes, M edges" + graph text |
| `llm-wiki:graph concepts/moe` | subgraph from that root |
| `llm-wiki:graph zzz-missing` | error message in tool call result |

## ingest
| Prompt | Expected |
|--------|----------|
| `llm-wiki:ingest` | tool call + summary (pages validated, commit) |
| `llm-wiki:ingest wiki/concepts/test.md` | ingest specific file |
| `llm-wiki:ingest /nonexistent` | tool call Failed + error text |

## use
| Prompt | Expected |
|--------|----------|
| `llm-wiki:use concepts/moe` | tool call Completed + full page markdown streamed |
| `llm-wiki:use` | "Usage: llm-wiki:use <slug>" |
| `llm-wiki:use zzz-missing` | tool call Failed + error text |

## help / unknown
| Prompt | Expected |
|--------|----------|
| `llm-wiki:help` | workflow listing |
| `llm-wiki:bogus` | "Unknown workflow" + workflow listing |

## Cancellation (Phase 2)

Start a workflow on a wiki with many pages, then cancel mid-run from the IDE.

| Scenario | Steps | Expected |
|----------|-------|----------|
| Cancel lint mid-run | Send `llm-wiki:lint`, immediately cancel | `"Cancelled."` message, no further findings |
| Cancel research mid-run | Send `llm-wiki:research <query>`, cancel after search | `"Cancelled."` after search tool result |
| New prompt after cancel | Cancel then send a new prompt | New prompt executes normally (flag reset) |

## Session cap (Phase 2)

| Scenario | Steps | Expected |
|----------|-------|----------|
| Exceed `acp_max_sessions` | Open 21 sessions with default config | 21st `NewSession` returns `InvalidParams` error with "Session limit reached (max: 20)" |

Config: `llm-wiki config set serve.acp_max_sessions 2 --global`, then open 3 sessions to test with a lower cap.

## Watcher push (Phase 2)

Requires `llm-wiki serve --acp --watch`.

| Scenario | Steps | Expected |
|----------|-------|----------|
| File drop triggers push | Open session targeting wiki; drop `.md` file in `wiki/`; wait for debounce | Session receives `"Wiki "<name>" updated: 1 page(s) changed."` |
| Active session not pushed | Open session with active run; drop file | No push during active run |
| Session on different wiki not pushed | Two sessions on different wikis; drop file in wiki A | Only wiki A session receives push |
