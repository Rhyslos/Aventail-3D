#!/usr/bin/env bash
# Window smoke test: opens the real app window on a virtual screen, takes a
# screenshot, checks the 3D viewport (background and the test cube), switches to
# Polygons mode with a click, quits with Ctrl+Q (or File > Quit as a
# fallback), and checks that the panel layout was saved.
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
import -window root "$OUT/window.png"

# Colour of one screen pixel, as "R G B".
pixel() {
    convert "$OUT/window.png" -format "%[fx:int(255*p{$1,$2}.r)] %[fx:int(255*p{$1,$2}.g)] %[fx:int(255*p{$1,$2}.b)]" info:
}

# 1. The 3D view is drawn. With the default layout on a 1440x900 screen,
#    (300, 130) is background near the top of the viewport and the middle
#    of the viewport (566, 416) is the test cube. A wrong colour-space
#    setting shows up as near-black or washed-out background.
read -r R G B <<<"$(pixel 300 130)"
echo "Viewport background at (300, 130): $R $G $B (expected a neutral mid grey)"
[ "$R" -ge 50 ] && [ "$R" -le 120 ] || fail "viewport background is not a mid grey: ($R, $G, $B)"
diff_rb=$(( R > B ? R - B : B - R ))
[ "$diff_rb" -le 15 ] || fail "viewport background is tinted: ($R, $G, $B)"
read -r CR CG CB <<<"$(pixel 566 416)"
echo "Viewport middle (the test cube) at (566, 416): $CR $CG $CB"
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

# 3. Quit with Ctrl+Q, or File > Quit if the key did not arrive.
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

# 4. The panel layout was saved for next time.
SAVED="$(grep -rl "panel_layout" "$TEST_HOME" 2>/dev/null | head -1 || true)"
[ -n "$SAVED" ] || fail "the panel layout was not saved"
echo "Layout saved in: ${SAVED#"$TEST_HOME"/}"

echo "Window smoke test passed."
