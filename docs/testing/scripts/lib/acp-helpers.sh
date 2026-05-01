#!/usr/bin/env bash
# acp-helpers.sh — shared helpers for ACP validation scripts
# Do not execute directly. Source after helpers.sh.
#
# Protocol: JSON-RPC 2.0 NDJSON over stdio (one server process per session).
# Method names:
#   initialize      session/new     session/load    session/list
#   session/prompt  session/cancel  (notification: cancel)
#
# Requires:
#   ACP_SERVER (array) — command to launch the ACP server via stdio
#   Set by validate-acp.sh: ACP_SERVER=("$CLI" --config "$CONFIG" serve --acp)
#
# Each acp_* helper spawns a short-lived co-process, sends NDJSON messages,
# reads responses, then kills the server. This mirrors how mcptools works for MCP.
#
# Global state written by acp_new_session: ACP_SESSION_ID

_ACP_REQ_ID=0

# ── Low-level wire helpers ────────────────────────────────────────────────────

# _acp_request <method> <params_json> — build a JSON-RPC request line
# Increments _ACP_REQ_ID in-place (no subshell — direct arithmetic assignment).
_acp_request() {
    local method="$1" params="$2"
    _ACP_REQ_ID=$((_ACP_REQ_ID + 1))
    jq -cn --argjson id "$_ACP_REQ_ID" --arg method "$method" --argjson params "$params" \
        '{"jsonrpc":"2.0","id":$id,"method":$method,"params":$params}'
}

# _acp_notification <method> <params_json> — build a JSON-RPC notification line (no id)
_acp_notification() {
    local method="$1" params="$2"
    jq -cn --arg method "$method" --argjson params "$params" \
        '{"jsonrpc":"2.0","method":$method,"params":$params}'
}

# _acp_session_exchange <messages_file> — feed messages to server, collect responses until
# a "session/prompt" response (or error) arrives. Returns concatenated JSON lines.
# Usage: echo '{"jsonrpc":...}' > /tmp/msgs; _acp_session_exchange /tmp/msgs
_acp_session_exchange() {
    local msgs_file="$1"
    local tmpout
    tmpout=$(mktemp)

    # Spawn server, pipe messages to stdin, read stdout
    "${ACP_SERVER[@]}" < "$msgs_file" > "$tmpout" 2>/dev/null &
    local pid=$!

    # Give server time to process and stream responses (workflows can take a moment)
    local deadline=$((SECONDS + ACP_TIMEOUT))
    local done=0
    while [ $SECONDS -lt $deadline ] && [ $done -eq 0 ]; do
        sleep 0.2
        # Stop when we see a response to the highest-id request (the prompt)
        if grep -q '"id":'"$_ACP_REQ_ID" "$tmpout" 2>/dev/null; then
            done=1
        fi
    done

    kill "$pid" 2>/dev/null
    wait "$pid" 2>/dev/null

    cat "$tmpout"
    rm -f "$tmpout"
}

# ── Session helper ────────────────────────────────────────────────────────────

# _acp_run <msgs_file> — send msgs, get all output; separate from exchange for reuse
_acp_run() {
    local msgs_file="$1"
    local tmpout
    tmpout=$(mktemp)
    "${ACP_SERVER[@]}" < "$msgs_file" > "$tmpout" 2>/dev/null &
    local pid=$!
    local deadline=$((SECONDS + ACP_TIMEOUT))
    while [ $SECONDS -lt $deadline ]; do
        sleep 0.2
        if grep -q '"id":'"$_ACP_REQ_ID" "$tmpout" 2>/dev/null; then
            break
        fi
    done
    kill "$pid" 2>/dev/null
    wait "$pid" 2>/dev/null
    cat "$tmpout"
    rm -f "$tmpout"
}

# ── High-level helpers ────────────────────────────────────────────────────────

# _acp_msgs_init_session [wiki_name] — write init + new_session messages to a tmp file
# Sets ACP_MSGS_FILE (caller must rm -f it)
_acp_msgs_init_session() {
    local wiki="${1:-}"
    ACP_MSGS_FILE=$(mktemp)
    _ACP_REQ_ID=0

    local meta='null'
    [ -n "$wiki" ] && meta=$(jq -cn --arg w "$wiki" '{"wiki":$w}')

    _acp_request "initialize" \
        '{"protocolVersion":1,"clientInfo":{"name":"acp-test","version":"0.1.0"}}' \
        >> "$ACP_MSGS_FILE"
    _acp_request "session/new" \
        "$(jq -cn --arg cwd "$TEST_DIR" --argjson meta "$meta" \
            '{"cwd":$cwd,"mcpServers":[],"_meta":$meta}')" \
        >> "$ACP_MSGS_FILE"
}

# _acp_session_id_from <output> — extract sessionId from session/new result line
_acp_session_id_from() {
    echo "$1" | while IFS= read -r line; do
        [ -z "$line" ] && continue
        r=$(echo "$line" | jq -r '.result.sessionId // empty' 2>/dev/null)
        [ -n "$r" ] && echo "$r" && break
    done | head -1
}

