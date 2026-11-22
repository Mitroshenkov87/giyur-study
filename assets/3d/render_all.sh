#!/usr/bin/env bash
# Render every scripts/obj_*.py (or the given names) with Blender headless,
# output RGBA PNGs to assets/img/objects/, then convert to webp.
# Usage: SAMPLES=16 ./render_all.sh [name ...]   (name = menorah_temple | obj_menorah_temple.py | path)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$ROOT/scripts"
OUT_DIR="$ROOT/../img/objects"
BLENDER="/home/box/.local/bin/blender"
SAMPLES="${SAMPLES:-96}"

mkdir -p "$OUT_DIR"

if [ "$#" -gt 0 ]; then
    files=()
    for a in "$@"; do
        case "$a" in
            /*.py) files+=("$a") ;;
            *.py) files+=("$SCRIPTS_DIR/$a") ;;
            *) files+=("$SCRIPTS_DIR/obj_$a.py") ;;
        esac
    done
else
    mapfile -t files < <(find "$SCRIPTS_DIR" -maxdepth 1 -name 'obj_*.py' | sort)
fi

for f in "${files[@]}"; do
    name="$(basename "$f" .py)"
    name="${name#obj_}"
    echo "== rendering $name (samples=$SAMPLES) =="
    "$BLENDER" -b --factory-startup -P "$f" -- --out "$OUT_DIR" --samples "$SAMPLES"
done

python3 "$ROOT/tools/to_webp.py" "$OUT_DIR"
echo "done -> $OUT_DIR"
