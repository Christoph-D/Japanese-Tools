#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script translates the command line parameters using Google.

echo "--- Broken ---"
exit 0

# shellcheck source=gettext/gettext.sh
. "$(dirname "$0")"/../gettext/gettext.sh

# Default target language
TARGET_LANG=${LANG:0:2}
# Fallback target language
SECOND_TARGET_LANG=ja
# Allowed target languages
KNOWN_LANGUAGES=( de en es 'fi' fr it ja sv zh ru )

# Default source language (empty string is "guess language")
SOURCE_LANG=

# Maximum number of chained translations.
MAX_PATH_LENGTH=5

TRANSLATE_SERVICE_URL="http://ajax.googleapis.com/ajax/services/language/translate"

set -u

# Make sure TARGET_LANG is not empty.
[[ $TARGET_LANG ]] || TARGET_LANG=en

# Accumulate all parameters
QUERY="$*"
# Restrict length
QUERY=${QUERY:0:300}

if [ -z "$QUERY" ]; then
    # No input.
    echo_ 'No input.'
    exit 0
fi

get_lang_selector() {
    # The first word might be a language selector.
    local LANG_CANDIDATE_LIST LANG_CANDIDATE L
    read -r -a LANG_CANDIDATE_LIST <<< "$1"
    LANG_CANDIDATE=${LANG_CANDIDATE_LIST[0]}
    for L in "${KNOWN_LANGUAGES[@]}"; do
        if [ "$LANG_CANDIDATE" = "$L" ]; then
            echo "$L"
            return 0
        fi
    done
    # No valid language selector found
    return 1
}
remove_lang_selector() {
    echo "$1" | cut -d ' ' -f 2-
}

# Parse requested translation path.
LANG_PATH=
LANG_PATH_PRETTY=
LAST_LANG_SELECTOR=
I=0
while true; do
    LANG_SELECTOR=$(get_lang_selector "$QUERY") || break
    (( ++I ))
    [[ $I -gt $MAX_PATH_LENGTH ]] && break
    QUERY=$(remove_lang_selector "$QUERY")
    if [[ $LANG_SELECTOR != "$LAST_LANG_SELECTOR" ]]; then
        LANG_PATH+="$LANG_SELECTOR "
        LANG_PATH_PRETTY+="->$LANG_SELECTOR"
    fi
    LAST_LANG_SELECTOR="$LANG_SELECTOR"
done

fix_html_entities() {
    sed "s/\&\#39;/'/g" |
    sed 's/\&lt;/</g' |
    sed 's/\&gt;/>/g' |
    sed 's/\&quot;/"/g' |
    sed 's/\&amp;/\&/g'
}
translate() {
    # Escape single quotes for use in perl
    local ENCODED_QUERY=${1//\'/\\\'}
    ENCODED_QUERY=$(perl -MURI::Escape -e "print uri_escape('$ENCODED_QUERY');")
    wget "$TRANSLATE_SERVICE_URL" \
        --quiet \
        --post-data="v=1.0&q=$ENCODED_QUERY&langpair=$SOURCE_LANG%7C$TARGET_LANG" \
        -O - \
        | grep -e '"translatedText"' \
        | sed 's/.*"translatedText":"\([^"]*\)".*/\1/' \
        | sed 's/\\u0026/\&/g' \
        | fix_html_entities
}

if [[ ! $LANG_PATH ]]; then
    # No language path requested. Use heuristic.
    RESULT=$(translate "$QUERY")
    if [ "$RESULT" = "$QUERY" ]; then
    # If the translation service did not translate anything, try again
    # with the fallback language.
        TARGET_LANG=$SECOND_TARGET_LANG
        RESULT=$(translate "$QUERY")
    fi
else
    # Walk language path.
    RESULT="$QUERY"
    for T in $LANG_PATH; do
        TARGET_LANG="$T"
        RESULT=$(translate "$RESULT")
        SOURCE_LANG="$TARGET_LANG"
    done
    RESULT="${LANG_PATH_PRETTY:2}: $RESULT"
fi

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
echo "${RESULT:0:300}"

exit 0
