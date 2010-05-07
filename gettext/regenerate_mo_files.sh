#!/bin/bash

generate_mo() {
    msgfmt -o "$1/LC_MESSAGES/japanese_tools.mo" "$1/LC_MESSAGES/japanese_tools.po"
}

cd "$(dirname "$0")"
for LANG_CODE in $(find . -maxdepth 1 -type d -not -name . -printf '%f'); do
    echo -n "Generating mo file for language \"$LANG_CODE\"..."
    generate_mo $LANG_CODE && echo 'OK.' || echo 'failed.'
done

exit 0
