#!/usr/bin/env bash
# Window smoke test: opens the real app window on a virtual screen, takes a
# screenshot, checks the 3D viewport background, creates a cube with the
# command palette's cube button and checks it is drawn, switches to
# Polygons mode with a click, orbits the view with Alt + drag, zooms with the
# wheel, frames everything with H, steps the grid through its levels, quits with Ctrl+Q (or File > Quit
# as a fallback), and checks that the panel layout and view options were
# saved.
#
# Usage (needs Xvfb, xdotool, ImageMagick and a Vulkan driver such as Mesa's
# lavapipe):
#   xvfb-run -s "-screen 0 1440x900x24" tools/smoke/window-smoke.sh <binary> <window title> [output folder]
#
# The window title is the app's name (APP_NAME in crates/app/src/branding.rs);
# CI passes it in, so this script never names the app (see RENAMING.md).
# The screenshots land in the output folder so they can be looked at in CI.
set -euo pipefail

BIN="${1:?usage: window-smoke.sh <binary> <window title> [output folder]}"
TITLE="${2:?usage: window-smoke.sh <binary> <window title> [output folder]}"
OUT="${3:-smoke-output}"
mkdir -p "$OUT"
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"

# A throwaway home folder, so the test starts like a first launch and never
# touches real settings.
TEST_HOME="$(mktemp -d)"
export HOME="$TEST_HOME"
export XDG_CONFIG_HOME="$TEST_HOME/.config"
export XDG_DATA_HOME="$TEST_HOME/.local/share"

"$BIN" >"$OUT/app.log" 2>&1 </dev/null &
PID=$!

fail() {
    echo "SMOKE TEST FAILED: $1"
    echo "--- App output ---"
    cat "$OUT/app.log" || true
    if [ -n "${GITHUB_ACTIONS:-}" ]; then
        local msg
        msg="$1"$'\n'"Last lines of the app's output:"$'\n'"$(tail -n 25 "$OUT/app.log" 2>/dev/null)"
        msg="${msg//'%'/'%25'}"
        msg="${msg//$'\r'/'%0D'}"
        msg="${msg//$'\n'/'%0A'}"
        echo "::error title=Window smoke test::$msg"
    fi
    kill "$PID" 2>/dev/null || true
    exit 1
}

# Wait up to 60 seconds for the window to appear.
for _ in $(seq 1 60); do
    if xdotool search --name "$TITLE" >/dev/null 2>&1; then break; fi
    kill -0 "$PID" 2>/dev/null || fail "the app exited before its window appeared"
    sleep 1
done
WINDOW_ID="$(xdotool search --name "$TITLE" 2>/dev/null | head -1)" || true
[ -n "$WINDOW_ID" ] || fail "no window after 60 seconds"

# Let the first frames draw (software rendering can be slow), then take
# the screenshot.
sleep 6
import -window root "$OUT/empty.png"

# Colour of one screen pixel, as "R G B".
pixel() {
    convert "$OUT/${3:-window}.png" -format "%[fx:int(255*p{$1,$2}.r)] %[fx:int(255*p{$1,$2}.g)] %[fx:int(255*p{$1,$2}.b)]" info:
}

# 1. The 3D view is drawn. With the default layout on a 1440x900 screen,
#    (300, 130) is background near the top of the viewport. A wrong
#    colour-space setting shows up as near-black or washed-out background.
#    The scene starts empty; the cube button on the command palette
#    (CUBE_BUTTON_X, 50) creates a 2 m cube, which then fills the middle
#    of the viewport (566, 416).
read -r R G B <<<"$(pixel 300 130 empty)"
echo "Viewport background at (300, 130): $R $G $B (expected a neutral mid grey)"
[ "$R" -ge 50 ] && [ "$R" -le 120 ] || fail "viewport background is not a mid grey: ($R, $G, $B)"
diff_rb=$(( R > B ? R - B : B - R ))
[ "$diff_rb" -le 15 ] || fail "viewport background is tinted: ($R, $G, $B)"
xdotool windowfocus --sync "$WINDOW_ID" 2>/dev/null || true
CUBE_BUTTON_X="${CUBE_BUTTON_X:-396}"
xdotool mousemove "$CUBE_BUTTON_X" 50 click 1
sleep 2
# Move the mouse away so the button's tooltip doesn't cover the view.
xdotool mousemove 1300 860
sleep 1.5
import -window root "$OUT/window.png"
read -r CR CG CB <<<"$(pixel 566 416)"
echo "Viewport middle (the new cube) at (566, 416): $CR $CG $CB"
[ $(( CR + CG + CB )) -gt $(( R + G + B + 90 )) ] || fail "no lit cube in the middle of the viewport: ($CR, $CG, $CB)"
grep -q "can't be shown" "$OUT/app.log" && fail "the 3D view could not be shown"

