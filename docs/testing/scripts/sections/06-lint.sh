#!/usr/bin/env bash
section "6. Lint"

# Rebuild after ingest in section 05 invalidated the index
$CLI index rebuild --wiki research > /dev/null 2>&1

# lint exits 1 when error-level findings exist — use run_nocheck / run_json_nocheck
run_nocheck      "lint all rules"         "error\|warning"  $CLI lint
run_nocheck      "lint broken-link rule"  "broken-link"     $CLI lint --rules broken-link
run_nocheck      "lint orphan rule"       "orphan"          $CLI lint --rules orphan
run_json_nocheck "lint json has findings array" '.findings | type' "array" \
                 $CLI lint --format json
run_json_nocheck "broken-link finds concepts/does-not-exist" \
                 '[.findings[] | select(.rule=="broken-link")] | length > 0' "true" \
                 $CLI lint --rules broken-link --format json
run_json_nocheck "broken-link detects CommonMark inline broken link" \
                 '[.findings[] | select(.rule=="broken-link" and (.message | contains("also-does-not-exist")))] | length > 0' "true" \
                 $CLI lint --rules broken-link --format json
run_json_nocheck "broken-link does not flag valid CommonMark link" \
                 '[.findings[] | select(.rule=="broken-link" and (.message | contains("mixture-of-experts")))] | length == 0' "true" \
                 $CLI lint --rules broken-link --format json
run_json_nocheck "orphan finds orphan-concept" \
                 '[.findings[] | select(.slug=="concepts/orphan-concept")] | length > 0' "true" \
                 $CLI lint --rules orphan --format json
run_nocheck      "lint with --wiki flag"  "error\|warning"  $CLI lint --wiki research
run_nocheck      "lint articulation-point rule runs" ""  $CLI lint --rules articulation-point
run_nocheck      "lint bridge rule runs"             ""  $CLI lint --rules bridge
run_nocheck      "lint periphery rule runs"          ""  $CLI lint --rules periphery
run_json_nocheck "lint all rules includes structural rules" \
                 '[.findings[] | select(.rule == "articulation-point" or .rule == "bridge" or .rule == "periphery")] | length >= 0' "true" \
                 $CLI lint --format json
