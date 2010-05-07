#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Prepares JMdict.gz for use with the lookup script known as "ja".

if [ ! -e "$1" ]; then
    echo "Error, input file \"$1\" does not exist."
    exit 1
fi

TMP1="$1".tmp1
TMP2="$1".tmp2

if [ -e "$TMP1" ]; then
    echo "Error, temporary file \"$TMP1\" already exists."
    exit 1
fi
if [ -e "$TMP2" ]; then
    echo "Error, temporary file \"$TMP2\" already exists."
    exit 1
fi

gunzip --to-stdout "$1" > "$TMP1"

# abbreviate entities
ENTITIES=$(grep -n ENTITY "$TMP1") 
FIRST_LINE=$(expr $(echo "$ENTITIES" | head -n 1 | cut -d ':' -f 1) - 1)
LAST_LINE=$(expr $(echo "$ENTITIES" | tail -n 1 | cut -d ':' -f 1) + 1)
ABBRV=$(echo "$ENTITIES" | cut -d ' ' -f 2 | xargs -n 1 -I '{}' echo \<\!ENTITY '{}' \"'{}'\"\>)

(head -n "$FIRST_LINE" "$TMP1" ; \
    echo $ABBRV ; \
    tail -n +"$LAST_LINE" "$TMP1") \
    > "$TMP2"

xsltproc prepare_jmdict.xslt "$TMP2"
rm "$TMP1" "$TMP2"

exit 0
