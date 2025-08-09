#!/usr/bin/env bash
set -eu

THIS_DIR=$(basename "$(readlink -f "$(dirname "$0")")")
cd "$(dirname "$0")"/../
find . \
     -path "./$THIS_DIR" -prune \
     -or -path "./.git" -prune \
     -or -path "./venv" -prune \
     -or -path "./ai" -prune \
     -or -path "./ircbot" -prune \
     -or \( -type f -executable \
     -exec xgettext -d japanese_tools -p "./$THIS_DIR" --from-code=UTF-8 \
     --keyword=_ --keyword=echo_ \
     --keyword=printf_ --keyword=printf_no_newline_ \
     --keyword=nprintf_:1,2 --keyword=nprintf_no_newline_:1,2 \
     --sort-by-file '{}' + \)
cd "$THIS_DIR"

POT_FILE=japanese_tools.pot

mv japanese_tools.po "$POT_FILE"
xtr ../ai/src/main.rs ../ircbot/src/main.rs --keywords formatget --keywords gettext --keywords ngettext:1,2 --omit-header -o rust.po
cat rust.po >> "${POT_FILE}"
rm rust.po

sed -i '1,+17s/charset=CHARSET/charset=UTF-8/' "$POT_FILE"
sed -i '1,+17s/^"POT-Creation-Date: .*\\n"$//;T;d' "$POT_FILE"
sed -i '1,+17s/^"Project-Id-Version: PACKAGE VERSION\\n"//;T;d' "$POT_FILE"
sed -i '1,+17s/^"Language-Team: LANGUAGE <LL@li\.org>\\n"$//;T;d' "$POT_FILE"
sed -i '1,+17s/^"Last-Translator: FULL NAME <EMAIL@ADDRESS>\\n"$/"Last-Translator: Christoph Dittmann <github@christoph-d.de>\\n"/' "$POT_FILE"
# Remove line numbers so we don't need to update the language files
# for every little code change.
sed -i '/^#: /s/:[0-9]\+\($\| \)/\1/g' "$POT_FILE"

merge_messages() {
    msgmerge --quiet --backup=none --update "po/$1.po" "$POT_FILE"
}

while IFS= read -r -d '' LANG_CODE; do
    LANG_CODE="${LANG_CODE%.po}"
    echo -n "Updating po file for language \"$LANG_CODE\"..."
    merge_messages "$LANG_CODE" && echo 'OK.' || echo 'failed.'
done < <(find ./po -maxdepth 1 -type f -name '*.po' -printf '%f\0')

exit 0
