#!/usr/bin/env bash
# Simulates exactly what cutechess sends for a tc=60+2 game, move 1.
cd "$(dirname "$0")/.." || exit 1

BINARY="${1:-./target/release/uci.exe}"
TIMEOUT=20

echo "=== Pinging: $BINARY ==="

output=$(printf 'uci\nisready\nucinewgame\nisready\nposition startpos\nisready\ngo wtime 60000 btime 60000 winc 2000 binc 2000\n' \
    | timeout $TIMEOUT "$BINARY" 2>&1)

echo "$output"
echo "---"

if echo "$output" | grep -q "^bestmove"; then
    echo "PASS: got bestmove"
else
    echo "FAIL: no bestmove within ${TIMEOUT}s"
fi