# run_acp <desc> <pattern> <prompt> [wiki]
# Full round-trip: init → new_session → prompt; checks output text matches pattern.
run_acp() {
    local desc="$1" pattern="$2" prompt="$3" wiki="${4:-}"
    _acp_msgs_init_session "$wiki"

    # Need session id from new_session response — do a two-phase send
    local init_out
    local init_msgs
    init_msgs=$(mktemp)
    local saved_id=$_ACP_REQ_ID
    cp "$ACP_MSGS_FILE" "$init_msgs"
    rm -f "$ACP_MSGS_FILE"

    init_out=$(_acp_run "$init_msgs")
    rm -f "$init_msgs"

    local sid
    sid=$(_acp_session_id_from "$init_out")
    if [ -z "$sid" ]; then
        fail "$desc" "could not obtain sessionId from new_session response"
        return
    fi
    ACP_SESSION_ID="$sid"

    # Now send prompt
    local prompt_msgs
    prompt_msgs=$(mktemp)
    _ACP_REQ_ID=$saved_id

    _acp_request "session/prompt" \
        "$(jq -cn --arg sid "$sid" --arg text "$prompt" \
            '{"sessionId":$sid,"prompt":[{"type":"text","text":$text}]}')" \
        >> "$prompt_msgs"

    local out
    out=$(_acp_run "$prompt_msgs")
    rm -f "$prompt_msgs"

    # Collect all text from session/update notifications (agent_message_chunk and tool_call)
    local text
    text=$(echo "$init_out"$'\n'"$out" | \
        jq -r 'select(.method == "session/update") |
               .params.update |
               if .sessionUpdate == "agent_message_chunk" then .content.text // empty
               elif .sessionUpdate == "tool_call" then (.title // empty)
               elif .sessionUpdate == "tool_call_update" then
                 (.fields.content // [] | .[].text // empty)
               else empty end' 2>/dev/null | tr '\n' ' ')

    if [ -z "$pattern" ] || echo "$text" | grep -q "$pattern"; then
        pass "$desc"
    else
        fail "$desc" "output did not match: $pattern"
        echo "    text: $(echo "$text" | head -c 200)"
    fi
}

# run_acp_json <desc> <jq_filter> <expected> <prompt> [wiki]
# Like run_acp but applies jq filter to the final prompt response.
run_acp_json() {
    local desc="$1" filter="$2" expected="$3" prompt="$4" wiki="${5:-}"
    _acp_msgs_init_session "$wiki"

    local init_msgs
    init_msgs=$(mktemp)
    local saved_id=$_ACP_REQ_ID
    cp "$ACP_MSGS_FILE" "$init_msgs"
    rm -f "$ACP_MSGS_FILE"

    local init_out
    init_out=$(_acp_run "$init_msgs")
    rm -f "$init_msgs"

    local sid
    sid=$(_acp_session_id_from "$init_out")
    if [ -z "$sid" ]; then
        fail "$desc" "could not obtain sessionId"
        return
    fi

    local prompt_msgs
    prompt_msgs=$(mktemp)
    _ACP_REQ_ID=$saved_id

    _acp_request "session/prompt" \
        "$(jq -cn --arg sid "$sid" --arg text "$prompt" \
            '{"sessionId":$sid,"prompt":[{"type":"text","text":$text}]}')" \
        >> "$prompt_msgs"

    local out
    out=$(_acp_run "$prompt_msgs")
    rm -f "$prompt_msgs"

    # Apply jq filter to each NDJSON line, take first non-empty match
    local actual
    actual=$(echo "$out" | while IFS= read -r line; do
        [ -z "$line" ] && continue
        r=$(echo "$line" | jq -r "$filter" 2>/dev/null)
        [ -n "$r" ] && [ "$r" != "null" ] && echo "$r" && break
    done | head -1)
    if [ "$actual" = "$expected" ]; then
        pass "$desc"
    else
        fail "$desc" "expected '$expected', got '$actual'"
    fi
}

# run_acp_error <desc> <error_pattern> <prompt> [wiki]
# Expects an error response.
run_acp_error() {
    local desc="$1" pattern="$2" prompt="$3" wiki="${4:-}"
    _acp_msgs_init_session "$wiki"

    local init_msgs
    init_msgs=$(mktemp)
    local saved_id=$_ACP_REQ_ID
    cp "$ACP_MSGS_FILE" "$init_msgs"
    rm -f "$ACP_MSGS_FILE"

    local init_out
    init_out=$(_acp_run "$init_msgs")
    rm -f "$init_msgs"

    local sid
    sid=$(_acp_session_id_from "$init_out")
    if [ -z "$sid" ]; then
        fail "$desc" "could not obtain sessionId"
        return
    fi

    local prompt_msgs
    prompt_msgs=$(mktemp)
    _ACP_REQ_ID=$saved_id

    _acp_request "session/prompt" \
        "$(jq -cn --arg sid "$sid" --arg text "$prompt" \
            '{"sessionId":$sid,"prompt":[{"type":"text","text":$text}]}')" \
        >> "$prompt_msgs"

    local out
    out=$(_acp_run "$prompt_msgs")
    rm -f "$prompt_msgs"

    local err_msg
    err_msg=$(echo "$out" | while IFS= read -r line; do
        [ -z "$line" ] && continue
        r=$(echo "$line" | jq -r '.error.message // empty' 2>/dev/null)
        [ -n "$r" ] && echo "$r" && break
    done | head -1)
    if echo "$err_msg" | grep -q "$pattern"; then
        pass "$desc"
    else
        fail "$desc" "expected error matching '$pattern', got: $err_msg"
    fi
}

# acp_new_session_raw [wiki] — sends init+new_session, sets ACP_SESSION_ID + ACP_INIT_OUT
acp_new_session_raw() {
    local wiki="${1:-}"
    _acp_msgs_init_session "$wiki"
    local init_msgs
    init_msgs="$ACP_MSGS_FILE"
    ACP_INIT_OUT=$(_acp_run "$init_msgs")
    rm -f "$init_msgs"
    ACP_SESSION_ID=$(_acp_session_id_from "$ACP_INIT_OUT")
}
