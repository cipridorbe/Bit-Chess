#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1
# Run a match between the current HEAD and a saved baseline binary.
#
# Usage:
#   ./self_match.sh                       # HEAD vs baselines/default.exe
#   ./self_match.sh -b baselines/foo.exe  # HEAD vs a specific baseline
#   ./self_match.sh -n my_test            # custom name for output file
#   ./self_match.sh -d "nmp vs no nmp"    # description in header
#   ./self_match.sh -m 8                  # fixed depth (no time control)

# ── Paths ─────────────────────────────────────────────────────────────────────
CUTECHESS="/c/Program Files (x86)/Cute Chess/cutechess-cli.exe"

# ── Defaults ──────────────────────────────────────────────────────────────────
BASELINE="baselines/default.exe"
NAME=""
DESCRIPTION=""
GAMES=50
TC="60+1"
MAXDEPTH=""

while getopts "b:n:d:g:t:m:" opt; do
    case "$opt" in
        b) BASELINE="$OPTARG" ;;
        n) NAME="$OPTARG" ;;
        d) DESCRIPTION="$OPTARG" ;;
        g) GAMES="$OPTARG" ;;
        t) TC="$OPTARG" ;;
        m) MAXDEPTH="$OPTARG" ;;
        *) echo "Usage: $0 [-b baseline.exe] [-n name] [-d description] [-g games] [-t tc] [-m maxdepth]"; exit 1 ;;
    esac
done

if [ ! -f "$BASELINE" ]; then
    echo "Baseline not found: $BASELINE"
    echo "Build one with: scripts/save_baseline.sh [-c <commit>] [-n <name>]"
    exit 1
fi

mkdir -p results

if [ -z "$NAME" ]; then
    N=1
    while [ -f "results/self${N}.pgn" ]; do N=$((N + 1)); done
    OUTPUT="results/self${N}.pgn"
else
    OUTPUT="results/${NAME}.pgn"
fi

# ── Build current HEAD ─────────────────────────────────────────────────────────
echo "Building current HEAD..."
cargo build --release --bin uci 2>&1
if [ $? -ne 0 ]; then echo "Build failed, aborting."; exit 1; fi

NEW_HASH=$(git rev-parse --short HEAD)
BASELINE_NAME=$(basename "$BASELINE" .exe)

# ── Write header ───────────────────────────────────────────────────────────────
{
    echo "# Bitchess self-match"
    echo "# Date:        $(date '+%Y-%m-%d %H:%M:%S')"
    echo "# New:         $NEW_HASH (HEAD)"
    echo "# Baseline:    $BASELINE_NAME"
    [ -n "$DESCRIPTION" ] && echo "# Description: $DESCRIPTION"
    if [ -n "$MAXDEPTH" ]; then
        echo "# Games:       $GAMES  Depth: $MAXDEPTH"
    else
        echo "# Games:       $GAMES  TC: $TC"
    fi
    echo "#"
} > "$OUTPUT"

echo "Saving results to: $OUTPUT"
echo "New: $NEW_HASH  vs  Baseline: $BASELINE_NAME"

# ── Build engine args ──────────────────────────────────────────────────────────
if [ -n "$MAXDEPTH" ]; then
    ENGINE_ARGS="tc=inf depth=$MAXDEPTH"
else
    ENGINE_ARGS="tc=$TC"
fi

# ── Run ────────────────────────────────────────────────────────────────────────
"$CUTECHESS" \
    -engine name="New_${NEW_HASH}"    proto=uci cmd="./target/release/uci.exe" $ENGINE_ARGS \
    -engine name="$BASELINE_NAME"     proto=uci cmd="./$BASELINE"              $ENGINE_ARGS \
    -games "$GAMES" \
    -openings file="./league/openings.epd" format=epd order=random -repeat \
    -pgnout "$OUTPUT" \
    -ratinginterval 10
