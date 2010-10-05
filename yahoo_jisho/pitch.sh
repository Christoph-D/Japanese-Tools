#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script performs a Yahoo dictionary lookup for Japanese words
# and prints only the pitch information.
#

. "$(dirname "$0")"/../gettext/gettext.sh

set -u

if [[ $# = 0 || $1 = 'help' || $1 = '' ]]; then
    printf_ 'Example: %s' '!pitch 赤'
    echo '0: low-high, 1: high-low, 2: low-high-low 3: low-high-high-low, 4: low-high-high-high-low, etc.'
    exit 0
fi

export NO_TINY_URL=1
RESULT=$("$(dirname "$0")"/daijirin.sh "$@" | head -n 1 | sed 's/^\(.*\)(  )$/\1/')

if [[ ! $RESULT || $RESULT = '見つかりませんでした。' ]] || \
    ( ! printf '%s\n' "$RESULT" | grep -q '\([0-9]\+\)' ); then
    echo_ 'No pitch information available.'
    exit 0
fi

printf '%s\n' "${RESULT:0:300}"

exit 0
