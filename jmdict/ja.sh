#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Dictionary lookup for Japanese words.

. "$(dirname "$0")"/../gettext/gettext.sh

set -u

MAX_RESULTS_PER_PATTERN=5
MAX_LENGTH_PER_ENGLISH=150
MAX_LINE_LENGTH=200
MAX_LINES=1

if [[ ! ${IRC_PLUGIN:-} ]]; then
    MAX_LENGTH_PER_ENGLISH=250
    MAX_LINE_LENGTH=300
    MAX_LINES=5
fi

DICT=$(dirname "$0")/JMdict_e_prepared

if [[ ! -e $DICT ]]; then
   printf_ 'Please run: %s' './prepare_jmdict.sh'
   exit 1
fi

# Get query and remove the character we use internally as a field
# separator.
QUERY=${@//□/}
# Escape special characters.
QUERY=$(printf '%s' "$QUERY" | sed 's/\([][().*+^$\]\)/\\\1/g')

if [[ -z $QUERY ]]; then
    echo_ 'epsilon.'
    exit 0
fi

clean_up_kanji() {
    # $1 is a ◊-delimited string containing the kanji elements.
    local IFS='◊' KANJI KANJI_BUFFER= REST="$1"
    while [[ $REST ]]; do
        read -r KANJI REST < <(printf '%s' "$REST")
        # Always print the first kanji element and after that only
        # matching kanji elements.
        if [[ ! $KANJI_BUFFER || $KANJI = *$QUERY* ]]; then
            KANJI_BUFFER="${KANJI_BUFFER:+$KANJI_BUFFER }$KANJI"
        fi
    done
    printf '%s' "$KANJI_BUFFER"
}

get_current_item() {
    local IFS='□' KANJI KANA POS ENGLISH
    read -r KANJI KANA POS ENGLISH < <(printf '%s' "$1")
    if [[ -n "$KANJI" ]]; then
        KANJI=$(clean_up_kanji "$KANJI")
        local L="$KANJI [$KANA] ($POS)"
    else
        local L="$KANA ($POS)"
    fi
    if [[ ${#ENGLISH} -gt $MAX_LENGTH_PER_ENGLISH ]]; then
        ENGLISH="${ENGLISH:0:$(expr $MAX_LENGTH_PER_ENGLISH - 3)}..."
    fi
    echo "$L, $ENGLISH"
}

print_result() {
    # Change $IFS to loop over lines instead of words.
    local IFS=$'\n'
    local SEEN=
    local LINE_COUNT=0
    local LINE_BUFFER=
    for R in $RESULT; do
        # Skip duplicate lines.
        [[ $SEEN != *$R* ]] || continue
        SEEN+="$R"

        local CURRENT_ITEM=$(get_current_item "$R")
        if [[ ${IRC_PLUGIN:-} ]]; then
            NEXT="${LINE_BUFFER:+$LINE_BUFFER / }$CURRENT_ITEM"
        else
            NEXT="${LINE_BUFFER:+$LINE_BUFFER\n}$CURRENT_ITEM"
        fi

        # If the final string would get too long, we're done.
        if [[ ${#NEXT} -gt $MAX_LINE_LENGTH ]]; then
            # Append the current line to the result.
            FINAL="${FINAL:+$FINAL\n}$LINE_BUFFER"
            # Remember the current item for the next line only if it
            # fits.
            LINE_BUFFER=
            if [[ ${#CURRENT_ITEM} -le $MAX_LINE_LENGTH ]]; then
                LINE_BUFFER="$CURRENT_ITEM"
            fi
            let ++LINE_COUNT
            [[ $LINE_COUNT -ge $MAX_LINES ]] && break
        else
            LINE_BUFFER=$NEXT
        fi
    done
    if [[ $LINE_COUNT -lt $MAX_LINES ]]; then
        FINAL="${FINAL:+$FINAL\n}$LINE_BUFFER"
    fi

    echo -e "$FINAL"
}

# The more specific search patterns are used first.
PATTERNS=(
    # Perfect match.
    "\(□\|^\|◊\)$QUERY\(\$\|□\|◊\)"
    # Match primary kana reading.
    "^[^□]*□$QUERY\(,\|□\)"
    # Match secondary kana readings.
    "^[^□]*□[^□]*,$QUERY\(,\|□\)"
    # Match "1. $QUERY (possibly something in brackets),".
    "□\(1□. \)$QUERY\( ([^,]*\?)\)\?,"
    # Match "1. $QUERY " or "1. $QUERY,".
    "□\(1□. \)\?$QUERY\( \|,\)"
    # Match $QUERY at the beginning of an entry (Kanji, Kana or English).
    "\(□\|^\|◊\)\(1□. \)\?$QUERY"
    # Match $QUERY at second position in the English definition.
    "2□. $QUERY\( ([^,]*\?)\)\?\(,\|\$\)"
    # Match $QUERY everywhere.
    "$QUERY"
    )

# Preselect some lines.
TMP_DICT=$(mktemp)
trap "rm '$TMP_DICT'" EXIT
grep -F "$QUERY" "$DICT" > "$TMP_DICT"

# Accumulate results over all patterns.
RESULT=
for I in $(seq 0 1 $(expr ${#PATTERNS[@]} - 1)); do
    P="${PATTERNS[$I]}"
    RESULT=$(echo "$RESULT" ; grep -m $MAX_RESULTS_PER_PATTERN -e "$P" "$TMP_DICT")
done

if [[ $RESULT ]]; then
    RESULT=$(print_result)
    if [[ $RESULT ]]; then
        echo "$RESULT"
    else
        echo_ 'Too little space to show the result.'
    fi
else
    echo_ 'Unknown word.'
fi

exit 0
