#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script finds audio files for Japanese words.
#

. "$(dirname "$0")"/../gettext/gettext.sh

set -u

if [[ $# -ne 2 && $# -ne 1 ]]; then
    printf_ 'Usage: %s word [reading]' "$(basename "$0")"
    exit 0
fi

# md5sum of the "audio missing" file.
NOT_FOUND=7e2c2f954ef6051373ba916f000168dc

# URL encoding.
encode_query() {
    # Make the query safe: Remove backslashes and escape single quotes.
    local ENCODED_QUERY=${1//\\/}
    ENCODED_QUERY=${ENCODED_QUERY//\'/\\\'}
    ENCODED_QUERY=$(perl -MURI::Escape -e "print uri_escape('$ENCODED_QUERY');")
    printf '%s\n' "$ENCODED_QUERY"
}
# Returns 0 if $1 contains only English characters (no kanji etc.).
is_english() {
    printf '%s' "$1" | grep -q '^[][/'"'"' a-zA-Z0-9.()]*$'
}
to_katakana() {
    # We need kakasi to convert to katakana. mecab only prints
    # hiragana.
    printf '%s\n' "$1" | \
        mecab --node-format='%f[5]' --eos-format= --unk-format='%m' | \
        kakasi -iutf8 -outf8 -KK -HK -JK
}
to_hiragana() {
    printf '%s\n' "$1" | \
        mecab --node-format='%f[5]' --eos-format= --unk-format='%m'
}
# Takes $KANJI as input and sets $READING, $KANJI and $WORD
# appropriately.
figure_out_reading() {
    READING=$(to_hiragana "$KANJI")
    local KATAKANA_READING=$(to_katakana "$KANJI")
    if [[ $READING = ${KANJI// /} ]]; then
        # Try to find a Kanji variant for this word.
        KANJI=$("$(dirname "$0")"/../jmdict/ja.sh "$KANJI" | head -n 1 | sed 's/^\([^ ,]*\).*$/\1/')
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
    echo_ 'Please provide a word.'
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
if [[ $(wget -q "$URL" --timeout=10 -O - | md5sum | cut -f 1 -d ' ') = $NOT_FOUND ]]; then
    if [[ $WORD ]]; then
        printf_ 'No audio file available for %s.' "$WORD"
    else
        printf_ 'No audio file available.'
    fi
    exit 0
fi

# Try generating the tinyurl link. If it fails, print the long URL.
TINY=$(wget 'http://tinyurl.com/api-create.php?url='"$(encode_query "$URL")" \
         --quiet -O - --timeout=5 --tries=1)
printf_ 'Audio for %s: %s' "$WORD" "${TINY:-$URL}"

exit 0
