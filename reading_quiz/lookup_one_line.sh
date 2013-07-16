#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Creates ony entry for a vocabulary lists for the quiz script.

set -u

DICT=$(dirname "$0")/../jmdict/JMdict_e_prepared
MAX_RESULTS_PER_PATTERN=10
NOT_FOUND_MSG="Unknown word."

if [ ! -e "$DICT" ]; then
   echo "Please run: wget http://ftp.monash.edu.au/pub/nihongo/JMdict_e.gz && ./prepare_jmdict.sh JMdict_e.gz > JMdict_e_prepared"
   exit 1
fi

QUERY=$@

if [ -z "$QUERY" ]; then
    echo "epsilon."
    exit 0
fi

MAX_LINE_LENGTH=200

split_result() {
    local IFS='□'
    read -r KANJI KANA POS ENGLISH < <(printf '%s' "$1")
}

print_result() {
    # Change $IFS to loop over lines instead of words.
    local IFS=$'\n'
    local KANA_BUFFER= ENGLISH_BUFFER=
    for R in $RESULT; do
        split_result "$R"
        KANA_BUFFER="${KANA_BUFFER:+$KANA_BUFFER,}$KANA"
        ENGLISH_BUFFER="${ENGLISH_BUFFER:+$ENGLISH_BUFFER / }$ENGLISH"
    done
    FINAL="$QUERY|$KANA_BUFFER|$ENGLISH_BUFFER"
    if [[ ${#FINAL} -gt $MAX_LINE_LENGTH ]]; then
        FINAL="${FINAL:0:$(expr $MAX_LINE_LENGTH - 3)}..."
    fi
    echo -e "$FINAL"
}

RESULT=$(grep -m $MAX_RESULTS_PER_PATTERN -e "^\([^□]*◊\)\?$QUERY\(□\|◊\)" "$DICT")
if [[ -n "$RESULT" ]]; then
    print_result
else
    echo "$NOT_FOUND_MSG"
fi

exit 0
