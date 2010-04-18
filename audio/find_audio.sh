#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script finds audio files for Japanese words.
#

set -u

if [[ $# -ne 2 && $# -ne 1 ]]; then
    echo "Usage: $(basename "$0") word [reading]"
    exit 0
fi

# md5sum of the "audio missing" file.
NOT_FOUND=7e2c2f954ef6051373ba916f000168dc

# Make sure we have a UTF-8 environment.
export LANG=en_US.UTF-8

# URL encoding.
encode_query() {
    # Escape single quotes for use in perl
    local ENCODED_QUERY=${1//\'/\\\'}
    ENCODED_QUERY=$(perl -MURI::Escape -e "print uri_escape('$ENCODED_QUERY');")
    printf '%s\n' "$ENCODED_QUERY"
}
find_reading() {
    # We need kakasi to convert to Hiragana. mecab only prints
    # Katakana.
    printf '%s\n' "$1" | \
        mecab --node-format='%f[7]' --eos-format= --unk-format='%m' | \
        iconv -s -c -f utf-8 -t sjis | \
        kakasi -isjis -osjis -KH -HH -JH | \
        iconv -s -c -f sjis -t utf-8
}

QUERY="$*"
# Split query into kanji and reading part.
KANJI=$(printf '%s' "$QUERY" | \
    sed 's#[ 　/／・[［「【〈『《].*##' | \
    sed 's#[　 ]##g')
READING=$(printf '%s' "$QUERY" | \
    sed 's#^[^ 　/／・[［「【〈『《]*.\(.*\)$#\1#' | \
    sed 's#[] 　／・［「【〈『《］」】〉』》]##g')

if [[ ! $KANJI ]]; then
    echo "Please provide a word."
    exit 0
fi

if [[ ! $READING ]]; then
    READING=$(find_reading "$KANJI")
    if [[ $READING = $KANJI ]]; then
        # Try to find a Kanji variant for this word.
        KANJI=$("$(dirname "$0")"/../jmdict/ja "$READING" | head -n 1 | cut -f 1 -d ' ')
        # If it didn't work, go with the kana variant.
        if [[ $KANJI = 'Unknown' || $(find_reading "$KANJI") = $KANJI ]]; then
            KANJI="$READING"
            WORD="$READING"
        fi
    fi
fi

WORD="${WORD:-$KANJI [$READING]}"
KANJI="$(encode_query "$KANJI")"
READING="$(encode_query "$READING")"

URL="http://assets.languagepod101.com/dictionary/japanese/audiomp3.php?kana=$READING&kanji=$KANJI"

# Only print the URL if the audio file exists.
if [[ $(wget -q "$URL" -O - | md5sum | cut -f 1 -d ' ') = $NOT_FOUND ]]; then
    printf 'No audio file available%s.\n' "${WORD:+ for $WORD}"
    exit 0
fi

# Try generating the tinyurl link. If it fails, print the long URL.
TINY=$(wget 'http://tinyurl.com/api-create.php?url='"$(encode_query "$URL")" \
         --quiet -O - --timeout=5 --tries=1)
printf 'Audio for %s: %s\n' "$WORD" "${TINY:-$URL}"

exit 0
