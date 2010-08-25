#!/bin/bash
# Prints some status data from the Large Hadron Collider.

set -e -u

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
cd "$TMP_DIR"

if ! wget --quiet --tries=1 --timeout=5 http://vistar-capture.web.cern.ch/vistar-capture/lhc1.png; then
    echo 'LHC data is currently unavailable.'
    exit 0
fi
convert lhc1.png -negate lhc1.png
convert lhc1.png -crop 1016x54+4+38 -monochrome -scale '200%' title.png
convert lhc1.png -crop 148x26+533+5 beam_energy.png
convert lhc1.png -crop 509x173+2+557 -scale '200%' comments.png

TITLE=$(gocr -d 0 -C 'A-Z0-9_:;,./--' -s 25 -i title.png | head -n 1)
ENERGY=$(gocr -d 0 -C '0-9MGeV' -i beam_energy.png | grep '^[0-9]\+ [a-zA-Z]\+$' | head -n 1)
COMMENTS=$(gocr -d 0 -C 'ABCDEFGHJKLMNOPQRSTUVWXYZa-z0-9_:;,./--' -s 23 -i comments.png)
COMMENTS="${COMMENTS//$'\n'/, }"
COMMENTS=$(printf '%s' "$COMMENTS" | sed 's/\(, \)\{2,\}/. /g')

BEAM_PRESENCE1=$(convert lhc1.png -crop 58x20+870+647 -negate - | pngtopnm | gocr -i -)
BEAM_PRESENCE2=$(convert lhc1.png -crop 58x20+942+647 -negate - | pngtopnm | gocr -i -)
STABLE_BEAM1=$(convert lhc1.png -crop 58x20+870+705 -negate - | pngtopnm | gocr -i -)
STABLE_BEAM2=$(convert lhc1.png -crop 58x20+942+705 -negate - | pngtopnm | gocr -i -)

if [[ $BEAM_PRESENCE1 = true && $BEAM_PRESENCE2 = true ]]; then
    if [[ $STABLE_BEAM1 = true && $STABLE_BEAM2 = true ]]; then
        printf '%s. Beam energy (stable beams): %s. %s\n' "$TITLE" "$ENERGY" "$COMMENTS"
    else
        printf '%s. Beam energy: %s. %s\n' "$TITLE" "$ENERGY" "$COMMENTS"
    fi
else
    printf '%s. No beam. %s\n' "$TITLE" "$COMMENTS"
fi

exit 0
