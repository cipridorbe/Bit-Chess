#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1

# ── Paths ─────────────────────────────────────────────────────────────────────
CUTECHESS="/c/Program Files (x86)/Cute Chess/cutechess-cli.exe"
BITCHESS="./target/release/uci.exe"
STOCKFISH="/c/Program Files/stockfish/stockfish-windows-x86-64-avx2.exe"

# ── Match settings ─────────────────────────────────────────────────────────────
GAMES=50

# ── Engine settings ────────────────────────────────────────────────────────────
BITCHESS_DEPTH=10
STOCKFISH_DEPTH=15
STOCKFISH_ELO=2600

# ── Log directory ──────────────────────────────────────────────────────────────
LOGDIR="logs/debug_$(date '+%Y%m%d_%H%M%S')"
mkdir -p "$LOGDIR"

echo "Debug logs: $LOGDIR/"

# ── Build ──────────────────────────────────────────────────────────────────────
cargo build --release --bin uci 2>&1
if [ $? -ne 0 ]; then
    echo "Build failed, aborting."
    exit 1
fi

# ── Resolve Windows-compatible paths (CuteChess is a Windows EXE) ──────────────
win_path() {
    if command -v cygpath &>/dev/null; then
        cygpath -w "$1"
    else
        echo "$1" | sed 's|^/\([a-zA-Z]\)/|\1:/|; s|/|\\|g'
    fi
}

BASH_WIN="$(win_path "$(which bash)")"
WRAPPER_WIN="$(win_path "$(realpath scripts/engine_log_wrapper.sh)")"
BITCHESS_WIN="$(win_path "$(realpath "$BITCHESS")")"
LOGPREFIX_WIN="$(win_path "$(realpath "$LOGDIR")")\bitchess"

BITCHESS_LOG="$LOGDIR/bitchess"

# ── Run ────────────────────────────────────────────────────────────────────────
"$CUTECHESS" \
    -engine name=Bitchess proto=uci \
        cmd="$BASH_WIN" \
        arg="$WRAPPER_WIN" arg="$LOGPREFIX_WIN" arg="$BITCHESS_WIN" \
        depth="$BITCHESS_DEPTH" tc=inf \
    -engine name=Stockfish proto=uci cmd="$STOCKFISH" \
        "option.UCI_LimitStrength=true" "option.UCI_Elo=$STOCKFISH_ELO" \
        depth="$STOCKFISH_DEPTH" tc=inf restart=on \
    -games "$GAMES" \
    -pgnout "$LOGDIR/game.pgn" \
    -ratinginterval 1

EXIT_CODE=$?

# ── Summary ────────────────────────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════"
echo "  Debug output in: $LOGDIR/"
echo ""

if [ -f "$LOGDIR/game.pgn" ]; then
    echo "  game.pgn      — PGN with full move list"
fi
if [ -f "${BITCHESS_LOG}.in.log" ]; then
    echo "  bitchess.in.log  — commands sent TO bitchess"
fi
if [ -f "${BITCHESS_LOG}.out.log" ]; then
    echo "  bitchess.out.log — responses FROM bitchess"
fi
if [ -f "${BITCHESS_LOG}.err.log" ]; then
    ERRSIZE=$(wc -c < "${BITCHESS_LOG}.err.log" 2>/dev/null || echo 0)
    echo "  bitchess.err.log — engine stderr ($ERRSIZE bytes)"
fi

echo "═══════════════════════════════════════════"
echo ""

# Print the full UCI dialogue inline for quick inspection
if [ -f "${BITCHESS_LOG}.in.log" ] && [ -f "${BITCHESS_LOG}.out.log" ]; then
    echo "── UCI dialogue (interleaved) ──────────────"
    # Merge in/out with labels; sort by mtime isn't reliable so show separately
    echo "=== Commands sent to Bitchess ==="
    cat "${BITCHESS_LOG}.in.log"
    echo ""
    echo "=== Responses from Bitchess ==="
    cat "${BITCHESS_LOG}.out.log"
fi

if [ -s "${BITCHESS_LOG}.err.log" ]; then
    echo ""
    echo "=== Bitchess stderr ==="
    cat "${BITCHESS_LOG}.err.log"
fi

exit $EXIT_CODE
