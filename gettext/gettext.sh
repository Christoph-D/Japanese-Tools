#!/bin/bash

THIS_DIR=$(readlink -e "$(dirname "$BASH_SOURCE")")
export TEXTDOMAINDIR="$THIS_DIR"
export TEXTDOMAIN=japanese_tools

_() {
    gettext "$1"
}
echo_() {
    local FIRST="$1"
    shift
    echo "$(gettext "$FIRST")" "$@"
}
printf_() {
    local FORMAT="$1"
    shift
    printf "$(gettext "$FORMAT")\n" "$@"
}
printf_no_newline_() {
    local FORMAT="$1"
    shift
    printf "$(gettext "$FORMAT")" "$@"
}
