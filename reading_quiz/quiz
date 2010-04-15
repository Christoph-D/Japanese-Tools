#!/bin/bash

set -u -e

DATA_DIR=$(dirname "$0")/data
VOCAB_DIR=$(dirname "$0")/vocabulary

IRC_COMMAND='!quiz'

USER=${DMB_SENDER//\'/}
CHANNEL_NAME=${DMB_RECEIVER-}

# Make sure we have a UTF-8 environment. Otherwise sed etc. will not
# work on utf-8 strings.
export LANG=en_US.UTF-8

QUERY="$*"
# Strip whitespace.
QUERY="$(printf '%s\n' "$QUERY" | sed 's/\(^[ 　]*\|[ 　]*$\)//g')"

[[ -d $DATA_DIR ]] || mkdir -p "$DATA_DIR"
[[ -d $VOCAB_DIR ]] || mkdir -p "$VOCAB_DIR"

if [[ ! $USER ]]; then
    echo 'Could not determine nick name. Please fix $USER.'
    exit 1
fi
if [[ ! $CHANNEL_NAME ]]; then
    echo 'Could not determine channel name or query sender. Please fix $CHANNEL_NAME.'
    exit 1
fi

TIMER_FILE="$DATA_DIR/timer.key.$CHANNEL_NAME"
STATS_DB="$DATA_DIR/stats.db"
QUESTION_FILE="$DATA_DIR/question.status.$CHANNEL_NAME"

# Checks if $1 is a valid level.
check_level() {
    [[ -s $VOCAB_DIR/$1.txt ]]
}

# Starts a timer. Delay in seconds is $1.
set_timer() {
    local TIMER_KEY=$RANDOM$RANDOM$RANDOM$RANDOM
    echo "$TIMER_KEY" > $TIMER_FILE
    echo "/timer $1 $TIMER_KEY"
}

# $1 is the level. Loads a random line out that list.
load_source_line() {
    sort --random-sort "$VOCAB_DIR/$1.txt" | head -n 1
}

split_lines() {
    KANJI=$(printf '%s\n' "$1" | head -n 1)
    READINGS=$(printf '%s\n' "$1" | head -n 2 | tail -n 1)
    MEANING=$(printf '%s\n' "$1" | head -n 3 | tail -n 1)
}

# $1 is the level. Returns non-zero on invalid levels.
ask_question() {
    check_level "$1" || return 1
    SOURCE=$(load_source_line "$1")
    split_lines "${SOURCE//|/$'\n'}"
    printf '%s\n%s\n%s\n%s\n' "$KANJI" "$READINGS" "$MEANING" "$1" > "$QUESTION_FILE"
    echo "Please read: $KANJI"
}

sql() {
    sqlite3 "$STATS_DB" "$1" 2> /dev/null
}

# $1 = 0 is "wrong answer" and $1 = 1 is "correct answer".
record_answer() {
    sql 'CREATE TABLE IF NOT EXISTS user_stats (
user NOT NULL,
word NOT NULL,
correct NOT NULL,
timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP );'
    sql "INSERT INTO user_stats (user, word, correct) VALUES ('$USER', '$KANJI', $1);"
}

