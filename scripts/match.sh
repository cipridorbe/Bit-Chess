#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1
# ── Paths ─────────────────────────────────────────────────────────────────────
CUTECHESS="/c/Program Files (x86)/Cute Chess/cutechess-cli.exe"
BITCHESS="./target/release/uci.exe"
STOCKFISH="/c/Program Files/stockfish/stockfish-windows-x86-64-avx2.exe"

# ── Match settings ─────────────────────────────────────────────────────────────
GAMES=50

# ── Engine settings ────────────────────────────────────────────────────────────
BITCHESS_DEPTH=11
STOCKFISH_DEPTH=15
STOCKFISH_ELO=2800   # min 1320, max 3190

# ── Args: [-n name] [-d description] ─────────────────────────────────────────
NAME=""
DESCRIPTION=""
while getopts "n:d:" opt; do
    case "$opt" in
        n) NAME="$OPTARG" ;;
        d) DESCRIPTION="$OPTARG" ;;
        *) echo "Usage: $0 [-n name] [-d description]"; exit 1 ;;
    esac
done

mkdir -p results

if [ -z "$NAME" ]; then
    # Auto-number: find next results/resultsN.pgn
    N=1
    while [ -f "results/results${N}.pgn" ]; do
        N=$((N + 1))
    done
    OUTPUT="results/results${N}.pgn"
else
    OUTPUT="results/${NAME}.pgn"
fi

# ── Build ──────────────────────────────────────────────────────────────────────
cargo build --release --bin uci 2>&1
if [ $? -ne 0 ]; then
    echo "Build failed, aborting."
    exit 1
fi

# ── Write header to results file ───────────────────────────────────────────────
{
    echo "# Bitchess match results"
    echo "# Date:             $(date '+%Y-%m-%d %H:%M:%S')"
    [ -n "$DESCRIPTION" ] && echo "# Description:      $DESCRIPTION"
    echo "#"
    echo "# --- Configuration ---"
    echo "# GAMES:            $GAMES"
    echo "# BITCHESS_DEPTH:   $BITCHESS_DEPTH"
    echo "# STOCKFISH_DEPTH:  $STOCKFISH_DEPTH"
    echo "# STOCKFISH_ELO:    $STOCKFISH_ELO"
    echo "#"
} > "$OUTPUT"

echo "Saving results to: $OUTPUT"

# ── Run ────────────────────────────────────────────────────────────────────────
"$CUTECHESS" \
    -engine name=Bitchess proto=uci cmd="$BITCHESS" depth="$BITCHESS_DEPTH" tc=inf \
    -engine name=Stockfish proto=uci cmd="$STOCKFISH" \
        "option.UCI_LimitStrength=true" "option.UCI_Elo=$STOCKFISH_ELO" \
        depth="$STOCKFISH_DEPTH" tc=inf restart=on \
    -games "$GAMES" \
    -pgnout "$OUTPUT" \
    -ratinginterval 10
