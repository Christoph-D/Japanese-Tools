#!/usr/bin/env bash

set -eu

cargo build --release
cp "${CARGO_TARGET_DIR-target}"/release/ai ai/
cp "${CARGO_TARGET_DIR-target}"/release/tokenizer tokenizer/
cp "${CARGO_TARGET_DIR-target}"/release/ircbot ircbot/

./gettext/regenerate_mo_files.sh || echo "Continuing without translations"

./kanjidic/prepare_kanjidic.sh || echo "Continuing without kanjidic"
./jmdict/prepare_jmdict.sh || echo "Continuing without jmdict"

if [[ ! -e jmdict/wadoku_prepared ]]; then
  echo
  echo "Warning: Cannot set up wadoku dictionary automatically."
  echo "Please download the latest XML dump from http://www.wadoku.de/wiki/display/WAD/Downloads+und+Links and extract it."
  echo "Then run ./jmdict/prepare_wadoku.sh <wadoku.xml>"
fi
