#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1
# Benchmark all league binaries at each engine's configured league depth.
# For each position, plays 5 consecutive moves and reports the average search time.
# Depths are read from league_run.sh.
#
# Usage:
#   ./scripts/league_bench.sh

LEAGUE_DIR="./league/binaries"
REPS=5

# Parse VERSION_DEPTH from league_run.sh without executing it
declare -A VERSION_DEPTH
while IFS= read -r line; do
    if [[ "$line" =~ VERSION_DEPTH\[\"([^\"]+)\"\]=([0-9]+) ]]; then
        VERSION_DEPTH["${BASH_REMATCH[1]}"]="${BASH_REMATCH[2]}"
    fi
done < "./scripts/league_run.sh"

POSITIONS=(
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"
    "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"
    "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4"
)
POS_LABELS=("startpos" "kiwipete" "rook-eg" "italian")

# bench_position <bin> <depth> <fen>
# Plays REPS consecutive moves from <fen>, returns average ms via stdout
bench_position() {
    local bin="$1" depth="$2" fen="$3"
    local moves="" total=0 count=0

    coproc ENGINE { "$bin" 2>/dev/null; }

    echo "uci" >&"${ENGINE[1]}"
    while IFS= read -ru "${ENGINE[0]}" line; do
        [[ "$line" == *"uciok"* ]] && break
    done

    echo "isready" >&"${ENGINE[1]}"
    while IFS= read -ru "${ENGINE[0]}" line; do
        [[ "$line" == *"readyok"* ]] && break
    done

    for (( i=0; i<REPS; i++ )); do
        local pos_cmd="position fen $fen"
        [[ -n "$moves" ]] && pos_cmd="$pos_cmd moves $moves"
        echo "$pos_cmd" >&"${ENGINE[1]}"

        local start
        start=$(date +%s%3N)
        echo "go depth $depth" >&"${ENGINE[1]}"

        local mv=""
        while IFS= read -ru "${ENGINE[0]}" line; do
            if [[ "$line" == bestmove* ]]; then
                local ms=$(( $(date +%s%3N) - start ))
                total=$(( total + ms ))
                count=$(( count + 1 ))
                mv=$(echo "$line" | awk '{print $2}')
                break
            fi
        done

        # Stop if game over (no move)
        [[ -z "$mv" || "$mv" == "0000" ]] && break
        moves="${moves:+$moves }$mv"
    done

    echo "quit" >&"${ENGINE[1]}" 2>/dev/null
    wait "$ENGINE_PID" 2>/dev/null

    [[ $count -gt 0 ]] && echo $(( total / count )) || echo "?"
}

bins=("$LEAGUE_DIR"/*.exe)
if [ ${#bins[@]} -eq 0 ]; then
    echo "No binaries found in $LEAGUE_DIR"
    exit 1
fi

echo "Reps per position: $REPS (avg search time shown)"
echo ""
printf "%-28s  %5s" "Engine" "depth"
for label in "${POS_LABELS[@]}"; do
    printf "%12s" "$label"
done
printf "%12s\n" "avg total"
printf '%s\n' "$(printf '%.0s-' {1..90})"

for bin in "${bins[@]}"; do
    name=$(basename "$bin" .exe)
    depth=${VERSION_DEPTH[$name]:-8}
    printf "%-28s  %5s" "$name" "$depth"
    grand_total=0
    pos_count=0

    for fen in "${POSITIONS[@]}"; do
        avg=$(bench_position "$bin" "$depth" "$fen")
        printf "%11sms" "$avg"
        if [[ "$avg" =~ ^[0-9]+$ ]]; then
            grand_total=$(( grand_total + avg ))
            pos_count=$(( pos_count + 1 ))
        fi
    done

    if [[ $pos_count -gt 0 ]]; then
        printf "%11dms\n" $(( grand_total / pos_count ))
    else
        printf "%12s\n" "?"
    fi
done
