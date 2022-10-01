#!/bin/sh
set -eu

ctlc() {
    echo ""
    echo "Caught Ctrl-C, exiting..."
    exit 1
}

trap ctlc INT TERM

if [ "$TARGET" = "debug" ]; then
    cargo build
else
    cargo build --"$TARGET"
fi

exec ./target-snake/"$TARGET"/treesnake $@
