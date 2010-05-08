#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# RTK keyword lookup.
# The lookup works in both directions: From keyword to kanji and from
# kanji to keyword.
# 
# The command line arguments must be utf-8 encoded.

. "$(dirname "$0")"/../gettext/gettext.sh

DICT=$(dirname "$0")/rtk.txt
MAX_KEYWORDS=10
# Hardcoded limit on line length for IRC
MAX_LINE_LENGTH=300

if [[ ! -e $DICT ]]; then
   echo_ 'Please put "rtk.txt" in the same directory as this script.'
   exit 1
fi

QUERY=$@

if [[ -z $QUERY ]]; then
    echo_ 'epsilon.'
    exit 0
fi

if echo "$QUERY" | grep -q -e "[*+()$^]"; then
    echo_ 'No regular expressions, please.'
    exit 0
fi

# Remove backslashes and escape the dot as they have special
# significance in regexps.
QUERY=${QUERY//\\/}
QUERY=${QUERY//./\\.}

format_entry() {
    KANJI=$(echo "$1" | cut -f 1)
    KEYWORD=$(echo "$1" | cut -f 2)
    NUMBER=$(echo "$1" | cut -f 3)
    printf "#%d: %s %s" "$NUMBER" "$KEYWORD" "$KANJI" 2> /dev/null
}

if echo "$QUERY" | grep -q -e "^\([a-zA-Z \-]\|\\\.\)*$"; then
    # Query contains a keyword, so look for the matching kanji.
    LINES1=$(grep -i -m 1 -e "	${QUERY}	" "$DICT")
    LINES2=$(grep -i -m $MAX_KEYWORDS -e "	${QUERY}..*	" "$DICT")
    LINES3=$(grep -i -m $MAX_KEYWORDS -e "	..*${QUERY}.*	" "$DICT")
    LINES=$(echo -e "$LINES1\n$LINES2\n$LINES3" | head -n $MAX_KEYWORDS)
    if [[ -n $LINES ]]; then
        # Change $IFS to loop over lines instead of words.
        ORIGIFS=$IFS
        IFS=$'\n'
        for LINE in $LINES; do
            RESULT="$RESULT${RESULT+, }$(format_entry "$LINE")"
        done
        IFS=$ORIGIFS
    else
        # Unknown keyword
        RESULT=$(_ 'Unknown keyword.')
    fi
elif echo "$QUERY" | grep -q -e '^[0-9 ]\+$'; then
    # Query contains Kanji numbers.
    while read -r CURRENT_NUMBER QUERY < <(echo "$QUERY") && \
        [[ -n $CURRENT_NUMBER ]]; do
        ENTRY=$(grep -m 1 -e "^[^	]*	[^	]*	$CURRENT_NUMBER\$" "$DICT")
        R='???'
        if [[ -n $ENTRY ]]; then
            # Found the corresponding Kanji
            R=$(format_entry "$ENTRY")
        fi
        RESULT="$RESULT${RESULT+ | }$R"
    done
    if [[ $RESULT = '???' ]]; then
        RESULT=$(_ 'Unknown kanji number.')
    fi
else
    # Query likely contains Japanese characters.

    # Decode letter by letter, restricted to the first few letters.
    # Note that it is crucial for this step to work that $LANG
    # contains something with "UTF-8". Otherwise substring access to
    # $QUERY would not work as expected.
    QUERY=${QUERY:0:15}
    for I in $(seq 0 $(expr ${#QUERY} - 1)); do
        CHAR="${QUERY:$I:1}"
        ENTRY=$(grep -m 1 -e "$CHAR	[^	]*	" "$DICT")
        R=$CHAR
        if [[ -n $ENTRY ]]; then
            R=$(format_entry "$ENTRY")
        fi
        RESULT="$RESULT${RESULT+ | }$R"
    done
fi

echo "${RESULT:0:$MAX_LINE_LENGTH}"

exit 0