# Number of pixels that differ between two screenshots.
changed_pixels() {
    { compare -metric AE "$1" "$2" null: 2>&1 || true; } | awk '{print int($1)}'
}

# 2. Clicking a mode in the left palette selects it: the button lights up
#    and the status bar changes. MODE_Y is the middle of the fifth button
#    (Polygons) with the standard layout.
xdotool windowfocus --sync "$WINDOW_ID" 2>/dev/null || true
MODE_Y="${MODE_Y:-234}"
xdotool mousemove 22 "$MODE_Y" click 1
sleep 1.5
import -window root "$OUT/after-mode.png"
MODE_CHANGE="$(changed_pixels "$OUT/window.png" "$OUT/after-mode.png")"
echo "Pixels changed by picking a mode: $MODE_CHANGE"
[ "$MODE_CHANGE" -gt 200 ] || fail "clicking the mode palette changed nothing"

# 3. Alt + left drag across the viewport orbits the view, so most of the
#    picture changes.
xdotool windowfocus --sync "$WINDOW_ID" 2>/dev/null || true
xdotool mousemove 600 300
sleep 0.3
xdotool keydown alt
sleep 0.2
xdotool mousedown 1
sleep 0.2
for i in $(seq 1 12); do
    xdotool mousemove $((600 + i * 15)) $((300 + i * 4))
    sleep 0.1
done
xdotool mouseup 1
sleep 0.2
xdotool keyup alt
sleep 1.5
import -window root "$OUT/after-orbit.png"
ORBIT_CHANGE="$(changed_pixels "$OUT/after-mode.png" "$OUT/after-orbit.png")"
echo "Pixels changed by orbiting: $ORBIT_CHANGE"
[ "$ORBIT_CHANGE" -gt 20000 ] || fail "Alt + drag did not orbit the view"

# 4. The mouse wheel zooms in.
xdotool mousemove 600 400
xdotool click 4
sleep 1.5
import -window root "$OUT/after-zoom.png"
ZOOM_CHANGE="$(changed_pixels "$OUT/after-orbit.png" "$OUT/after-zoom.png")"
echo "Pixels changed by zooming: $ZOOM_CHANGE"
[ "$ZOOM_CHANGE" -gt 20000 ] || fail "the mouse wheel did not zoom the view"

# 4b. H frames everything again after zooming in.
xdotool key --window "$WINDOW_ID" h || true
sleep 1.5
import -window root "$OUT/after-frame.png"
FRAME_CHANGE="$(changed_pixels "$OUT/after-zoom.png" "$OUT/after-frame.png")"
echo "Pixels changed by framing everything (H): $FRAME_CHANGE"
[ "$FRAME_CHANGE" -gt 20000 ] || fail "H did not frame the scene"

# 5. The grid detail button at the bottom of the left palette steps
#    through the grid levels: one click adds the 50 cm lines (each metre
#    square cut into four), a second the fine 10 cm lines. GRID_Y is the
#    middle of that button with the standard layout.
GRID_Y="${GRID_Y:-381}"
xdotool mousemove 22 "$GRID_Y" click 1
sleep 1.5
import -window root "$OUT/after-grid-half.png"
HALF_CHANGE="$(changed_pixels "$OUT/after-frame.png" "$OUT/after-grid-half.png")"
echo "Pixels changed by the 50 cm grid level: $HALF_CHANGE"
[ "$HALF_CHANGE" -gt 1000 ] || fail "the 50 cm grid level changed nothing"
xdotool mousemove 22 "$GRID_Y" click 1
sleep 1.5
import -window root "$OUT/after-grid.png"
GRID_CHANGE="$(changed_pixels "$OUT/after-grid-half.png" "$OUT/after-grid.png")"
echo "Pixels changed by the 10 cm grid level: $GRID_CHANGE"
[ "$GRID_CHANGE" -gt 5000 ] || fail "the 10 cm grid level changed nothing"

