#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Dictionary lookup for Japanese words.

# Don't use this version, use ja.sh instead.  This is an experiment
# with sqlite and currently broken.  Technically the initialization of
# the database works but it's far too slow.
#
# The idea is to use an sqlite database instead of text files and
# grep.  The current implementation writes a suffix array into the
# database for faster lookups.  There are only two problems with this
# approach.
# 1) The suffix array could become too large.  A rough estimate puts
#    the size of the final database at a few 100MB.
# 2) It's too slow.  The sql() function can either write the
#    statements to a temporary file and call sqlite3 only once at the
#    end or it can call sqlite3 for each statement.  Calling sqlite3
#    for each statement is not possible because it's unimaginably
#    slow.  So it only has the choice to write the statements to an
#    intermediate file, but this file grows fast.  And it's still very
#    slow because processing a single line takes very long in bash.

. "$(dirname "$0")"/../gettext/gettext.sh

set -u

dict=$(dirname "$0")/dict.db

sql() {
    printf '%s\n' "$1" >> "$statement_buffer"
}
execute_sql() {
    sqlite3 "$dict" < "$statement_buffer"
    rm "$statement_buffer"
}

init_database() {
    statement_buffer=$(mktemp)
    echo "$statement_buffer"
    local source_dict="$1" entry_id=0 line kanji kana translation total_lines
    sql 'CREATE TABLE entries(id INTEGER PRIMARY KEY ASC);
        CREATE TABLE kanji(entry INTEGER, kanji, is_suffix INTEGER);
        CREATE TABLE kana(entry INTEGER, kana, is_suffix INTEGER);
        CREATE TABLE translation(entry INTEGER, translation, is_suffix INTEGER);'
    total_lines=$(wc -l "$source_dict" | cut -f 1 -d ' ')
    while read -r line; do
        kanji=${line%% *}
        if [[ $line =~ \[.*\] ]]; then
            kana=${line#*[}
            kana=${kana%%]*}
            translation=${line#*] /}
        else
            kana=$kanji
            kanji=
            translation=${line#* /}
        fi
        translation=${translation%EntL*/}

        IFS=';' kana=( $kana )
        IFS=';' kanji=( $kanji )
        IFS='/' translation=( $translation )

        sql "INSERT INTO entries VALUES ($entry_id);"
        [[ ${#kanji[@]} -ne 0 ]] || insert_suffixes "$entry_id" kanji "${kanji[@]}"
        insert_suffixes "$entry_id" kana "${kana[@]}"
        insert_suffixes "$entry_id" translation "${translation[@]}"

        let ++entry_id
        printf "%6d/%6d (%0.2f%%)\r" "$entry_id" "$total_lines" "$(echo "scale=4;$entry_id/$total_lines*100" | bc)"
    done < "$source_dict"
    echo
}

sql_escape() {
    s=${1//\"/\\\"}
    printf '%s' "${s//\\/\\\\}"
}

insert_suffixes() {
    [[ $# -ge 3 ]] || return
    local entry_id="$1" insert_statement="INSERT INTO $2 (entry,$2,is_suffix) VALUES " statement
    shift 2
    for t in "$@"; do
        for ((i=0;i<${#t};++i)) do
            statement+="$insert_statement ($entry_id,'$(sql_escape "${t:$i}")',$i);"
        done
    done
    sql "$statement"
}

if [[ ! -e $dict ]]; then
    if [[ ! ${source_dict:-} || ! -e $source_dict ]]; then
        echo_ 'Please run: source_dict=<source> ./ja.sh'
        exit 1
    fi
    init_database "$source_dict"
    exit
fi

exit 0
