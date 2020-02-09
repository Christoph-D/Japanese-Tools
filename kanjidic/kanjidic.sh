#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Kanji lookup.

. "$(dirname "$0")"/../gettext/gettext.sh

DICT=$(dirname "$0")/kanjidic
MAX_NUMBER_OF_LINES=3

if [[ ! ${IRC_PLUGIN:-} ]]; then
    MAX_NUMBER_OF_LINES=20
fi

if [[ ! -e $DICT ]]; then
   printf_ 'Please run: %s' './prepare_kanjidic.sh'
   exit 1
fi

QUERY="$*"

if [[ -z $QUERY ]]; then
    echo_ 'epsilon.'
    exit 0
fi

if echo "$QUERY" | grep -q '^[a-zA-Z0-9]*$'; then
    echo_ 'Please enter some kanji.'
    exit 0
fi

if echo "$QUERY" | grep -q -e "[][*+()$^.]"; then
    echo_ 'No regular expressions, please.'
    exit 0
fi

# Remove backslashes and escape the dot as they have special
# significance in regexps.
QUERY=${QUERY//\\/}
QUERY=${QUERY//./\\.}

find_meanings() {
    local IFS=$'\n' X MEANING RESULT
    X=$(echo "$1" | sed 's/^[^{]*//' | sed 's/[{}]/\n/g')
    for MEANING in $X; do
        if ! echo "$MEANING" | grep -q '^[[:space:]]*$'; then
            RESULT="$RESULT${RESULT:+, }$MEANING"
        fi
    done
    printf '%s' "$RESULT"
}

find_readings() {
    local RESULT ID X
    for X in $(echo "$1" | cut -d ' ' -f 3-); do
        ID="${X:0:1}"
        [[ $ID = '{' ]] && break
        if [[ $ID = 'T' ]]; then
            RESULT="${RESULT%, }$(echo_ '. In names: ')"
        elif ! echo "$ID" | grep -q '[A-Z]'; then
            RESULT="$RESULT$X, "
        fi
    done
    printf '%s' "${RESULT%, }"
}

find_stroke_count() {
    echo "$1" | sed 's/.* S\([0-9]*\) .*/\1/'
}

format_entry() {
    local STROKES READINGS MEANINGS
    STROKES=$(find_stroke_count "$1")
    READINGS=$(find_readings "$1")
    MEANINGS=$(find_meanings "$1")
    nprintf_ "%s: %s stroke. %s {%s}" "%s: %s strokes. %s {%s}" "$STROKES" \
        "${1:0:1}" "$STROKES" "$READINGS" "$MEANINGS"
}

FOUND=0
for I in $(seq 0 $(( ${#QUERY} - 1 ))); do
    CHAR="${QUERY:$I:1}"
    ENTRY=$(grep -m 1 -e "^$CHAR " "$DICT")
    if [[ $ENTRY ]]; then
        let ++FOUND
        format_entry "$ENTRY"
    fi
    [[ $FOUND -eq $MAX_NUMBER_OF_LINES ]] && break
done

if [[ $FOUND -eq 0 ]]; then
    echo_ 'No kanji found.'
fi

exit 0
