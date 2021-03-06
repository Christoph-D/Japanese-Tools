#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Fetches and prepares JMdict.gz for use with the lookup script known
# as "ja.sh".
set -eu

cd "$(dirname "$0")"

TMP1="$1".tmp1
TMP2="$1".tmp2

if ! command -v xsltproc &>/dev/null; then
    echo "Error, could not find xsltproc."
    echo "Please install xmlstarlet."
    exit 1
fi
if [[ -e $TMP1 ]]; then
    echo "Error, temporary file \"$TMP1\" already exists."
    exit 1
fi
if [[ -e $TMP2 ]]; then
    echo "Error, temporary file \"$TMP2\" already exists."
    exit 1
fi

echo 'This script will download and preprocess the jmdic file for use with the kanjidic script.
The file will be placed in the following directory:'
echo "$(readlink -f .)/"
read -rp "Proceed? [y]" OK

[[ $OK && $OK != 'y' && $OK != 'Y' ]] && exit 1

SOURCE=JMdict_e.gz

echo "Fetching $SOURCE..."
DONT_DELETE=
if [[ -s "$SOURCE" ]]; then
    echo "Not necessary, found $SOURCE in current directory..."
    DONT_DELETE=1
elif ! wget "http://ftp.monash.edu.au/pub/nihongo/$SOURCE"; then
    echo 'Failed.'
    exit 1
fi

gunzip --to-stdout "$SOURCE" > "$TMP1"
[[ $DONT_DELETE ]] || rm "$SOURCE"

# abbreviate entities
ENTITIES=$(grep -n ENTITY "$TMP1") 
FIRST_LINE=$(( $(echo "$ENTITIES" | head -n 1 | cut -d ':' -f 1) - 1 ))
LAST_LINE=$(( $(echo "$ENTITIES" | tail -n 1 | cut -d ':' -f 1) + 1 ))
ABBRV=$(echo "$ENTITIES" | cut -d ' ' -f 2 | xargs -n 1 -I '{}' echo \<\!ENTITY '{}' \"'{}'\"\>)

(head -n "$FIRST_LINE" "$TMP1" ; \
    echo "$ABBRV" ; \
    tail -n +"$LAST_LINE" "$TMP1") \
    > "$TMP2"

echo "Transforming XML..."
xsltproc prepare_jmdict.xslt "$TMP2" > JMdict_e_prepared
echo "Done."
rm "$TMP1" "$TMP2"

exit 0
