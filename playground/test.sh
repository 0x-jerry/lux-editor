#!/usr/bin/env bash
# build script
set -euo pipefail

OUT_DIR="./target"
count=3

build() {
  local name="$1"
  echo "building ${name}..." # comment
  cargo build --release --bin "$name" || exit 1
}

for f in a b c; do
  build "$f" && echo "$f ok"
done
