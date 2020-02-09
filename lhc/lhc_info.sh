#!/usr/bin/env bash
# Prints some status data from the Large Hadron Collider.

set -eu
shopt -s extglob

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
cd "$TMP_DIR"

if ! command -v tesseract &> /dev/null; then
    echo "Please install tesseract."
    if [[ $(lsb_release --short --id) == Ubuntu ]]; then
        echo "You can install it with: sudo apt install tesseract-ocr"
    fi
    exit 1
fi

if [[ $# = 1 && $1 = 'help' ]]; then
    echo "See http://op-webtools.web.cern.ch/op-webtools/vistar/vistars.php?usr=LHC1"
    exit 0
fi

if ! wget --quiet --tries=1 --timeout=5 \
     'http://vistar-capture.web.cern.ch/vistar-capture/lhc1.png'; then
    echo 'LHC data is currently unavailable.'
    exit 0
fi

ocr() {
    convert lhc1.png "$@" tmp.tif &> /dev/null
    tesseract tmp.tif tmp &> /dev/null
    local result
    result=$(cat tmp.txt 2>/dev/null)
    # Remove newlines and trim spaces.
    result="${result//,$'\n'/,}"
    result="${result//$'\n'/ }"
    result="${result##*( )}"
    result="${result%%*( )}"
    printf '%s' "$result"
}

TITLE=$(ocr -crop 1016x54+4+38)
ENERGY=$(ocr -crop 190x45+140+104)
COMMENTS=$(ocr -crop 509x173+2+557 -scale 150%)
COMMENTS=$(printf '%s' "$COMMENTS" | sed 's/\(, \)\{2,\}/. /g;s/ \+/ /g')

BEAM_PRESENCE1=$(ocr -crop 58x20+870+647)
BEAM_PRESENCE2=$(ocr -crop 58x20+942+647)
STABLE_BEAM1=$(ocr -crop 58x20+870+705)
STABLE_BEAM2=$(ocr -crop 58x20+942+705)

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
