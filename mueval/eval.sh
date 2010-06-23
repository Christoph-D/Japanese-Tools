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

RESULT="$(mueval --rlimits \
    $([[ $MODE = 'type' ]] && echo '--inferred-type') \
    --timelimit="$TIME_LIMIT_SECONDS" \
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
    if printf '%s\n' "$QUERY" | grep -q ' '; then
        QUERY="($QUERY)"
    fi
    INFERRED_TYPE="$RESULT"
    RESULT="$QUERY :: $INFERRED_TYPE"
    # Leave room for $QUERY of at least 4 characters. The space in
    # front plus the " :: " part makes 9 characters in total.
    if [[ ${#INFERRED_TYPE} -ge $(( $MAX_LINE_LENGTH - 9 )) ]]; then
        RESULT=":: $INFERRED_TYPE"
    elif [[ ${#RESULT} -ge $MAX_LINE_LENGTH ]]; then
        RESULT="${QUERY:0:$(( $MAX_LINE_LENGTH - ${#INFERRED_TYPE} - 8 ))}... :: $INFERRED_TYPE"
    fi
fi

# Truncate result if too long.
# In any case prepend a space to prevent accidently calling other irc
# bots.
if [[ ${#RESULT} -ge $MAX_LINE_LENGTH ]]; then
    RESULT="${RESULT:0:$(( $MAX_LINE_LENGTH-4 ))}..."
fi
printf '%s\n' " $RESULT"

exit 0
