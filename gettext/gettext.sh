#!/usr/bin/env bash

THIS_DIR=$(readlink -f "$(dirname "$BASH_SOURCE")")
export TEXTDOMAINDIR="$THIS_DIR/locale"
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
nprintf_() {
    local SINGULAR="$1"
    local PLURAL="$2"
    local N="$3"
    shift 3
    printf "$(ngettext "$SINGULAR" "$PLURAL" "$N")\n" "$@"
}
printf_no_newline_() {
    local FORMAT="$1"
    shift
    printf "$(gettext "$FORMAT")" "$@"
}
nprintf_no_newline_() {
    local SINGULAR="$1"
    local PLURAL="$2"
    local N="$3"
    shift 3
    printf "$(ngettext "$SINGULAR" "$PLURAL" "$N")" "$@"
}
