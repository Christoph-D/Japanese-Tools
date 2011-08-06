#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script performs a Google dictionary lookup for english words.
#

URL='http://www.google.com/search?hl=en&tbs=dfn%3A1&cad=h&q='

MAX_RESULT_LENGTH=300

# Accumulate all parameters
QUERY="$*"
# Restrict length
QUERY=${QUERY:0:300}

if [[ -z $QUERY ]]; then
    QUERY="empty"
fi

fix_html_entities() {
    sed "s/\&\#39;/'/g" |
    sed 's/\&lt;/</g' |
    sed 's/\&gt;/>/g' |
    sed 's/\&quot;/"/g' |
    sed 's/\&amp;/\&/g'
}
encode_query() {
    # Escape single quotes for use in perl
    local ENCODED_QUERY=${1//\'/\\\'}
    ENCODED_QUERY=$(perl -MURI::Escape -e "print uri_escape('$ENCODED_QUERY');")
    echo "$URL$ENCODED_QUERY"
}
ask_google() {
    RESULT=$(wget --user-agent='Mozilla/5.0 (X11; Linux x86_64; rv:5.0) Gecko/20100101 Firefox/5.0' "$(encode_query "$1")" --quiet -O - \
        | sed 's/<li style="list-style:decimal">\(\([^<]\|<[^d]\|<d[^i]\)*\)</[[\1]]/g;t;d' \
        | sed 's#>&emsp;\(/\([^/]\|/[^&]\)*/\)&emsp;<#[[\1]]#g;s/&emsp;//g' \
        | sed 's/^[^[]*\[\[//;s/\]\][^]]*$//;s/\]\][^[]*\[\[/.\n/g' \
        | sed 's#/\.#/#g' \
        | sed 's#</\?em>#*#g' \
        | sed 's/<[^>]*>//g' \
        | fix_html_entities \
        | fix_html_entities)
    if [[ -n $RESULT ]]; then
        echo "${RESULT//$'\n'/   }"
        return
    fi
}

RESULT=$(ask_google "$QUERY")

if [[ -z $RESULT ]]; then
    echo "No result. :-("
    exit 0
fi

if [[ ${#RESULT} -lt $(( $MAX_RESULT_LENGTH - 3 )) ]]; then
    echo "$RESULT"
    exit 0
fi

# Restrict length and print result
RESULT="${RESULT:0:$(( $MAX_RESULT_LENGTH - 3 ))}"
RESULT=$(echo "$RESULT" | sed 's/ [^ ]*$/ /')
echo "$RESULT... (more at $(encode_query "$QUERY") )"

exit 0
