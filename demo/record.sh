#!/usr/bin/env bash
# Records the README's screenshots (docs/images/{mocha,latte}/) and demo
# GIF (docs/images/demo.gif) with VHS, each from a fresh demo history.
# Needs vhs and the Iosevka font, and ffmpeg (which vhs installs); see
# demo/README.md.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
out="$root/docs/images"
tmp=$(mktemp -d)

cargo build --release --locked --quiet
cargo build --release --locked --quiet --example demo_data
export PATH="$root/target/release:$PATH"

# record TAPE THEME VHS_THEME
record() {
  local tape="$tmp/$1-$2.tape" data="$tmp/data-$1-$2"
  sed -e "s|@OUT@|$out|g" -e "s|@TMP@|$tmp|g" -e "s|@THEME@|$2|g" -e "s|@VHS_THEME@|$3|g" \
    "demo/$1.tape" > "$tape"
  "$root/target/release/examples/demo_data" "$data"
  mkdir -p "$out/$2"
  echo "recording $1 ($2)"
  ANZAN_DATA_DIR="$data" ANZAN_CONFIG="$root/demo/$2.toml" vhs --quiet "$tape"
}

# questions THEME: the four question screenshots, cropped to the question
# and tiled two by two.
questions() {
  local crop="crop=800:420:400:210"
  ffmpeg -loglevel error -y \
    -i "$tmp/$1-question-multiplication.png" -i "$tmp/$1-question-fraction.png" \
    -i "$tmp/$1-question-square-root.png" -i "$tmp/$1-question-square.png" \
    -filter_complex "[0]$crop[a];[1]$crop[b];[2]$crop[c];[3]$crop[d];[a][b]hstack[t];[c][d]hstack[u];[t][u]vstack" \
    "$out/$1/questions.png"
}

record screenshots mocha "Catppuccin Mocha"
questions mocha
record screenshots latte "Catppuccin Latte"
questions latte
record demo mocha "Catppuccin Mocha"
