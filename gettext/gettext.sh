#!/bin/bash

THIS_DIR=$(readlink -e "$(dirname "$BASH_SOURCE")")
export TEXTDOMAINDIR="$THIS_DIR"
export TEXTDOMAIN=japanese_tools
