#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script evaluates Haskell expressions using mueval:
# http://hackage.haskell.org/package/mueval

. "$(dirname "$0")"/../gettext/gettext.sh

MAX_LINE_LENGTH=200
TIME_LIMIT_SECONDS=6

if [[ ! -x $(which mueval) ]]; then
    printf_ 'Please install mueval: %s' 'http://hackage.haskell.org/package/mueval'
    exit 1
fi

QUERY="$*"

if [[ $QUERY = 'help' ]]; then
    echo_ 'Example: !calc 1+1'
    exit 0
fi

# Prepend a space to prevent calling other irc bots.
RESULT=" $(mueval --rlimits --timelimit="$TIME_LIMIT_SECONDS" --expression "$QUERY" 2>&1)"
# Remove newlines.
RESULT=${RESULT//$'\n'/ }
# Remove all control characters.
RESULT=$(printf '%s\n' "$RESULT" | tr --delete '\000-\037')

if [[ $? -ne 0 ]]; then
    if printf '%s' "$RESULT" | grep -q '^mueval\(-core\)\?: '; then
        if printf '%s' "$RESULT" | grep -q 'memory'; then
            RESULT=$(_ 'Memory limit exceeded.')
        else
            RESULT=$(_ 'Time limit exceeded.')
        fi
    fi
    RESULT=${RESULT//<no location info>:/}
    if printf '%s' "$RESULT" | \
        grep -q '^No instance for (GHC\.Show\.Show (\(GHC\.IOBase\.\)\?IO '; then
        RESULT="IO not allowed."
    fi
    # Remove multiple spaces and other unnecessary parts.
    RESULT=$(printf '%s\n' "$RESULT" | \
        sed 's/(possibly incorrect indentation)//g' | \
        sed 's/ at <interactive>:[^ ].*$//' | \
        sed 's/GHC\.\(Types\.\)\?//g' | \
        sed 's/ \+/ /g' | \
        sed 's/\(^ \+\)\|\( \+$\)//g')
fi

# Truncate result if too long.
if [[ ${#RESULT} -gt $MAX_LINE_LENGTH ]]; then
    RESULT="${RESULT:0:$(( $MAX_LINE_LENGTH-3 ))}..."
fi
printf '%s\n' "$RESULT"

exit 0
