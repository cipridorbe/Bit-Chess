#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1
# Round-robin league between all binaries in league/binaries/.
# Run league_build.sh first.
#
# Usage:
#   ./league_run.sh              # default settings
#   ./league_run.sh -g 30 -d 8  # 30 games per pair, depth 8

CUTECHESS="/c/Program Files (x86)/Cute Chess/cutechess-cli.exe"
LEAGUE_DIR="./league/binaries"
RESULTS_DIR="./league/results"
GAMES=10   # games per pair (round-robin plays each pair twice: once per colour)

# Depth each version was originally tested at (from match.sh history)
declare -A VERSION_DEPTH
VERSION_DEPTH["v01_uci_1500elo"]=4
VERSION_DEPTH["v02_depth_and_captures"]=5
VERSION_DEPTH["v03_draws"]=5
VERSION_DEPTH["v04_quiescence"]=5
VERSION_DEPTH["v05_transposition_table"]=6
VERSION_DEPTH["v06_tt_memory"]=6
VERSION_DEPTH["v07_2000elo"]=6
VERSION_DEPTH["v08_score_tracking"]=7
VERSION_DEPTH["v09_pawn_king_eval"]=8
VERSION_DEPTH["v10_pvs_mate"]=8

while getopts "g:" opt; do
    case "$opt" in
        g) GAMES="$OPTARG" ;;
        *) echo "Usage: $0 [-g games_per_pair]"; exit 1 ;;
    esac
done

mkdir -p "$RESULTS_DIR"
OUTPUT="$RESULTS_DIR/league_$(date +%Y%m%d_%H%M%S).pgn"

# Collect all binaries
bins=("$LEAGUE_DIR"/*.exe)
if [ ${#bins[@]} -eq 0 ]; then
    echo "No binaries found in $LEAGUE_DIR — run league_build.sh first."
    exit 1
fi

echo "Engines in league:"
engine_args=()
for bin in "${bins[@]}"; do
    name=$(basename "$bin" .exe)
    depth=${VERSION_DEPTH[$name]:-8}
    echo "  $name  (depth $depth)"
    engine_args+=(-engine "name=$name" proto=uci "cmd=$bin" "depth=$depth" tc=inf)
done

n=${#bins[@]}
pairs=$(( n * (n - 1) / 2 ))
total=$(( pairs * GAMES ))
echo ""
echo "Format:  round-robin, $GAMES games per pair"
echo "Engines: $n   Pairs: $pairs   Total games: $total"
echo "Output:  $OUTPUT"
echo ""

"$CUTECHESS" \
    "${engine_args[@]}" \
    -tournament round-robin \
    -rounds "$GAMES" \
    -openings file="./league/openings.epd" format=epd order=random \
    -pgnout "$OUTPUT" \
    -ratinginterval 10 \
    -recover
