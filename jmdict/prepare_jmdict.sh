#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Fetches and prepares JMdict.gz for use with the lookup script known
# as "ja.sh".
set -eu

cd "$(dirname "$0")"

if [[ -e JMdict_e_prepared ]]; then
    exit 0
fi

TMP1=$(mktemp)
TMP2=$(mktemp)

if ! command -v xsltproc &>/dev/null; then
    echo "Error, could not find xsltproc."
    echo "Please install xmlstarlet."
    exit 1
fi

SOURCE=JMdict_e.gz

echo "Fetching $SOURCE..."
DONT_DELETE=
if [[ -s "$SOURCE" ]]; then
    echo "Not necessary, found $SOURCE in current directory..."
    DONT_DELETE=1
elif ! wget "http://ftp.edrdg.org/pub/Nihongo/$SOURCE"; then
    echo 'Failed.'
    exit 1
fi

gunzip --to-stdout "$SOURCE" > "$TMP1"
[[ $DONT_DELETE ]] || rm "$SOURCE"

# abbreviate entities
ENTITIES=$(grep -n ENTITY "$TMP1") 
FIRST_LINE=$(( $(echo "$ENTITIES" | head -n 1 | cut -d ':' -f 1) - 1 ))
LAST_LINE=$(( $(echo "$ENTITIES" | tail -n 1 | cut -d ':' -f 1) + 1 ))
ABBRV=$(echo "$ENTITIES" | cut -d ' ' -f 2 | xargs -I '{}' echo \<\!ENTITY '{}' \"'{}'\"\>)

(head -n "$FIRST_LINE" "$TMP1" ; \
    echo "$ABBRV" ; \
    tail -n +"$LAST_LINE" "$TMP1") \
    > "$TMP2"

echo "Transforming XML..."
xsltproc prepare_jmdict.xslt "$TMP2" > JMdict_e_prepared
echo "Done."
rm "$TMP1" "$TMP2"

exit 0
