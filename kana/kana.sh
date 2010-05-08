#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Hiragana and katakana trainer.

. "$(dirname "$0")"/../gettext/gettext.sh

if [ -z "$KANA_FILE" ]; then
    printf "$(gettext "Please don't call this script directly, use %s or %s instead.")\n" \
        "$(dirname "$0")/hira" "$(dirname "$0")/kata"
    exit 1
fi

# Configuration
TEMP_PATH="$DIRECTORY/tmp" # directory for temporary files
USER_STATS_DIR="$DIRECTORY/user_statistics" # this directory will be
                                            # filled with files
USER=$DMB_SENDER
CHANNEL_NAME=$DMB_RECEIVER

DEFAULT_RESULT_ITEMS=5
MAX_RESULT_ITEMS=30
LESSON_MAP=( 5 10 15 20 25 30 35 38 43 45 46 ) # line numbers in $KANA_FILE
# End of configuration

# Preliminary checks
if [[ ! -e $KANA_FILE ]]; then
   printf "$(gettext 'Please fix %s.')\n" '$KANA_FILE'
   exit 1
fi
# Cache kana file so we don't need to read it over and over again.
KANA_SRC=$(cat "$KANA_FILE")

# Try to create data directories.
[[ -d $USER_STATS_DIR ]] || mkdir -p "$USER_STATS_DIR"
if [[ ! -d $USER_STATS_DIR ]]; then
   printf "$(gettext 'Could not create directory %s. Please fix %s.')" \
       "$USER_STATS_DIR" '$USER_STATS_DIR'
   exit 2
fi
[[ -d $TEMP_PATH ]] || mkdir -p "$TEMP_PATH"
if [[ ! -d $TEMP_PATH ]]; then
   printf "$(gettext 'Could not create directory %s. Please fix %s.')" \
       "$TEMP_PATH" '$TEMP_PATH'
   exit 2
fi
if [[ -z $USER ]]; then
    printf "$(gettext 'Could not determine nick name. Please fix %s.')\n" '$USER'
    exit 1
fi
if [[ -z $CHANNEL_NAME ]]; then
    printf "$(gettext 'Could not determine channel name or query sender. Please fix %s.')\n" '$CHANNEL_NAME'
    exit 1
fi

# Prune old temporary files
find "$TEMP_PATH" -maxdepth 1 -type f -mmin +60 -exec rm '{}' \;

