#!/usr/bin/env bash
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

MODE="$1"
shift
QUERY="$*"

if [[ $MODE != 'type' && $MODE != 'run' ]]; then
    echo_ "Please don't call this script directly."
    exit 1
fi

if [[ $QUERY = 'help' ]]; then
    echo_ 'Example: !calc 1+1'
    exit 0
fi

RESULT="$(mueval \
    "$([[ $MODE = 'type' ]] && echo '--inferred-type')" \
    --time-limit="$TIME_LIMIT_SECONDS" \
    --expression "$QUERY" 2>&1)"
MUEVAL_EXIT_CODE=$?

# Remove newlines and control characters.
RESULT=$(printf '%s\n' "${RESULT//$'\n'/ }" | tr --delete '\000-\037')

if [[ $MUEVAL_EXIT_CODE -ne 0 ]]; then
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
elif [[ $MODE = 'type' ]]; then
    RESULT=":: $RESULT"
fi

# Truncate result if too long.
# In any case prepend a space to prevent accidently calling other irc
# bots.
if [[ ${#RESULT} -ge $MAX_LINE_LENGTH ]]; then
    RESULT="${RESULT:0:$(( MAX_LINE_LENGTH - 4 ))}..."
fi
printf '%s\n' " $RESULT" | iconv -f utf8 -t latin1

exit 0
