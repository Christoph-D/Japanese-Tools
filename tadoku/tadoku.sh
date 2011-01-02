#!/bin/bash

set -e -u

RANKING_URL='http://digitalartificer.com/~silent/ranking.php'
BREAKDOWN_URL='http://digitalartificer.com/~silent/breakdown.php?name=%s'

USER="${1-}"
if ! printf '%s\n' "$USER" | grep -q '^[a-zA-Z_0-9]*$'; then
    echo 'Invalid user name.'
    exit 0
fi

if [[ $USER = 'help' ]]; then
    echo 'This command shows the current tadoku ranking taken from http://digitalartificer.com/~silent/ranking.php .'
    echo 'To learn more about tadoku please visit http://readmod.wordpress.com/ .'
    exit 0
fi

RANKING=$(curl --silent "$RANKING_URL" | \
    sed "s/<li>/\n/g" | \
    sed "s/^<a href='[^']*'>\([^ ]*\) .* \([0-9]\+\.\?[0-9]*\).*$/\1 \2/;t;d")

if [[ ! $USER  ]]; then
    RANKING=$(printf '%s\n' "$RANKING" | \
        head -n 10)
    printf 'Top 10: %s\n' "${RANKING//$'\n'/, }"
    exit 0
fi

RANKING_LINE=$(printf '%s\n' "$RANKING" | grep -ni "^$USER " | sed 's/^\([0-9]\+\).\([^ ]*\) \(.*\)/\1\t\2\t\3/')
if [[ ! $RANKING_LINE ]]; then
    printf 'Unknown user: %s\n' "$USER"
    exit 0
fi
read POSITION USER PAGES < <(printf '%s\n' "$RANKING_LINE")

BREAKDOWN=$(curl --silent $(printf "$BREAKDOWN_URL" "$USER"))
PERCENTS=$(printf '%s' "$BREAKDOWN" | \
    sed "s/ *<td>\([0-9]\+\.\?[0-9]*%\)<.*$/\1/;t;d")
AVERAGE=$(printf '%s' "$BREAKDOWN" | \
    sed "s/^\([0-9]\+\.\?[0-9]*\)<.*$/\1/;t;d")

printf '%s is at position %s with %s pages and daily reading average of %s pages.\n' \
    "$USER" "$POSITION" "$PAGES" "$AVERAGE"

TAGS='Books
Manga
Net
Fullgame
Game
Lyric
Subs
News
Sentences
Nico'

PERCENTS=$(paste -d' ' <(echo "$TAGS") <(echo "$PERCENTS") | grep -v '^[^ ]* 0%$')
printf 'Breakdown: %s\n' "${PERCENTS//$'\n'/, }"

exit 0
