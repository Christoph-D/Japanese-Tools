#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")"

if [[ -e kanjidic ]]; then
    exit 0
fi

echo 'Fetching kanjidic.gz...'
wget http://ftp.edrdg.org/pub/Nihongo/kanjidic.gz

echo 'Unzipping...'
gunzip kanjidic.gz

echo 'Encoding kanjidic to utf-8...'
iconv -f EUCJP -t UTF-8 kanjidic > kanjidic_utf8
mv kanjidic_utf8 kanjidic

echo 'Removing comments...'
grep -v '^#' kanjidic > kanjidic_no_comments
mv kanjidic_no_comments kanjidic

echo 'Done.'

exit 0