# Determine file names for the status and the solution file.
# First, turn slashes into || to make is safe for a file name.
CHANNEL_NAME=${CHANNEL_NAME////||}
SOLUTION_FILE=$TEMP_PATH/solution-$CHANNEL_NAME
LESSON_STATUS_FILE=$TEMP_PATH/status-$CHANNEL_NAME

# Prints the smaller of the two arguments.
min() { if (( "$1" < "$2" )); then echo "$1"; else echo "$2"; fi; }

show_help() {
    printf "$(gettext 'Start with "%s <level> [count]". Known levels are 0 to %s. To learn more about some level please use "%s help <level>".
To only see the differences between consecutive levels, please use "%s helpdiff <level>".')\n" \
        "$IRC_COMMAND" "$(( ${#LESSON_MAP[*]} - 1))" \
        "$IRC_COMMAND" "$IRC_COMMAND"
}

# Parameters: Lesson number
# Result:     The lesson and only the current lesson on stdout.
read_single_lesson() {
    if (( "$1" >= ${#LESSON_MAP[*]} )); then
        # If the lesson number is outside the valid range, don't print
        # anything.
        return
    fi

    local LESSON_LINES_START=0
    if (( "$1" > 0 )); then
        LESSON_LINES_START=${LESSON_MAP[$(( $1 - 1 ))]}
    fi
    echo "$KANA_SRC" | head -n ${LESSON_MAP[$1]} | tail -n +$(( $LESSON_LINES_START + 1))
}
# Parameters: Lesson number
# Result:     The lessons 0 to $1 on stdout.
read_lesson() {
    local UPTO=$(min "$1" "${#LESSON_MAP[*]}")
    for I in $(seq 0 $UPTO); do
        read_single_lesson "$I"
    done
}
# Parameters: Lesson number
# Result:     The lessons 0 to $1 on stdout. Each lesson may
#             appear multiple times in a row. See the
#             implementation for the general formula.
read_weighted_lessons() {
    local UPTO=$(min "$1" "${#LESSON_MAP[*]}")
    local COUNT=1
    for I in $(seq 0 $UPTO); do
        local L=$(read_single_lesson "$I")
        for J in $(seq 0 $COUNT); do
            echo "$L"
        done
        if (( $UPTO - $I < 5 )); then
            let "COUNT = COUNT * 2"
        fi
    done
}

# Parameters: lesson-number result-lines
start_lesson() {
    RESULT_LINES=$2
    # Sanitize $RESULT_LINES
    if echo "$RESULT_LINES" | grep -q -v -E '^[0-9]+$'; then
        RESULT_LINES=$DEFAULT_RESULT_ITEMS
    elif (( $RESULT_LINES < 1 )); then
        RESULT_LINES=1
    elif (( $RESULT_LINES > $MAX_RESULT_ITEMS )); then
        RESULT_LINES=$MAX_RESULT_ITEMS
    fi
    # Generate the lesson, i.e. shuffle it and restrict it to
    # $RESULT_LINES lines.
    LESSON=$(read_weighted_lessons "$1" | shuf | head -n "$RESULT_LINES")

    KANA=$(echo "$LESSON" | cut -d ' ' -f 1)
    KANA=${KANA//$'\n'/ }

    printf "$(gettext 'Please write in romaji: %s')\n" "$KANA"
    echo "$LESSON" > "$SOLUTION_FILE"
    # Save the status so we can generate a similar question again
    # after this one has been answered.
    echo "$1 $RESULT_LINES" > "$LESSON_STATUS_FILE"
}
# Result: $USER_STATS
read_user_statistics() {
    USER=${USER////} # remove slashes
    USER_FILE="$USER_STATS_DIR"/"$USER"
    if [[ -f $USER_FILE ]]; then
        USER_STATS=( $(cat "$USER_FILE") )
    else
        USER_STATS=( 0 0 )
    fi
}
# Paramaters: $USER_STATS
print_user_statistics() {
    if (( ${USER_STATS[1]} > 0 )); then
        local PERCENT=$(echo "scale=2; ${USER_STATS[0]} * 100 / ${USER_STATS[1]}" | bc)
        printf "$(gettext 'Statistics for %s: %s%% of %s characters correct.')\n" \
            "$USER" "$PERCENT" "${USER_STATS[1]}"
    else
        printf "$(gettext 'No statistics available for %s.')\n" "$USER"
    fi
}

if [[ $* = "help" ]]; then
    show_help
    exit 0
fi

QUERY=( $@ )
if [[ ${QUERY[0]} = "stats" ]]; then
    if [[ -n ${QUERY[1]} ]]; then
        USER=${QUERY[1]}
    fi
    read_user_statistics
    print_user_statistics
    exit 0
fi

QUERY_BEGIN="${QUERY[0]} ${QUERY[1]}"
if echo "$QUERY_BEGIN" | grep -q -E '^help [0-9]+$'; then
    # Lesson help
    LESSON=$(read_lesson "${QUERY[1]}")
    LESSON=${LESSON// /=}
    echo "${LESSON//$'\n'/ }"
    exit 0
elif echo "$QUERY_BEGIN" | grep -q -E '^helpdiff [0-9]+$'; then
    # Lesson helpdiff
    LESSON_DIFF=$(read_single_lesson "${QUERY[1]}")
    LESSON_DIFF=${LESSON_DIFF// /=}
    LESSON_DIFF=${LESSON_DIFF//$'\n'/ }
    if [ -n "$LESSON_DIFF" ]; then
        echo "$LESSON_DIFF"
    else
        echo "$(gettext 'No diff available. :-(')"
    fi
    exit 0
fi

if echo "$QUERY_BEGIN" | grep -q -E '^[0-9]+ [0-9]*$'; then
    start_lesson "${QUERY[0]}" "${QUERY[1]}"
    exit 0
fi

if [[ ! -f "$SOLUTION_FILE" ]]; then
    show_help
    exit 0
fi

SOLUTION=( $(cut -d ' ' -f 2 "$SOLUTION_FILE") )
KANA_SOLUTION=( $(cut -d ' ' -f 1 "$SOLUTION_FILE") )
EXPECTED_NUMBER=${#SOLUTION[*]}
rm "$SOLUTION_FILE"

CORRECT=0
PRETTY_SOLUTION=
for I in $(seq 0 $(( $EXPECTED_NUMBER - 1 ))); do
    EXPECTED=${SOLUTION[$I]}
    GOT=${QUERY[$I]}
    if [[ $EXPECTED = $GOT ]]; then
        let "++CORRECT"
    else
        PRETTY_SOLUTION="$PRETTY_SOLUTION ${KANA_SOLUTION[$I]}=$EXPECTED"
    fi
done
if (( $CORRECT == 0 )); then
    printf "$(gettext 'Unfortunately, no character was right. Solution:%s.') " "$PRETTY_SOLUTION"
elif (( $CORRECT == $EXPECTED_NUMBER )); then
    printf "$(gettext 'Perfect! %s of %s.') " \
        "$CORRECT" "$EXPECTED_NUMBER"
else
    printf "$(gettext 'Correct: %s of %s, Corrections:%s.') " \
        "$CORRECT" "$EXPECTED_NUMBER" "$PRETTY_SOLUTION"
fi

# update user statistics
read_user_statistics
USER_STATS[0]=$(( ${USER_STATS[0]} + $CORRECT ))
USER_STATS[1]=$(( ${USER_STATS[1]} + $EXPECTED_NUMBER ))
print_user_statistics
echo "${USER_STATS[*]}" > "$USER_FILE"

# Start new lesson.
if [[ -f $LESSON_STATUS_FILE ]]; then
    # Note: No quoting here because start_lesson expects 2 parameters.
    start_lesson $(cat "$LESSON_STATUS_FILE")
fi

exit 0
