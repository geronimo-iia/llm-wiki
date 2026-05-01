#!/usr/bin/env bash
section "1. Session Lifecycle (ACP)"

# Helper: extract sessionId from NDJSON response
_sid_from() {
    echo "$1" | while IFS= read -r line; do
        [ -z "$line" ] && continue
        r=$(echo "$line" | jq -r '.result.sessionId // empty' 2>/dev/null)
        [ -n "$r" ] && echo "$r" && break
    done | head -1
}

# ── initialize ────────────────────────────────────────────────────────────────

{
    _ACP_REQ_ID=0
    local_msgs=$(mktemp)
    _acp_request "initialize" \
        '{"protocolVersion":1,"clientInfo":{"name":"acp-test","version":"0.1.0"}}' \
        > "$local_msgs"
    raw=$(_acp_run "$local_msgs")
    rm -f "$local_msgs"
    init_result=$(echo "$raw" | jq -r '.result.agentInfo.name // empty' 2>/dev/null | head -1)
    if [ "$init_result" = "llm-wiki" ]; then
        pass "initialize returns agentInfo.name=llm-wiki"
    else
        fail "initialize returns agentInfo.name=llm-wiki" "got: $init_result"
        echo "    raw: $(echo "$raw" | head -c 300)"
    fi
}

# ── session/new ───────────────────────────────────────────────────────────────

{
    _ACP_REQ_ID=0
    local_msgs=$(mktemp)
    _acp_request "initialize" \
        '{"protocolVersion":1,"clientInfo":{"name":"acp-test","version":"0.1.0"}}' \
        >> "$local_msgs"
    _acp_request "session/new" \
        "$(jq -cn --arg cwd "$TEST_DIR" '{"cwd":$cwd,"mcpServers":[]}')" \
        >> "$local_msgs"
    raw=$(_acp_run "$local_msgs")
    rm -f "$local_msgs"
    sid=$(_sid_from "$raw")
    if [ -n "$sid" ]; then
        pass "session/new returns sessionId"
        export _LIFECYCLE_SID="$sid"
    else
        fail "session/new returns sessionId" "no sessionId in output"
        echo "    raw: $(echo "$raw" | head -c 300)"
    fi
}

# ── session/new with wiki meta ────────────────────────────────────────────────

{
    _ACP_REQ_ID=0
    local_msgs=$(mktemp)
    _acp_request "initialize" \
        '{"protocolVersion":1,"clientInfo":{"name":"acp-test","version":"0.1.0"}}' \
        >> "$local_msgs"
    _acp_request "session/new" \
        "$(jq -cn --arg cwd "$TEST_DIR" '{"cwd":$cwd,"mcpServers":[],"_meta":{"wiki":"research"}}')" \
        >> "$local_msgs"
    raw=$(_acp_run "$local_msgs")
    rm -f "$local_msgs"
    sid=$(_sid_from "$raw")
    if [ -n "$sid" ]; then
        pass "session/new with wiki meta returns sessionId"
    else
        fail "session/new with wiki meta returns sessionId" "no sessionId"
    fi
}

# ── session/load (existing) ───────────────────────────────────────────────────

if [ -n "${_LIFECYCLE_SID:-}" ]; then
    _ACP_REQ_ID=0
    local_msgs=$(mktemp)
    _acp_request "initialize" \
        '{"protocolVersion":1,"clientInfo":{"name":"acp-test","version":"0.1.0"}}' \
        >> "$local_msgs"
    _acp_request "session/load" \
        "$(jq -cn --arg sid "$_LIFECYCLE_SID" --arg cwd "$TEST_DIR" \
            '{"sessionId":$sid,"cwd":$cwd,"mcpServers":[]}')" \
        >> "$local_msgs"
    raw=$(_acp_run "$local_msgs")
    rm -f "$local_msgs"
    has_result=$(echo "$raw" | while IFS= read -r line; do
        [ -z "$line" ] && continue
        r=$(echo "$line" | jq -e 'has("result")' 2>/dev/null)
        [ "$r" = "true" ] && echo "yes" && break
    done | head -1)
    if [ "$has_result" = "yes" ]; then
        pass "session/load existing session succeeds"
    else
        fail "session/load existing session succeeds" "no result in response"
        echo "    raw: $(echo "$raw" | head -c 300)"
    fi
else
    skip "session/load existing session" "no session id from session/new"
fi

# ── session/load (unknown) ────────────────────────────────────────────────────

{
    _ACP_REQ_ID=0
    local_msgs=$(mktemp)
    _acp_request "initialize" \
        '{"protocolVersion":1,"clientInfo":{"name":"acp-test","version":"0.1.0"}}' \
        >> "$local_msgs"
    _acp_request "session/load" \
        "$(jq -cn --arg cwd "$TEST_DIR" \
            '{"sessionId":"session-does-not-exist","cwd":$cwd,"mcpServers":[]}')" \
        >> "$local_msgs"
    raw=$(_acp_run "$local_msgs")
    rm -f "$local_msgs"
    err=$(echo "$raw" | while IFS= read -r line; do
        [ -z "$line" ] && continue
        r=$(echo "$line" | jq -r '.error.message // empty' 2>/dev/null)
        [ -n "$r" ] && echo "$r" && break
    done | head -1)
    if echo "$err" | grep -q "not found"; then
        pass "session/load unknown session returns error"
    else
        fail "session/load unknown session returns error" "got: $err"
    fi
}

# ── session/list ──────────────────────────────────────────────────────────────

{
    _ACP_REQ_ID=0
    local_msgs=$(mktemp)
    _acp_request "initialize" \
        '{"protocolVersion":1,"clientInfo":{"name":"acp-test","version":"0.1.0"}}' \
        >> "$local_msgs"
    _acp_request "session/new" \
        "$(jq -cn --arg cwd "$TEST_DIR" '{"cwd":$cwd,"mcpServers":[]}')" \
        >> "$local_msgs"
    _acp_request "session/list" '{}' >> "$local_msgs"
    raw=$(_acp_run "$local_msgs")
    rm -f "$local_msgs"
    count=$(echo "$raw" | while IFS= read -r line; do
        [ -z "$line" ] && continue
        has=$(echo "$line" | jq -r 'if .result | has("sessions") then .result.sessions | length else empty end' 2>/dev/null)
        [ -n "$has" ] && echo "$has" && break
    done | head -1)
    if [ -n "$count" ] && [ "$count" -ge 1 ]; then
        pass "session/list returns at least one session"
    else
        fail "session/list returns at least one session" "count=$count"
        echo "    raw: $(echo "$raw" | head -c 400)"
    fi
}
