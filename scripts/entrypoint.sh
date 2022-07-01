#!/usr/bin/env bash
set -eu

if [ "$TARGET" = "debug" ]; then
    cargo build
else
    cargo build --"$TARGET"
fi

exec ./target-snake/"$TARGET"/treesnake $@
