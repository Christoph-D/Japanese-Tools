#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Katakana trainer.

DIRECTORY="$(dirname "$0")"
KANA_FILE="$DIRECTORY/katakana.txt"
IRC_COMMAND='!kata'

# shellcheck source=kana/kata.sh
. "$(dirname "$0")/kana.sh"
