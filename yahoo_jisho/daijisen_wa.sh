#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script asks wadoku.de for a daijisen entry
#

. "$(dirname "$0")"/../gettext/gettext.sh

MAX_TITLE_LENGTH=60
MAX_RESULT_LENGTH=120
URL="http://wadoku.de/dict/daijisen/"
LONGURL="http://dic.search.yahoo.co.jp/search?ei=UTF-8&stype=prefix&fr=dic&p="

set -u

# Accumulate all parameters
QUERY="$*"
# Restrict length
QUERY=${QUERY:0:100}

if [[ $QUERY = 'help' || $QUERY = '' ]]; then
    printf_ 'Example: %s' "$IRC_COMMAND 車　くるま"
    echo_ 'Providing the reading is optional. If it is missing, I will guess it.'
    exit 0
fi

# Split query into kanji and reading part.
KANJI=$(printf '%s' "$QUERY" | \
    sed 's#[ 　/／・[［「【〈『《].*##' | \
    sed 's#[　 ]##g')
READING=$(printf '%s' "$QUERY" | \
    sed 's#^[^ 　/／・[［「【〈『《]*.\(.*\)$#\1#' | \
    sed 's#[] 　／・［「【〈『《］」】〉』》]##g')

QUERY="${READING+$READING }$KANJI"

fix_html_entities() {
    sed "s/\&\#39;/'/g" |
    sed 's/\&lt;/</g' |
    sed 's/\&gt;/>/g' |
    sed 's/\&quot;/"/g' |
    sed 's/\&amp;/\&/g' |
    sed 's/\&nbsp;/ /g'
}
# Creates a tinyurl from $1.
make_tinyurl() {
    [[ ${NO_TINY_URL-} ]] || wget 'http://tinyurl.com/api-create.php?url='"$(encode_query "$1")" \
        --quiet -O - --timeout=5 --tries=1
}
# URL encoding.
encode_query() {
    # Escape single quotes for use in perl
    local ENCODED_QUERY=${1//\'/\\\'}
    ENCODED_QUERY=$(perl -MURI::Escape -e "print uri_escape('$ENCODED_QUERY');")
    printf '%s\n' "$ENCODED_QUERY"
}
ask_dictionary() {
    local URL="$URL$(encode_query "$1")"
    local LONGURL="$LONGURL$(encode_query "$KANJI")"
    local SOURCE
    SOURCE=$(wget "$URL" --quiet -O - --timeout=10 --tries=1)
    if [[ $? -ne 0 ]]; then
        echo_ 'A network error occured.'
        return
    fi
    SOURCE=$(printf '%s' "$SOURCE" | sed 's/^\[//;s/\]$//;s/[{}]//g;s#<[^>]*>##g')
    [[ $SOURCE ]] || return
    TITLE=$(printf '%s' "$SOURCE" | sed 's/"headword":"\([^"]*\)",".*/\1/')
    DEFINITION=$(printf '%s' "$SOURCE" | \
        sed 's/"headword":"\([^"]*\)","explanation":"//
             s/"headword":"//g;s/","explanation":"//g;
             s/［常用漢字］　\?//g;
             s/［音］[^１]*１/１/g;
             s/  \+/ /g;s/　/ /g;s/^ //')
    printf '%s\n' "${TITLE:0:$MAX_TITLE_LENGTH} ( $( make_tinyurl "$LONGURL" ) )"
    printf '%s\n' "${DEFINITION//$'\n'/   }"
}

RESULT=$(ask_dictionary "$QUERY")

if [[ ! $RESULT ]]; then
    echo "見つかりませんでした。"
    exit 0
fi

# Print title line.
printf '%s\n' "$RESULT" | head -n 1

# Cut off title line.
RESULT=$(printf '%s\n' "$RESULT" | tail -n +2)

# Restrict length if necessary.
if [[ ${#RESULT} -ge $(( $MAX_RESULT_LENGTH - 3 )) ]]; then
    RESULT="${RESULT:0:$(( $MAX_RESULT_LENGTH - 3 ))}"
    RESULT=$(printf '%s\n' "$RESULT")...
fi

# Print main result.
printf '%s\n' "$RESULT"

exit 0
