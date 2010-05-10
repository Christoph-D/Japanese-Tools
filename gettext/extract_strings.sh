#!/bin/bash

THIS_DIR=$(basename "$(readlink -e "$(dirname "$0")")")
cd "$(dirname "$0")"/../
xgettext -d japanese_tools -p "./$THIS_DIR" --from-code=UTF-8 \
    --keyword=_ --keyword=echo_ \
    --keyword=printf_ --keyword=printf_no_newline_ \
    $(\
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
    msgmerge --quiet --backup=none --update "po/$1.po" "$POT_FILE"
}

for LANG_CODE in $(find ./po -maxdepth 1 -type f -name '*.po' -printf '%f\n'); do
    LANG_CODE="${LANG_CODE%.po}"
    echo -n "Updating po file for language \"$LANG_CODE\"..."
    merge_messages "$LANG_CODE" && echo 'OK.' || echo 'failed.'
done

exit 0
