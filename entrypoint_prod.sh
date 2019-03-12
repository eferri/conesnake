#!/usr/bin/env bash
set -eu

N_THREADS="$(nproc)"
exec /app/treesnake --num-threads "$N_THREADS" $@
