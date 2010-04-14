#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Creates ony entry for a vocabulary lists for the quiz script.

set -u

DICT=$(dirname "$0")/../jmdict/JMdict_e_prepared
MAX_RESULTS_PER_PATTERN=10
MAX_LENGTH_PER_ENGLISH=100
NOT_FOUND_MSG="Unknown word."

if [ ! -e "$DICT" ]; then
   echo "Please run: wget http://ftp.monash.edu.au/pub/nihongo/JMdict_e.gz && ./prepare_jmdict JMdict_e.gz > JMdict_e_prepared"
   exit 1
fi

# Get query and remove backslashes because we use them internally.
QUERY=${@//\\/}

if [ -z "$QUERY" ]; then
    echo "epsilon."
    exit 0
fi

print_result() {
    # Change $IFS to loop over lines instead of words.
    ORIGIFS=$IFS
    IFS=$'\n'
    SEEN=
    for R in $RESULT; do
        # Skip duplicate lines.
        if echo "$SEEN" | grep -qF "$R"; then
            continue
        fi
        SEEN=$(echo "$SEEN" ; echo "$R")

        KANJI=$(echo "$R" | cut -d '\' -f 1)
        KANA=$(echo "$R" | cut -d '\' -f 2)
        POS=$(echo "$R" | cut -d '\' -f 3)
        ENGLISH=$(echo "$R" | cut -d '\' -f 4)
        ENGLISH="($POS) $ENGLISH"
        if [ ${#ENGLISH} -gt $MAX_LENGTH_PER_ENGLISH ]; then
            ENGLISH="${ENGLISH:0:$(expr $MAX_LENGTH_PER_ENGLISH - 3)}..."
        fi
        FINAL="${FINAL:+$FINAL,}$KANA"
        FINAL_ENGLISH="${FINAL_ENGLISH:+$FINAL_ENGLISH / }$ENGLISH"
    done
    IFS=$ORIGIFS

    echo -e "$QUERY|$FINAL|$FINAL_ENGLISH"
}

# The more specific search patterns are used first
PATTERNS=( "^$QUERY\\\\" )

# Accumulate results over all patterns
RESULT=
for I in $(seq 0 1 $(expr ${#PATTERNS[@]} - 1)); do
    P="${PATTERNS[$I]}"
    RESULT=$(echo "$RESULT" ; grep -m $MAX_RESULTS_PER_PATTERN -e "$P" "$DICT")
done

if [ -n "$RESULT" ]; then
    print_result
else
    echo "$NOT_FOUND_MSG"
fi

exit 0
