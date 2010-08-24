#!/bin/bash
# Prints some status data from the Large Hadron Collider.

set -e -u

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
cd "$TMP_DIR"

wget --quiet http://vistar-capture.web.cern.ch/vistar-capture/lhc1.png
convert lhc1.png -negate lhc1.png
convert lhc1.png -crop 1016x54+4+38 -monochrome -scale '200%' title.png
convert lhc1.png -crop 192x43+142+108 beam_energy.png
convert lhc1.png -crop 509x173+2+557 comments.png

TITLE=$(gocr -d 0 -C 'A-Z0-9_:;,./--' -s 25 -i title.png)
ENERGY=$(gocr -d 0 -C '0-9MGeV' -i beam_energy.png)
COMMENTS=$(gocr -d 0 -C 'a-z0-9_:;,./--' -s 11 -i comments.png)
COMMENTS="${COMMENTS//$'\n'/, }"

printf '%s. Beam energy: %s. %s\n' "$TITLE" "$ENERGY" "$COMMENTS"
exit 0