get_user_stats() {
    local STATS=$(sql "SELECT correct,COUNT(*) FROM user_stats WHERE user = '$1'
AND julianday(timestamp) > julianday('now', '-2 month')
GROUP BY correct ORDER BY correct ASC;")
    local WRONG=$(echo "$STATS" | grep -m 1 '^0|' | sed 's/^0|//')
    local CORRECT=$(echo "$STATS" | grep -m 1 '^1|' | sed 's/^1|//')
    if [[ ! $WRONG && ! $CORRECT ]]; then
        echo "Unknown user: $1"
        return 1
    fi
    WRONG=${WRONG-0}
    CORRECT=${CORRECT-0}
    local TOTAL=$(( $WRONG + $CORRECT ))
    local PERCENT=$(echo "scale=2; $CORRECT * 100 / ($TOTAL)" | bc)
    printf 'In the last 2 months, %s answered %s/%s questions correctly, that is %s%%.\n' "$1" "$CORRECT" "$TOTAL" "$PERCENT"
    local HARD_WORDS=$(sql "SELECT word, COUNT(*) FROM user_stats WHERE user = '$1' AND correct = 0 
AND julianday(timestamp) > julianday('now', '-2 month')
GROUP BY word ORDER BY COUNT(*) DESC LIMIT 10;" | \
        sed 's/^\([^|]*\)|\(.*\)$/\1 (\2)/')
    [[ $HARD_WORDS ]] && echo "Hardest words for $1 (number of mistakes): ${HARD_WORDS//$'\n'/, }"
}

# Checks if $1 is a correct answer.
check_if_answer() {
    if [[ ! -s $QUESTION_FILE ]]; then
        echo 'Please specify a level.'
        return
    fi
    PROPOSED="${1// /}"
    split_lines "$(cat "$QUESTION_FILE")"
    IFS=','
    for R in $READINGS; do
        if [[ $R = $PROPOSED ]]; then
            echo "$USER: Correct! ($READINGS: $MEANING)"
            record_answer 1
            # Ignore additional answers for a few seconds.
            set_timer 2
            return 0
        fi
    done
    echo "$USER: Sadly, no."
    record_answer 0
}

# Handle the help command.
if [[ ! $QUERY || $QUERY = 'help' ]]; then
    echo "Try \"$IRC_COMMAND jlpt4\". With \"$IRC_COMMAND skip\" you can skip questions."
    echo "Statistics can be accessed by \"$IRC_COMMAND stats <nickname>\"."
    exit 0
fi

# Handle the stats command.
if printf '%s\n' "$QUERY" | grep -q '^stats'; then
    if printf '%s\n' "$QUERY" | grep -q '^stats \+[][a-zA-Z0-9|_-`]\+$'; then
        get_user_stats "$(printf '%s\n' "$QUERY" | sed 's/^stats \+//')"
    else
        echo "Usage: $IRC_COMMAND stats <nickname>"
    fi
    exit 0
fi

# Handle the timer.
if [[ -s $TIMER_FILE ]]; then
    if [[ ! $(find "$TIMER_FILE" -cmin 1) ]]; then
        rm "$TIMER_FILE"
    else
        # The timer is running, so ignore answers.
        if [[ $(cat "$TIMER_FILE") = $QUERY ]]; then
            rm "$TIMER_FILE"
            # The timer expired. Ask next question.
            ask_question "$(tail -n 1 "$QUESTION_FILE")"
        fi
        exit 0
    fi
fi

# Handle the skip/next command.
if printf '%s\n' "$QUERY" | grep -q '^\(next\|skip\) *$'; then
    # Display answer and skip current question.
    if [[ ! -s $QUESTION_FILE ]]; then
        echo "Nothing to skip!"
        exit 0
    fi
    split_lines "$(cat "$QUESTION_FILE")"
    echo "Skipping $KANJI [$READINGS]..."
    set_timer 2
    exit 0
fi

# Handle answers.
if echo "$QUERY" | grep -v -q '^[][a-zA-Z0-9 ./-_`]\+$'; then
    # $QUERY contains non-latin characters, so assume it's an answer.
    check_if_answer "$QUERY"
    exit 0
fi

# The only remaining possibility is that $QUERY contains a level.
if ! ask_question "$QUERY"; then
    for LEVEL in "$VOCAB_DIR"/*.txt; do
        BASE_NAME="$(basename "$LEVEL" | sed 's/\.txt$//')"
        LINE_COUNT="$(wc -l "$LEVEL" | cut -d ' ' -f 1)"
        VALID_LEVELS="${VALID_LEVELS:+$VALID_LEVELS$'\n'}$BASE_NAME ($LINE_COUNT)"
    done
    echo "Unknown level \"$QUERY\". Valid levels (number of words): ${VALID_LEVELS//$'\n'/, }"
fi

exit 0
