#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Prepares wadoku for use with the lookup script "wa.sh".

cd "$(dirname "$0")"

TMP1=.tmp1

if [[ ! -x $(which xsltproc) ]]; then
    echo "Error, could not find xsltproc."
    echo "Please install xmlstarlet."
    exit 1
fi
if [[ -e $TMP1 ]]; then
    echo "Error, temporary file \"$TMP1\" already exists."
    exit 1
fi

if [[ $# -ne 1 ]]; then
    echo 'Usage:'
    echo './prepare_wadoku.sh <wadoku.xml>'
    exit 1
fi

SOURCE="$1"

if [[ ! -s "$SOURCE" ]]; then
    echo 'Please visit http://www.wadoku.de/wiki/display/WAD/Downloads+und+Links and'
    echo 'download and extract the latest XML dump.'
    exit 0
fi

remove_redundant_information() {
    (
    # Remove duplicate entries from the kanji section
    set -f # disable glob patterns
    local LINE KANJI KANA SEEN IFS='◊'
    while read -r LINE; do
        KANJI="${LINE%%□*}"
        KANA="${LINE#*□}"
        KANA="${KANA%%□*}"
        SEEN=◊
        for K in $KANJI; do
            [[ $KANA != *$K* && $SEEN != *◊${K}◊* ]] || continue
            SEEN+="$K◊"
        done
        SEEN="${SEEN%◊}"
        printf '%s□%s\n' "${SEEN#◊}" "${LINE#*□}"
    done
    )
}
merge_alternative_readings() {
    local LINE PREFIX LAST_PREFIX= KANA KANA_BUFFER
    while read -r LINE; do
        PREFIX="${LINE%□*}"
        KANA="${LINE##*□}"
        if [[ $PREFIX = $LAST_PREFIX ]]; then
            KANA_BUFFER+=",$KANA"
        else
            if [[ $LAST_PREFIX ]]; then
                printf '%s□%s\n' "$LAST_PREFIX" "$KANA_BUFFER"
            fi
            KANA_BUFFER="$KANA"
        fi
        LAST_PREFIX="$PREFIX"
    done | sed 's#\([^□]*\)□\(.*\)□\([^□]*\)#\1□\3□\2#'
}

echo "Transforming XML..."
cp "$SOURCE" "$TMP1"
# Remove xmlns attributes because they don't work together with xslt
sed 's# xmlns="http://www.wadoku.de/xml/entry"##' -i "$TMP1"
xsltproc prepare_wadoku.xslt "$TMP1" > wadoku_prepared
# Remove redundant spaces
echo 'Compressing dictionary file...'
sed 's# ,#,#g;s# \+# #g;s# \([“)]\)#\1#g;s#\([„(]\) #\1#g' -i wadoku_prepared
sed 's#,\+#,#g;s#,□#□#g;s#[×〈〉△{}]##g;s# \+$##' -i wadoku_prepared
echo 'Removing redundant information...'
remove_redundant_information < wadoku_prepared > "$TMP1"
mv "$TMP1" wadoku_prepared
echo 'Merging alternative readings...'
sed 's#\([^□]*\)□\([^□]*\)□\(.*\)#\1□\3□\2#' -i wadoku_prepared
LC_ALL=C sort wadoku_prepared > "$TMP1"
merge_alternative_readings < "$TMP1" > wadoku_prepared
rm "$TMP1"
echo "Done."

exit 0
