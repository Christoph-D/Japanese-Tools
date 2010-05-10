#!/bin/bash

generate_mo() {
    mkdir -p "locale/$1/LC_MESSAGES"
    msgfmt -o "locale/$1/LC_MESSAGES/japanese_tools.mo" "po/$1.po"
}

cd "$(dirname "$0")"
for LANG_CODE in $(find ./po -maxdepth 1 -type f -name '*.po' -printf '%f\n'); do
    LANG_CODE="${LANG_CODE%.po}"
    echo -n "Generating mo file for language \"$LANG_CODE\"..."
    generate_mo "$LANG_CODE" && echo 'OK.' || echo 'failed.'
done

exit 0
