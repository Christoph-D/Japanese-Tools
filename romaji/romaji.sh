#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Converts Japanese text into Romaji. The quality of the output
# heavily depends on kakasi and mecab. Kanji readings might be a bit
# off.

. "$(dirname "$0")"/../gettext/gettext.sh

READING=$(printf '%s' "$*" | mecab --node-format="%f[5] " --eos-format= --unk-format=%m)
# For some reason utf-8 support in kakasi on Ubuntu 9.04 seems to be
# broken.
# Fortunately, we have iconv.
# Unfortunately, this breaks the backslash '\' (among other, less
# important things), so we remove it first.
READING=${READING//\\/}
RESULT=$(echo "$READING" | \
    iconv -s -c -f utf-8 -t sjis | \
    kakasi -isjis -osjis -Ka -Ha -Ja | \
    iconv -s -c -f sjis -t utf-8)
if [ -n "$RESULT" ]; then
    # Remove newlines. There shouldn't be any, but we make sure.
    RESULT=${RESULT//$'\n'/}
    # Restrict length and print result
    printf " %s\n" "${RESULT:0:300}"
else
    echo_ 'No result.'
fi

exit 0
