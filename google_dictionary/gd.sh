#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script performs a Google dictionary lookup for english words.
#

URL='http://www.google.com/search?hl=en&q=define+'

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
    RESULT=$(wget --user-agent='Mozilla/5.0 (X11; Linux x86_64; rv:5.0) Gecko/20100101 Firefox/5.0' \
        --quiet -O - \
        "$(encode_query "$1")" \
        | grep 'class="vk_ans"' \
        | perl -pe '
# Find the start of the definition and remove the hyphenated word in
# the result.
s#.*?<div class="vk_ans".*?>.*?</div>(.*)#$1#g;

# Replace <i>foo</i> with *foo*.
s#</?i>#*#g;

s#<script.*?(</script>|$)##g;

# Find the pronunciation and mark it with //.
s#.*<span class="lr_dct_ph"><span>([^<]*)#/$1/#;

# Remove the word class ("adjective", "noun", etc.).
s#<div class="lr_dct_sf_h">.*?</div>##;
s#<div class="xpdxpnd vk_gy".*?</div>##g;

# Remove all definitions except the first one.
s#<div class="lr_dct_sf_h">.*##;

# Remove example sentences.
s#<div class="vk_gy">.*?</div>##g;

# Remove synonyms and antonyms.
s#<table class="vk_tbl vk_gy">.*?</table>##g;

# Remove etymology.
s#<div class="xpdxpnd".*##;

# Surround topic markers with parentheses.  Example: (Music) in the
# definition of "space".
s#<span class="lr_dct_lbl_blk.*?>(.*?)</span>#($1)#g;

# Sanity check: The result should begin with a pronunciation or a
# <div> tag without attributes.  If it begins with something else,
# something is wrong and we better drop everything.
s#^(?!(/|<div>)).*##;

# Remove all remaining tags
s/<.*?>/ /g;

# Collapse spaces.
s# / ##; s# +# #g; s#^ ##; s/ $//; s# ([,.])#$1#g;

# Remove a single dot at the end.
s/\.$//;' \
        | sed '/^[^2]*$/s/1\. //' \
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
