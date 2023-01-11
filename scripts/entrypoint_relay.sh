#!/bin/sh
set -eu

ctlc() {
    echo ""
    echo "Caught Ctrl-C, exiting..."
    exit 1
}

trap ctlc INT TERM

ORIG_ARGS="$@"

for i in "$@"; do
    case "$i" in
        --worker-pod)
            WORKER_URL=$(echo "$2" | awk -F, '{print $1}')
            shift

            until curl --max-time 0.5 -sf -o /dev/null "$WORKER_URL"; do
                echo "Waiting for worker $WORKER_URL"
                sleep 0.5
            done
            ;;
        *)
            shift
            ;;
        esac
done

exec ./target-snake/"$TARGET"/conesnake $ORIG_ARGS
