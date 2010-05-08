#!/bin/bash

THIS_DIR=$(basename "$(readlink -e "$(dirname "$0")")")
cd "$(dirname "$0")"/../
xgettext -E -d japanese_tools -p "./$THIS_DIR" --from-code=UTF-8 $(\
    find . \
    -path "./$THIS_DIR" -prune \
    -or -path "./.git" -prune \
    -or \( -type f -executable -print \)
)
cd "$THIS_DIR"

POT_FILE=japanese_tools.pot

mv japanese_tools.po "$POT_FILE"

sed -i 's/charset=CHARSET\\n"$/charset=UTF-8\\n"/' "$POT_FILE"
sed -i 's/"Project-Id-Version: PACKAGE VERSION\\n"//' "$POT_FILE"
sed -i 's/^"Language-Team: LANGUAGE <LL@li\.org>\\n"$//' "$POT_FILE"
sed -i 's/^"Last-Translator: FULL NAME <EMAIL@ADDRESS>\\n"$/"Last-Translator: Christoph Dittmann <github@christoph-d.de>\\n"/' "$POT_FILE"

merge_messages() {
    msgmerge --quiet --update "$1/LC_MESSAGES/japanese_tools.po" "$POT_FILE"
}

for LANG_CODE in $(find . -maxdepth 1 -type d -not -name . -printf '%f'); do
    echo -n "Updating po file for language \"$LANG_CODE\"..."
    merge_messages $LANG_CODE && echo 'OK.' || echo 'failed.'
done

exit 0
