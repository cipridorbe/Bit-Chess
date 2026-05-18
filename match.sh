#!/usr/bin/env bash
# ── Paths ─────────────────────────────────────────────────────────────────────
CUTECHESS="/c/Program Files (x86)/Cute Chess/cutechess-cli.exe"
BITCHESS="./target/release/uci.exe"
STOCKFISH="/c/Program Files/stockfish/stockfish-windows-x86-64-avx2.exe"

# ── Match settings ─────────────────────────────────────────────────────────────
GAMES=50
OUTPUT="results.pgn"

# ── Engine settings ────────────────────────────────────────────────────────────
BITCHESS_DEPTH=6
STOCKFISH_DEPTH=15
STOCKFISH_ELO=1500   # min 1320, max 3190

# ── Build ──────────────────────────────────────────────────────────────────────
cargo build --release --bin uci 2>&1
if [ $? -ne 0 ]; then
    echo "Build failed, aborting."
    exit 1
fi

# ── Run ────────────────────────────────────────────────────────────────────────
"$CUTECHESS" \
    -engine name=Bitchess proto=uci cmd="$BITCHESS" depth="$BITCHESS_DEPTH" tc=inf \
    -engine name=Stockfish proto=uci cmd="$STOCKFISH" \
        "option.UCI_LimitStrength=true" "option.UCI_Elo=$STOCKFISH_ELO" \
        depth="$STOCKFISH_DEPTH" tc=inf restart=on \
    -games "$GAMES" \
    -pgnout "$OUTPUT" \
    -ratinginterval 10
