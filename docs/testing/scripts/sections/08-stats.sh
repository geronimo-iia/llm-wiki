#!/usr/bin/env bash
section "8. Stats"

run      "stats returns output"    "research"  $CLI stats
run_json "stats json has pages"    '.pages > 0' "true" \
         $CLI stats --format json
run_json "stats communities present (threshold=5)" '.communities != null' "true" \
         $CLI stats --format json
run_json "stats has diameter field (null or number)" \
         '.diameter == null or (.diameter | type) == "number"' "true" \
         $CLI stats --format json
run_json "stats has radius field (null or number)" \
         '.radius == null or (.radius | type) == "number"' "true" \
         $CLI stats --format json
run_json "stats has center field (array)" \
         '.center | type' "array" \
         $CLI stats --format json
run_json "stats has structural_note field (null or string)" \
         '.structural_note == null or (.structural_note | type) == "string"' "true" \
         $CLI stats --format json
