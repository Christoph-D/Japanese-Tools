#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script prints the number of google results for a given string.

. "$(dirname "$0")"/../gettext/gettext.sh

set -u
MAX_LINE_LENGTH=250

if [[ $# -ne 1 ]]; then
    printf_ 'Usage: %s some_string ' "$(basename "$0")"
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

QUERY=${1:0:100}

if printf '%s\n' "$QUERY" | grep -q '^\([[:punct:]]\|[a-zA-Z0-9 ]\)*$'; then
    # The query does not contain CJK characters.
    PATTERN='About [0-9,]* results'
    BASE_URL="http://google.com/search?lr=lang_en&as_q="
    NO_RESULTS='No results.'
else
    # The query does contain CJK characters.
    PATTERN='約 [0-9,]* 件'
    BASE_URL="http://google.co.jp/search?lr=lang_ja&as_q="
    NO_RESULTS='見つかりませんでした。'
fi

QUERY=$(encode_query "$QUERY")
URL="$BASE_URL$QUERY"

# We need to set the user-agent to something common or google will not
# send UTF8.
if ! RESULT=$(wget -q -U 'Mozilla/5.0' "$URL" -O -); then
    echo_ 'An error occured.'
    exit 0
fi

if ! NUMBER_OF_RESULTS=$(printf '%s' "$RESULT" | grep -o "$PATTERN"); then
    echo "$NO_RESULTS"
    exit 0
fi

RESULT="$NUMBER_OF_RESULTS: $URL"
if [[ ${#RESULT} -gt $MAX_LINE_LENGTH ]]; then
    RESULT="$NUMBER_OF_RESULTS."
fi
printf '%s\n' "$RESULT"

exit 0
