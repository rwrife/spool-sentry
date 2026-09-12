#!/bin/sh
# Render the editable mechanical source to preview + per-part STLs (issue #3).
# Requires Docker and the pinned OpenSCAD image; run from the repo root.
# NOTE: outputs are review-only geometry — never printed or load-tested.
set -eu
IMAGE="${SCAD_IMAGE:-parts-tally-openscad:arm64}"
cd "$(dirname "$0")/.."
mkdir -p mechanical
for part in assembly base platform bracket tray stop; do
  docker run --rm -v "$PWD:/w" -w /w --user "$(id -u):$(id -g)" \
    -e HOME=/tmp/oscad-home "$IMAGE" \
    sh -c "openscad -D 'export_part=\"$part\"' -o mechanical/spool-sentry-$part.stl mechanical/spool-sentry-platform.scad" >&2
done
echo "exported mechanical/spool-sentry-{assembly,base,platform,bracket,tray,stop}.stl"
