#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script evaluates Haskell expressions using mueval:
# http://hackage.haskell.org/package/mueval

# shellcheck source=gettext/gettext.sh
. "$(dirname "$0")"/../gettext/gettext.sh

MAX_LINE_LENGTH=200
TIME_LIMIT_SECONDS=6

if ! command -v mueval &>/dev/null; then
    echo_ "Couldn't find mueval. Please run install_mueval.sh from the project root."
    exit 1
fi

if ! command -v firejail &>/dev/null; then
    echo_ "Couldn't find firejail. Please install firejail."
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

RESULT="$(firejail \
    --name=mueval \
    --quiet \
    --noprofile \
    --private-tmp \
    --private-dev \
    --private-etc=invalid \
    --private-cwd=/ \
	--read-only=$HOME \
	--noblacklist="$HOME/.cabal" \
	--noblacklist="$HOME/.ghc" \
    --blacklist="$HOME/*" \
	--restrict-namespaces \
    --disable-mnt \
    --no3d \
    --nodvd \
    --nogroups \
    --noinput \
    --noroot \
    --nosound \
    --notv \
    --nou2f \
    --novideo \
    --net=none \
    --caps.drop=all \
    --nonewprivs \
    --seccomp \
    --nodbus \
    --x11=none \
    --rlimit-cpu="$TIME_LIMIT_SECONDS" \
    --rlimit-nofile=300 \
    --rlimit-nproc=100 \
    --oom=1000 \
    mueval \
    $([[ $MODE = 'type' ]] && echo '--inferred-type --type-only') \
    --time-limit="$TIME_LIMIT_SECONDS" \
    --expression "$QUERY" 2>&1)"
MUEVAL_EXIT_CODE=$?

if [[ $MODE = 'type' ]]; then
  RESULT=$(printf '%s' "$RESULT" | tail -n +2)
fi

# Remove newlines and control characters.
RESULT=$(printf '%s\n' "${RESULT//$'\n'/ }" | tr --delete '\000-\037')

if [[ $MUEVAL_EXIT_CODE -ne 0 ]]; then
    if [[ -z $RESULT ]]; then
        RESULT=$(_ 'An error occurred.')
    elif [[ $RESULT = '*Exception: <<timeout>>' ]]; then
        RESULT=$(_ 'Time limit exceeded.')
    elif [[ $RESULT =~ ^mueval(-core)?:  ]]; then
        if [[ $RESULT =~ memory ]]; then
            RESULT=$(_ 'Memory limit exceeded.')
        elif [[ $RESULT =~ time ]]; then
            RESULT=$(_ 'Time limit exceeded.')
        else
            RESULT=$(_ 'An error occurred.')
        fi
    fi
    RESULT=${RESULT//<no location info>:/}
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
printf '%s\n' " $RESULT"

exit 0
