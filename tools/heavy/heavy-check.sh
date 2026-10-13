#!/usr/bin/env bash
# The heavy-mesh check (DESIGN.md §4.8, decisions 0033 and 0035): times
# what a person does with one object of millions of polygons against the
# targets, and prints a table for the pull request.
#
# Not part of the normal test run: run it at the end of every phase, and
# whenever the project lead asks. It builds in release mode (as the app
# ships) and takes a few minutes.
#
# Usage:
#   tools/heavy/heavy-check.sh                 # three million polygons
#   HEAVY_POLYGONS=500000 tools/heavy/heavy-check.sh
#
# It checks the interface (selecting, hovering, dragging, undo), the
# drawing geometry (made once, updated after a move) and project files
# (saving and opening), and how quickly edits are worked out (the history
# while dragging, bevelling many edges, finding edits again). Drawing
# speed on screen depends on the graphics card, so it is checked by trying
# Help > Add a heavy test shape in the app.
set -euo pipefail

export HEAVY_POLYGONS="${HEAVY_POLYGONS:-3000000}"
failed=0
run() {
    local out
    if ! out=$(cargo test --release --locked -q -p "$1" "$2" -- --ignored --nocapture --test-threads=1 2>&1); then
        failed=1
    fi
    grep -E "Heavy-mesh check|Edit-speed check|ms |MB|SLOW|panicked" <<<"$out" || true
}
run ui heavy_mesh_check
run render heavy_geometry_check
run fileio heavy_project_check
run scene edit_speed_check
if [ "$failed" -ne 0 ]; then
    echo "Some steps missed their targets (marked SLOW above), or a check failed."
    exit 1
fi
echo "Heavy-mesh check passed."
