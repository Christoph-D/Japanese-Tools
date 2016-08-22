#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Dictionary lookup for Japanese words with jmdict.

. "$(dirname "$0")"/../gettext/gettext.sh

set -u

export DICT=$(dirname "$0")/JMdict_e_prepared

if [[ ! -e $DICT ]]; then
   printf_ 'Please run: %s' './prepare_jmdict.sh'
   exit 1
fi

exec "$(dirname "$0")"/ja.sh "$@"
