#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script translates the command line parameters using Bablefish.

. "$(dirname "$0")"/../gettext/gettext.sh

TRANSLATE_SERVICE_URL='http://babelfish.yahoo.com/translate_txt'

set -u

# Accumulate all parameters
QUERY="$*"
# Restrict length
QUERY=${QUERY:0:300}

if [ -z "$QUERY" ]; then
    # No input.
    echo_ 'No input.'
    exit 0
fi

fix_html_entities() {
    sed "s/\&\#39;/'/g" |
    sed 's/\&lt;/</g' |
    sed 's/\&gt;/>/g' |
    sed 's/\&quot;/"/g' |
    sed 's/\&amp;/\&/g'
    return 0
}
translate() {
    # Escape single quotes for use in perl
    local ENCODED_QUERY=${1//\'/\\\'}
    local PAGE
    ENCODED_QUERY=$(perl -MURI::Escape -e "print uri_escape('$ENCODED_QUERY');")
    if ! PAGE=$(wget "$TRANSLATE_SERVICE_URL" \
        --quiet \
        --timeout=7 \
        --post-data="ei=UTF-8&doit=done&fr=bf-res&intl=1&tt=urltext&trtext=$ENCODED_QUERY&lp=$TARGET_LANG&btnTrTxt=Translate" \
        --header='Accept-Charset: utf-8' \
        -O -); then
        echo_ 'Time limit exceeded.'
    else
        printf '%s\n' "$PAGE" \
            | grep -m 1 '<div id="result">' \
            | sed 's/.*<div[^>]*>\([^<]*\)<.*/\1/' \
            | fix_html_entities
    fi
}

if [[ $QUERY =~ ^[a-zA-Z0-9' '.,?!:\;\<\>()\&\"/\']*$ ]]; then
    TARGET_LANG=en_ja
else
    TARGET_LANG=ja_en
fi

RESULT=$(translate "$QUERY")

if [ "$RESULT" = "$QUERY" ]; then
    # The service did not translate anything.
    echo_ 'This is untranslatable. :/'
    exit 0
fi
if [ -z "$RESULT" ]; then
    # The service did not return a result.
    echo_ 'No result. :-('
    exit 0
fi

# Restrict length and print result
printf '%s\n' "${RESULT:0:300}"

exit 0
