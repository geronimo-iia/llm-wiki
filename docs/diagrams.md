# Diagrams

Mermaid sources for all llm-wiki diagrams. Intended for inline use in the
README and specification documents.

---

## 1. Architecture Overview

How the engine sits between humans, LLMs, and the wiki repository.

→ [Overview](specifications/overview.md) · [Serve](specifications/commands/serve.md)

```mermaid
graph LR
    Human([Human])
    LLM([LLM])

    subgraph Engine["wiki engine"]
        CLI[CLI]
        MCP[MCP server]
        ACP[ACP server]
    end

    subgraph Repo["git repository"]
        inbox[inbox/]
        raw[raw/]
        wiki[wiki/]
    end

    Git[(git)]
    Index[(tantivy index)]

    Human -->|drops files| inbox
    Human -->|commands| CLI
    LLM -->|tools| MCP
    LLM -->|prompts| ACP

    CLI --> wiki
    MCP --> wiki
    ACP --> wiki

    wiki --> Git
    wiki --> Index
    raw --> Git
```

---

## 2. Repository Layers

The four-layer structure of a wiki repository.

→ [Repository layout](specifications/core/repository-layout.md)

```mermaid
graph TD
    Root["my-wiki/"]

    Root --> README["README.md — for humans"]
    Root --> Config["wiki.toml — per-wiki config"]
    Root --> Schema["schema.md — categories, conventions"]
    Root --> Inbox["inbox/ — drop zone"]
    Root --> Raw["raw/ — immutable archive"]
    Root --> Wiki["wiki/ — compiled knowledge"]

    Inbox -..->|"human drops files"| Inbox
    Raw -..->|"originals preserved"| Raw
    Wiki -..->|"authors write here"| Wiki

    style Inbox fill:#ffeeba
    style Raw fill:#d4edda
    style Wiki fill:#cce5ff
```

---

## 3. Ingest Pipeline

How content enters the wiki — from source to committed knowledge.

→ [Ingest](specifications/pipelines/ingest.md)

```mermaid
flowchart LR
    A[Author writes file\ninto wiki/ tree] --> B{wiki ingest}
    B --> C[Validate frontmatter]
    C -->|valid| D[git add + commit]
    C -->|invalid| E[Error — file rejected]
    D --> F[Update tantivy index]
    F --> G[IngestReport returned]

    style E fill:#f8d7da
    style G fill:#d4edda
```

---

## 4. LLM Ingest Workflow

The full LLM-driven ingest loop via MCP tools.

→ [Ingest](specifications/pipelines/ingest.md) · [MCP clients](specifications/integrations/mcp-clients.md)

```mermaid
sequenceDiagram
    participant LLM
    participant Engine as wiki engine
    participant Repo as git repo

    LLM->>Engine: wiki_search("topic")
    Engine-->>LLM: related pages

    LLM->>Engine: wiki_read(hub page)
    Engine-->>LLM: current knowledge

    Note over LLM: reads schema.md<br/>reads inbox file<br/>synthesizes pages

    LLM->>Engine: wiki_write("concepts/topic.md", content)
    Engine-->>LLM: ok

    LLM->>Engine: wiki_ingest("concepts/topic.md")
    Engine->>Repo: validate → git commit → index
    Engine-->>LLM: IngestReport
```

---

## 5. Bootstrap / Crystallize Loop

The compounding loop across sessions — each session starts richer than the last.

→ [Session bootstrap](specifications/llm/session-bootstrap.md) · [Crystallize](specifications/pipelines/crystallize.md)

```mermaid
flowchart TD
    B["bootstrap\nread hub pages"] --> W["work\nsearch, read, write"]
    W --> C["crystallize\ndistil session into pages"]
    C --> I["wiki ingest\nvalidate, commit, index"]
    I --> R["hub pages updated\nwiki is richer"]
    R -->|"next session"| B

    style B fill:#cce5ff
    style C fill:#ffeeba
    style R fill:#d4edda
```

---

## 6. Epistemic Model

The three epistemic roles and how they relate.

→ [Epistemic model](specifications/core/epistemic-model.md) · [Source classification](specifications/core/source-classification.md)

```mermaid
graph TD
    C["concept\nwhat we know"]
    S1["paper / article / docs\nwhat sources claim"]
    Q["query-result\nwhat we concluded"]

    S1 -->|"feeds into"| C
    C -->|"used by"| Q
    S1 -->|"cited by"| Q

    C -.-|"provenance"| S1
    Q -.-|"auditable"| S1

    style C fill:#cce5ff
    style S1 fill:#d4edda
    style Q fill:#ffeeba
```

---

## 7. RAG vs DKR

Side-by-side comparison of the two approaches.

→ [Overview](specifications/overview.md)

```mermaid
flowchart LR
    subgraph RAG["Traditional RAG"]
        direction TB
        RQ[Query] --> RR[Retrieve chunks]
        RR --> RG[Generate answer]
        RG --> RA[Answer — ephemeral]
    end

    subgraph DKR["llm-wiki DKR"]
        direction TB
        DS[Source arrives] --> DI[LLM processes at ingest]
        DI --> DW[Wiki pages updated]
        DW --> DC[Knowledge compounds]
        DC -->|"next source"| DI
    end

    style RA fill:#f8d7da
    style DC fill:#d4edda
```
