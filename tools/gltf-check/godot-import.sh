#!/usr/bin/env bash
# Imports every .glb file in a folder into a fresh Godot project, the way
# a game developer's Godot would, and fails if Godot reports an error or
# leaves a file unimported. Then opens each imported scene and checks it
# holds meshes, and that Godot turned the collision names into collision
# shapes (decision 0078), and the levels of detail into visibility ranges
# (decision 0155).
#
# Usage:
#   tools/gltf-check/godot-import.sh <godot program> <folder of .glb files>
#
# CI downloads a pinned Godot (MIT licence) from Godot's own releases and
# runs this in the `gltf-check` job (.github/workflows/ci.yml). Godot runs
# with --headless: no window and no graphics card needed.
set -euo pipefail

godot="$1"
source="$2"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project="$(mktemp -d)"
trap 'rm -rf "$project"' EXIT

shopt -s nullglob
files=("$source"/*.glb)
if [ "${#files[@]}" -eq 0 ]; then
    echo "::error::No .glb files in $source: the export step wrote nothing."
    exit 1
fi

# Godot's log without its colour codes, so its lines can be searched.
plain() { sed 's/\x1b\[[0-9;]*m//g'; }

# The smallest project Godot will open: a project file and the models.
cat >"$project/project.godot" <<'GODOT'
config_version=5

[application]
config/name="glTF import check"
GODOT
mkdir "$project/models"
cp "${files[@]}" "$project/models/"
cp "$here/check-scenes.gd" "$project/"
# Levels of detail (decision 0155) are imported with the post-import
# script the manual gives, as a game developer would set it up.
cp "$here/lod_ranges.gd" "$project/"
if [ -f "$project/models/godot-lods.glb" ]; then
    cat >"$project/models/godot-lods.glb.import" <<'IMPORT'
[remap]

importer="scene"

[params]

import_script/path="res://lod_ranges.gd"
IMPORT
fi

# Import everything, then quit.
log="$project/import.log"
status=0
"$godot" --headless --path "$project" --import >"$log.raw" 2>&1 || status=$?
plain <"$log.raw" >"$log"
cat "$log"
failed=0
if [ "$status" -ne 0 ]; then
    echo "::error::Godot's import stopped with exit code $status."
    failed=1
fi
# Godot keeps going after most errors, so its log is read as well.
if grep -E "^(ERROR|SCRIPT ERROR):" "$log" >/dev/null; then
    grep -E "^(ERROR|SCRIPT ERROR):" "$log" | head -n 20 | while IFS= read -r line; do
        echo "::error title=Godot import::$line"
    done || true
    failed=1
fi
# Every file has its import record, pointing at an imported scene.
for file in "${files[@]}"; do
    name="$(basename "$file")"
    record="$project/models/$name.import"
    if [ ! -f "$record" ] || grep -q '^valid=false' "$record"; then
        echo "::error title=Godot import::$name was not imported."
        failed=1
    fi
done
if [ "$failed" -ne 0 ]; then
    exit 1
fi

# Open each imported scene and look inside.
"$godot" --headless --path "$project" --script res://check-scenes.gd 2>&1 | plain | tee "$log"
if grep -E "^(ERROR|SCRIPT ERROR):|^FAILED" "$log" >/dev/null; then
    echo "::error title=Godot import::The imported scenes aren't what was exported (see the log)."
    exit 1
fi
if ! grep -q "^All scenes checked" "$log"; then
    echo "::error title=Godot import::The scene check didn't finish."
    exit 1
fi
echo "Godot imported all ${#files[@]} files."
