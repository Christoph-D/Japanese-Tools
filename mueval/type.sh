#!/bin/bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script determines the type of Haskell expressions using mueval:
# http://hackage.haskell.org/package/mueval

exec "$(dirname "$0")"/eval.sh type "$@"
