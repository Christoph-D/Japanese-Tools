#!/usr/bin/env bash

# Install mueval

set -eu

curl --proto '=https' --tlsv1.2 -sSf https://get-ghcup.haskell.org | BOOTSTRAP_HASKELL_NONINTERACTIVE=y sh
source ~/.ghcup/env
cabal install --lib array bytestring show simple-reflect QuickCheck pretty containers mtl random
cabal install mueval
