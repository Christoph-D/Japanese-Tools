#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script wraps cdecl and c++decl.

. "$(dirname "$0")"/../gettext/gettext.sh

set -u
MAX_LINE_LENGTH=250

QUERY=${2:0:100}

if [[ "$1" = 'cdecl' ]]; then
    PROGRAM=cdecl
else
    PROGRAM=c++decl
fi

if [[ $# -ne 2 || ! $QUERY =~ ^(explain|declare)' ' ]]; then
    printf_ 'Usage: !%s [explain|declare] <something>' "$PROGRAM"
    exit 0
fi

RESULT=$(printf '%s\n' "$QUERY" | $PROGRAM)
RESULT=${RESULT//$'\n'/, }

# Restrict length if necessary.
if [[ ${#RESULT} -ge $(( $MAX_LINE_LENGTH - 3 )) ]]; then
    RESULT="${RESULT:0:$(( $MAX_LINE_LENGTH - 3 ))}"
    RESULT=$(printf '%s\n' "$RESULT")...
fi

printf '%s\n' "> $RESULT"

exit 0
