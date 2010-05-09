#!/bin/bash

cd "$(dirname "$0")"

echo 'This script will download and preprocess the kanjidic file for use with the kanjidic script.
The file will be placed in the following directory:'
echo "$(readlink -e .)/"
read -p "Proceed? [y]" OK

[[ $OK && $OK != 'y' && $OK != 'Y' ]] && exit 1

echo 'Fetching kanjidic.gz...'
wget http://ftp.monash.edu.au/pub/nihongo/kanjidic.gz

echo 'Unzipping...'
gunzip kanjidic.gz

echo 'Encoding kanjidic to utf-8...'
iconv -f EUCJP -t UTF-8 kanjidic > kanjidic_utf8
mv kanjidic_utf8 kanjidic

echo 'Done.'

exit 0
