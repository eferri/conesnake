#! /usr/bin/env bash
set -eu

ctlc() {
    echo ""
    echo "Caught Ctrl-C, exiting..."
    exit 1
}

trap ctlc SIGINT SIGTERM

ORIG_ARGS="$@"

for i in "$@"; do
    case "$i" in
        --url)
            SNAKE_URL="$2"
            shift

            until curl --max-time 0.5 -sf -o /dev/null "$SNAKE_URL"; do
                echo "Waiting for snake $SNAKE_URL"
                sleep 0.5
            done
            ;;
        *)
            shift
            ;;
        esac
done

exec battlesnake play $ORIG_ARGS
