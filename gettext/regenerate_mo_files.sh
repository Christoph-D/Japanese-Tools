#!/usr/bin/env bash
set -eu

generate_mo() {
    mkdir -p "locale/$1/LC_MESSAGES"
    msgfmt -o "locale/$1/LC_MESSAGES/japanese_tools.mo" "po/$1.po"
}

cd "$(dirname "$0")"
while IFS= read -r -d '' LANG_CODE; do
    LANG_CODE="${LANG_CODE%.po}"
    echo -n "Generating mo file for language \"$LANG_CODE\"..."
    generate_mo "$LANG_CODE" && echo 'OK.' || echo 'failed.'
done < <(find ./po -maxdepth 1 -type f -name '*.po' -printf '%f\0')

exit 0
