#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Dictionary lookup for Japanese words.
set -eu

# shellcheck source=gettext/gettext.sh
. "$(dirname "$0")"/../gettext/gettext.sh

MAX_RESULTS_PER_PATTERN=5
MAX_LENGTH_PER_ENGLISH=150
MAX_LINE_LENGTH=200
MAX_LINES=1

if [[ ! ${IRC_PLUGIN:-} ]]; then
    MAX_LENGTH_PER_ENGLISH=250
    MAX_LINE_LENGTH=300
    MAX_LINES=5
fi

if [[ ! -v DICT ]]; then
   printf_ 'Please call jm.sh or wa.sh instead.'
   exit 1
fi
if [[ ! -e $DICT ]]; then
   printf_ 'Could not find the dictionary.'
   exit 1
fi

# Get query and remove the character we use internally as a field
# separator.
QUERY=${*//□/}
# Escape special characters.
QUERY=$(printf '%s' "$QUERY" | sed 's/\([][().*+^$\]\)/\\\1/g')

if [[ -z $QUERY ]]; then
    echo_ 'epsilon.'
    exit 0
fi

clean_up_kanji() {
    # $1 is a ◊-delimited string containing the kanji elements.
    local IFS='◊' kanji='' kanji_buffer='' rest="$1"
    while [[ $rest ]]; do
        read -r kanji rest < <(printf '%s' "$rest")
        # Always print the first kanji element and after that only
        # matching kanji elements.
        if [[ ! $kanji_buffer || $kanji = *$QUERY* ]]; then
            kanji_buffer="${kanji_buffer:+$kanji_buffer }$kanji"
        fi
    done
    printf '%s' "$kanji_buffer"
}

get_current_item() {
    local IFS='□' kanji kana pos english japanese
    read -r kanji kana pos english <<< "$1"
    pos=${pos:+ ($pos)}
    if [[ -n "$kanji" ]]; then
        kanji=$(clean_up_kanji "$kanji")
        japanese="$kanji [$kana]$pos"
    else
        japanese="$kana$pos"
    fi
    if [[ ${#english} -gt $MAX_LENGTH_PER_ENGLISH ]]; then
        english="${english:0:$(( MAX_LENGTH_PER_ENGLISH - 3))}..."
    fi
    local result="$japanese, $english"
    if [[ ${#result} -gt $MAX_LINE_LENGTH ]]; then
        english="${english:0:$(( MAX_LINE_LENGTH - 5 - ${#L} ))}..."
        result="$japanese, $english"
    fi
    printf '%s' "$result"
}

format_result() {
    # Change $IFS to loop over lines instead of words.
    local IFS=$'\n' seen='' line_count=0 line_buffer='' r final
    for r in $1; do
        # Skip duplicate lines.
        [[ $seen != *$r* ]] || continue
        seen+="$r"

        local current_item next
        current_item=$(get_current_item "$r")
        if [[ ${IRC_PLUGIN:-} ]]; then
            next="${line_buffer:+$line_buffer / }$current_item"
        else
            next="${line_buffer:+$line_buffer\n}$current_item"
        fi

        # If the final string would get too long, we're done.
        if [[ ${#next} -gt $MAX_LINE_LENGTH ]]; then
            # Append the current line to the result.
            final="${final:+$final\n}$line_buffer"
            # Remember the current item for the next line only if it
            # fits.
            line_buffer=
            if [[ ${#current_item} -le $MAX_LINE_LENGTH ]]; then
                line_buffer="$current_item"
            fi
            (( ++line_count ))
            [[ $line_count -ge $MAX_LINES ]] && break
        else
            line_buffer=$next
        fi
    done
    if [[ $line_count -lt $MAX_LINES ]]; then
        final="${final:+$final\n}$line_buffer"
    fi
    printf '%s\n' "${final//\\n/$'\n'}"
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
tmp_dict=$(mktemp)
# shellcheck disable=SC2064
trap "rm '$tmp_dict'" EXIT
grep -F "$QUERY" "$DICT" | head -n 10000 > "$tmp_dict"

# Accumulate results over all patterns.
result=
for i in $(seq 0 1 $(( ${#PATTERNS[@]} - 1 ))); do
    pattern="${PATTERNS[$i]}"
    result="${result:+$result$'\n'}$(grep -m $MAX_RESULTS_PER_PATTERN -e "$pattern" "$tmp_dict")" || true
done

if [[ $result ]]; then
    result=$(format_result "$result")
    if [[ $result ]]; then
        echo "$result"
    else
        echo_ 'Too little space to show the result.'
    fi
else
    echo_ 'Unknown word.'
fi

exit 0
