#!/bin/bash

set -e

mkdir -p ~/persistent-config/opencode ~/persistent-config/opencode-state ~/.local/share/ ~/.local/state/
[[ -e ~/.local/share/opencode ]] || ln -s ~/persistent-config/opencode ~/.local/share/opencode
[[ -e ~/.local/state/opencode ]] || ln -s ~/persistent-config/opencode-state ~/.local/state/opencode
