#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1
# Run a match between the current HEAD and a previous commit (or any two binaries).
#
# Usage:
#   ./self_match.sh                     # HEAD vs HEAD~1
#   ./self_match.sh -c abc1234          # HEAD vs specific commit
#   ./self_match.sh -n my_test          # custom name for output file
#   ./self_match.sh -d "nmp vs no nmp"  # description in header

# ── Paths ─────────────────────────────────────────────────────────────────────
CUTECHESS="/c/Program Files (x86)/Cute Chess/cutechess-cli.exe"

# ── Defaults ──────────────────────────────────────────────────────────────────
BASELINE_COMMIT="HEAD~1"
NAME=""
DESCRIPTION=""
GAMES=50
TC="60+1"

while getopts "c:n:d:g:t:" opt; do
    case "$opt" in
        c) BASELINE_COMMIT="$OPTARG" ;;
        n) NAME="$OPTARG" ;;
        d) DESCRIPTION="$OPTARG" ;;
        g) GAMES="$OPTARG" ;;
        t) TC="$OPTARG" ;;
        *) echo "Usage: $0 [-c commit] [-n name] [-d description] [-g games] [-t tc]"; exit 1 ;;
    esac
done

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
cp target/release/uci.exe target/release/uci_new.exe
NEW_HASH=$(git rev-parse --short HEAD)

# ── Build baseline commit ──────────────────────────────────────────────────────
echo "Building $BASELINE_COMMIT..."
BASELINE_HASH=$(git rev-parse --short "$BASELINE_COMMIT")
git stash --include-untracked -q

git checkout "$BASELINE_COMMIT" -q -- .
cargo build --release --bin uci 2>&1
if [ $? -ne 0 ]; then
    git checkout HEAD -- . -q
    git stash pop -q 2>/dev/null
    echo "Baseline build failed, aborting."
    exit 1
fi
cp target/release/uci.exe target/release/uci_old.exe

# ── Restore current state ──────────────────────────────────────────────────────
git checkout HEAD -- . -q
git stash pop -q 2>/dev/null

# ── Write header ───────────────────────────────────────────────────────────────
{
    echo "# Bitchess self-match"
    echo "# Date:        $(date '+%Y-%m-%d %H:%M:%S')"
    echo "# New:         $NEW_HASH (HEAD)"
    echo "# Baseline:    $BASELINE_HASH ($BASELINE_COMMIT)"
    [ -n "$DESCRIPTION" ] && echo "# Description: $DESCRIPTION"
    echo "# Games:       $GAMES  TC: $TC"
    echo "#"
} > "$OUTPUT"

echo "Saving results to: $OUTPUT"
echo "New: $NEW_HASH  vs  Baseline: $BASELINE_HASH"

# ── Run ────────────────────────────────────────────────────────────────────────
"$CUTECHESS" \
    -engine name="New_${NEW_HASH}"      proto=uci cmd="./target/release/uci_new.exe" tc="$TC" \
    -engine name="Old_${BASELINE_HASH}" proto=uci cmd="./target/release/uci_old.exe" tc="$TC" \
    -games "$GAMES" \
    -pgnout "$OUTPUT" \
    -ratinginterval 10
