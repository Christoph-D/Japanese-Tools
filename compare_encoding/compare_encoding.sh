#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script compares the efficiency of different utf encodings for
# Japanese Wikipedia articles.

. "$(dirname "$0")"/../gettext/gettext.sh

set -u

if [[ $# -ne 1 ]]; then
    printf_ 'Usage: %s lemma ' "$(basename "$0")"
    exit 0
fi

# URL encoding.
encode_query() {
    # Make the query safe: Remove backslashes and escape single quotes.
    local ENCODED_QUERY=${1//\\/}
    ENCODED_QUERY=${ENCODED_QUERY//\'/\\\'}
    ENCODED_QUERY=$(perl -MURI::Escape -e "print uri_escape('$ENCODED_QUERY');")
    printf '%s\n' "$ENCODED_QUERY"
}

QUERY="${1:0:100}"

LEMMA=$(encode_query "$QUERY")
URL="http://ja.wikipedia.org/wiki/$LEMMA"

TMP_FILE8=$(mktemp)
TMP_FILE16=$(mktemp)
trap "rm -f '$TMP_FILE8' '$TMP_FILE16'" EXIT

if ! wget -q "$URL" -O "$TMP_FILE8"; then
    echo_ 'Article not found on Wikipedia.'
    exit 0
fi

UTF8="$(du -sb "$TMP_FILE8" | cut -f 1)"
iconv -f utf8 -t utf16 $TMP_FILE8 > $TMP_FILE16
UTF16="$(du -sb "$TMP_FILE16" | cut -f 1)"

printf_no_newline_ "UTF-8 vs. UTF-16: %d vs. %d bytes." "$UTF8" "$UTF16"

compute_relative_difference() {
    # Without bc we cannot compute the relative difference.
    [[ -x $(which bc) ]] || return
    local UTF8="$1" UTF16="$2"
    if [[ $UTF8 -lt $UTF16 ]]; then
        # Workaround for printf not accepting numbers like "1.2" on non-US
        # locale.
        OUTPUT=$(printf_ "UTF-8 wins by %s." '%.1f%%')
        export LC_ALL=en_US.UTF8
        printf "$OUTPUT" "$(echo "scale=3;($UTF16-$UTF8)/$UTF16*100" | bc)"
    elif [[ $UTF8 -gt $UTF16 ]]; then
        OUTPUT=$(printf_ "UTF-16 wins by %s." '%.1f%%')
        export LC_ALL=en_US.UTF8
        printf "$OUTPUT" "$(echo "scale=3;($UTF8-$UTF16)/$UTF8*100" | bc)"
    else
        echo_ "It's a tie."
    fi
}
printf ' %s\n' "$(compute_relative_difference "$UTF8" "$UTF16")"

exit 0
