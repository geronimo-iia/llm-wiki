#!/usr/bin/env bash
section "8. Session Cap (ACP)"

# Session cap enforcement requires sessions to arrive with distinct timestamps
# (server uses timestamp_millis for session IDs — concurrent requests collide).
# Reliable test requires manual pacing; see docs/testing/validate-acp.md.
#
# setup-test-env.sh sets serve.acp_max_sessions = 3 so manual testing uses a
# low cap without editing config by hand.
#
# This section verifies only that the config is in place.

if command -v "$CLI" &>/dev/null && [ -f "$CONFIG" ]; then
    cap=$("$CLI" --config "$CONFIG" config get serve.acp_max_sessions 2>/dev/null | tr -d '[:space:]')
    if [ -n "$cap" ] && [ "$cap" -lt 20 ]; then
        pass "serve.acp_max_sessions configured for manual cap test (cap=$cap)"
    else
        fail "serve.acp_max_sessions not set for cap test" \
             "expected <20, got: $cap — run setup-test-env.sh"
    fi
else
    skip "serve.acp_max_sessions check" "binary or config not found"
fi