# 5b. F5 shows the four views (perspective, top, front, right); F1 goes
#     back to the perspective view alone.
xdotool mousemove 600 400
xdotool key --window "$WINDOW_ID" F5 || true
sleep 1.5
import -window root "$OUT/four-views.png"
FOUR_CHANGE="$(changed_pixels "$OUT/after-grid.png" "$OUT/four-views.png")"
echo "Pixels changed by showing four views (F5): $FOUR_CHANGE"
[ "$FOUR_CHANGE" -gt 20000 ] || fail "F5 did not show four views"
xdotool key --window "$WINDOW_ID" F1 || true
sleep 1.5
import -window root "$OUT/back-to-one.png"
ONE_CHANGE="$(changed_pixels "$OUT/four-views.png" "$OUT/back-to-one.png")"
echo "Pixels changed by going back to one view (F1): $ONE_CHANGE"
[ "$ONE_CHANGE" -gt 20000 ] || fail "F1 did not go back to one view"

# 5c. Deleting the cube in the Object Manager takes it out of the
#     view; Ctrl+Z brings it back. CUBE_ROW_Y is the middle of its row
#     with the standard layout.
CUBE_ROW_Y="${CUBE_ROW_Y:-631}"
xdotool mousemove 1200 "$CUBE_ROW_Y" click 1
sleep 1
xdotool key --window "$WINDOW_ID" Delete || true
sleep 1.5
import -window root "$OUT/cube-deleted.png"
DELETE_CHANGE="$(changed_pixels "$OUT/back-to-one.png" "$OUT/cube-deleted.png")"
echo "Pixels changed by deleting the cube: $DELETE_CHANGE"
[ "$DELETE_CHANGE" -gt 20000 ] || fail "deleting the cube changed nothing"
xdotool key --window "$WINDOW_ID" ctrl+z || true
sleep 1.5
import -window root "$OUT/cube-undone.png"
UNDO_CHANGE="$(changed_pixels "$OUT/cube-deleted.png" "$OUT/cube-undone.png")"
echo "Pixels changed by undoing (Ctrl+Z): $UNDO_CHANGE"
[ "$UNDO_CHANGE" -gt 20000 ] || fail "Ctrl+Z did not bring the cube back"

# 6. Quit with Ctrl+Q, or File > Quit if the key did not arrive.
xdotool mousemove 500 400
xdotool key --window "$WINDOW_ID" ctrl+q || true
for _ in $(seq 1 15); do
    kill -0 "$PID" 2>/dev/null || break
    sleep 1
done
if kill -0 "$PID" 2>/dev/null; then
    echo "Ctrl+Q did not close the window; trying File > Quit."
    xdotool mousemove 20 14 click 1
    sleep 1
    xdotool key Up Return
    for _ in $(seq 1 15); do
        kill -0 "$PID" 2>/dev/null || break
        sleep 1
    done
fi
kill -0 "$PID" 2>/dev/null && fail "the app did not quit"
wait "$PID" || fail "the app exited with an error"

# 7. The panel layout and the grid choice were saved for next time.
SAVED="$(grep -rl "panel_layout" "$TEST_HOME" 2>/dev/null | head -1 || true)"
[ -n "$SAVED" ] || fail "the panel layout was not saved"
echo "Layout saved in: ${SAVED#"$TEST_HOME"/}"
grep -q "grid_detail:TenCentimetres" "$SAVED" || fail "the grid detail choice was not saved"
echo "Grid detail choice saved."

echo "Window smoke test passed."
