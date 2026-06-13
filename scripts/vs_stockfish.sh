#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1
# ── Paths ─────────────────────────────────────────────────────────────────────
CUTECHESS="/c/Program Files (x86)/Cute Chess/cutechess-cli.exe"
BITCHESS="./target/release/uci.exe"
STOCKFISH="/c/Program Files/stockfish/stockfish-windows-x86-64-avx2.exe"

# ── Match settings ─────────────────────────────────────────────────────────────
GAMES=100
TC="60+1"            # time control: 1 minute + 2 second increment
STOCKFISH_ELO=2000   # min 1320, max 3190

# ── Args: [-n name] [-d description] [-g games] [-t tc] [-e elo] ─────────────
NAME=""
DESCRIPTION=""
while getopts "n:d:g:t:e:" opt; do
    case "$opt" in
        n) NAME="$OPTARG" ;;
        d) DESCRIPTION="$OPTARG" ;;
        g) GAMES="$OPTARG" ;;
        t) TC="$OPTARG" ;;
        e) STOCKFISH_ELO="$OPTARG" ;;
        *) echo "Usage: $0 [-n name] [-d description] [-g games] [-t tc] [-e elo]"; exit 1 ;;
    esac
done

mkdir -p results

if [ -z "$NAME" ]; then
    N=1
    while [ -f "results/vs_stockfish${N}.pgn" ]; do N=$((N + 1)); done
    OUTPUT="results/vs_stockfish${N}.pgn"
else
    OUTPUT="results/${NAME}.pgn"
fi

# ── Build ──────────────────────────────────────────────────────────────────────
cargo build --release --bin uci 2>&1
if [ $? -ne 0 ]; then
    echo "Build failed, aborting."
    exit 1
fi

# ── Write header ───────────────────────────────────────────────────────────────
{
    echo "# Bitchess vs Stockfish"
    echo "# Date:             $(date '+%Y-%m-%d %H:%M:%S')"
    [ -n "$DESCRIPTION" ] && echo "# Description:      $DESCRIPTION"
    echo "#"
    echo "# --- Configuration ---"
    echo "# GAMES:            $GAMES"
    echo "# TC:               $TC"
    echo "# STOCKFISH_ELO:    $STOCKFISH_ELO"
    echo "#"
} > "$OUTPUT"

echo "Saving results to: $OUTPUT"

# ── Run ────────────────────────────────────────────────────────────────────────
"$CUTECHESS" \
    -engine name=Bitchess proto=uci cmd="$BITCHESS" tc="$TC" \
    -engine name=Stockfish proto=uci cmd="$STOCKFISH" \
        "option.UCI_LimitStrength=true" "option.UCI_Elo=$STOCKFISH_ELO" \
        tc="$TC" restart=on \
    -games "$GAMES" \
    -pgnout "$OUTPUT" \
    -ratinginterval 10
