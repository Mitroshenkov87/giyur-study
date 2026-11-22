#!/usr/bin/env bash
# Render procedural objects with Blender headless, then convert to WebP.
#   obj_*.py     -> assets/img/objects/<name>.webp            (one render)
#   emblem_*.py  -> assets/img/emblems/<name>-{wood,leather}  (two themes)
#   icon_*.py    -> assets/img/icons/<name>-{wood,leather}[-hover]  (two themes x rest/hover)
#   decor_*.py   -> assets/img/decor/<name>-{wood,leather}
# Usage: SAMPLES=32 JOBS=3 ./render_all.sh [script-name ...]
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$ROOT/scripts"
IMG="$ROOT/../img"
BLENDER="${BLENDER:-/home/box/.local/bin/blender}"
SAMPLES="${SAMPLES:-96}"
JOBS="${JOBS:-2}"
RES="${RES:-}"

if [ "$#" -gt 0 ]; then
    files=()
    for a in "$@"; do
        a="$(basename "$a" .py)"
        files+=("$SCRIPTS_DIR/$a.py")
    done
else
    mapfile -t files < <(find "$SCRIPTS_DIR" -maxdepth 1 \( -name 'obj_*.py' -o -name 'icon_*.py' -o -name 'emblem_*.py' -o -name 'decor_*.py' \) | sort)
fi

jobs_list=()
for f in "${files[@]}"; do
    b="$(basename "$f" .py)"
    case "$b" in
        obj_*)    jobs_list+=("$f|$IMG/objects|") ;;
        emblem_*) for t in wood leather; do jobs_list+=("$f|$IMG/emblems|--theme $t"); done ;;
        decor_*)  for t in wood leather; do jobs_list+=("$f|$IMG/decor|--theme $t"); done ;;
        icon_*)   for t in wood leather; do
                      jobs_list+=("$f|$IMG/icons|--theme $t")
                      jobs_list+=("$f|$IMG/icons|--theme $t --hover")
                  done ;;
    esac
done

run_one() {
    IFS='|' read -r f out extra <<<"$1"
    mkdir -p "$out"
    echo "== $(basename "$f") $extra"
    # shellcheck disable=SC2086
    "$BLENDER" -b --factory-startup -P "$f" -- --out "$out" --samples "$SAMPLES" ${RES:+--res $RES} $extra \
        > "/tmp/blender-$(basename "$f" .py)-$(echo "$extra" | tr -cd 'a-z').log" 2>&1 \
        || { echo "!! FAILED $(basename "$f") $extra"; tail -5 "/tmp/blender-$(basename "$f" .py)-$(echo "$extra" | tr -cd 'a-z').log"; }
}
export -f run_one
export BLENDER SAMPLES RES
printf '%s\n' "${jobs_list[@]}" | xargs -P "$JOBS" -I{} bash -c 'run_one "$@"' _ {}

for d in objects emblems icons decor; do
    [ -d "$IMG/$d" ] && python3 "$ROOT/tools/to_webp.py" "$IMG/$d"
done
echo "done"
