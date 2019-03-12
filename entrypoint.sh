#!/usr/bin/env bash
set -eu

if [ "$TARGET" = "release" ]; then
    cargo build --release
    N_THREADS="$(nproc)"
else
    cargo build
    N_THREADS="1"
fi

exec ./target-snake/"$TARGET"/treesnake --num-threads "$N_THREADS" $@
