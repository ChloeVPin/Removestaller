#!/bin/sh
# cargo-build.sh <source-root> <target-dir> <binary-name> <output-path> <profile>
set -eu

SRC="$1"
TARGET_DIR="$2"
BIN="$3"
OUTPUT="$4"
PROFILE="$5"

if [ "$PROFILE" = "release" ]; then
  cargo build --release --manifest-path "$SRC/Cargo.toml" --target-dir "$TARGET_DIR"
else
  cargo build --manifest-path "$SRC/Cargo.toml" --target-dir "$TARGET_DIR"
fi

cp "$TARGET_DIR/$PROFILE/$BIN" "$OUTPUT"
