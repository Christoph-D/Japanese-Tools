#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script finds audio files for Japanese words.
#

. "$(dirname "$0")"/../gettext/gettext.sh

set -u

if [[ $# -ne 2 && $# -ne 1 ]]; then
    printf "$(gettext 'Usage: %s word [reading]')\n" "$(basename "$0")"
    exit 0
fi

# md5sum of the "audio missing" file.
NOT_FOUND=7e2c2f954ef6051373ba916f000168dc

# URL encoding.
encode_query() {
    # Escape single quotes for use in perl
    local ENCODED_QUERY=${1//\'/\\\'}
    ENCODED_QUERY=$(perl -MURI::Escape -e "print uri_escape('$ENCODED_QUERY');")
    printf '%s\n' "$ENCODED_QUERY"
}
# Returns 0 if $1 does not contain characters not present in English.
is_english() {
    printf '%s' "$1" | grep -q '^[][/'"'"' a-zA-Z0-9.()]*$'
}
to_katakana() {
    printf '%s\n' "$1" | \
        mecab --node-format='%f[7]' --eos-format= --unk-format='%m'
}
to_hiragana() {
    # We need kakasi to convert to Hiragana. mecab only prints
    # Katakana.
    printf '%s\n' "$1" | \
        mecab --node-format='%f[7]' --eos-format= --unk-format='%m' | \
        iconv -s -c -f utf-8 -t sjis | \
        kakasi -isjis -osjis -KH -HH -JH | \
        iconv -s -c -f sjis -t utf-8
}
# Takes $KANJI as input and sets $READING, $KANJI and $WORD
# appropriately.
figure_out_reading() {
    READING=$(to_hiragana "$KANJI")
    local KATAKANA_READING=$(to_katakana "$KANJI")
    if [[ $READING = ${KANJI// /} ]]; then
        # Try to find a Kanji variant for this word.
        KANJI=$("$(dirname "$0")"/../jmdict/ja "$KANJI" | head -n 1 | sed 's/^\([^ ,]*\).*$/\1/')
        # If it didn't work, go with the kana variant.
        if [[ $KANJI = 'Unknown' || $(to_hiragana "$KANJI") = $KANJI ]]; then
            KANJI="$READING"
            WORD="$READING"
        fi
    elif [[ $KATAKANA_READING = ${KANJI// /} ]]; then
        READING="$KATAKANA_READING"
        WORD="$KATAKANA_READING"
    fi
}

QUERY="$*"
# Split query into kanji and reading part.
KANJI=$(printf '%s' "$QUERY" | \
    sed 's#[ 　/／・[［「【〈『《].*##' | \
    sed 's#[　 ]##g')
READING=$(printf '%s' "$QUERY" | \
    sed 's#^[^ 　/／・[［「【〈『《]*.\(.*\)$#\1#' | \
    sed 's#[] 　／・［「【〈『《］」】〉』》]##g')

# Grab the whole command line if it is all in English.
if is_english "$QUERY"; then
    KANJI="$QUERY"
    READING=
fi

if [[ ! $KANJI ]]; then
    echo "$(gettext "Please provide a word.")"
    exit 0
fi

if [[ ! $READING ]]; then
    figure_out_reading
fi

# If the reading appears to be English, we need to fix it.
if is_english "$READING"; then
    figure_out_reading
fi

WORD="${WORD:-$KANJI [$READING]}"
KANJI="$(encode_query "$KANJI")"
READING="$(encode_query "$READING")"

URL="http://assets.languagepod101.com/dictionary/japanese/audiomp3.php?kana=$READING&kanji=$KANJI"

# Only print the URL if the audio file exists.
if [[ $(wget -q "$URL" -O - | md5sum | cut -f 1 -d ' ') = $NOT_FOUND ]]; then
    if [[ $WORD ]]; then
        printf "$(gettext 'No audio file available for %s.')\n" "$WORD"
    else
        printf "$(gettext 'No audio file available.')\n"
    fi
    exit 0
fi

# Try generating the tinyurl link. If it fails, print the long URL.
TINY=$(wget 'http://tinyurl.com/api-create.php?url='"$(encode_query "$URL")" \
         --quiet -O - --timeout=5 --tries=1)
printf "$(gettext 'Audio for %s: %s')\n" "$WORD" "${TINY:-$URL}"

exit 0
