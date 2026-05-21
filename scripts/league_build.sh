#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1
# Build one binary per milestone commit into league/binaries/.
# Re-run safely — already-built binaries are skipped.

LEAGUE_DIR="./league/binaries"
mkdir -p "$LEAGUE_DIR"

# Ordered oldest → newest. Format: "short_hash  friendly_name"
COMMITS=(
    "7310fff  v01_uci_1500elo"
    "b049d8b  v02_depth_and_captures"
    "4a71454  v03_draws"
    "2a0589c  v04_quiescence"
    "0db1fc5  v05_transposition_table"
    "dca2b97  v06_tt_memory"
    "c9b9c77  v07_2000elo"
    "785e3de  v08_score_tracking"
    "29007c1  v09_pawn_king_eval"
)

CURRENT_REF=$(git symbolic-ref --short HEAD 2>/dev/null || git rev-parse HEAD)
git stash -q
STASHED=$?

# Detach HEAD so git reset --hard can move freely between commits
git checkout --detach -q

failed=()

for entry in "${COMMITS[@]}"; do
    hash=$(echo "$entry" | awk '{print $1}')
    name=$(echo "$entry" | awk '{print $2}')
    out="$LEAGUE_DIR/${name}.exe"

    if [ -f "$out" ]; then
        echo "skip  $name (already exists)"
        continue
    fi

    echo "build $name  ($hash)..."

    # Hard reset to exactly this commit — clears any patches and updates all files
    if ! git reset --hard "$hash" -q; then
        echo "  -> CHECKOUT FAILED"
        failed+=("$name ($hash) - checkout failed")
        continue
    fi

    # v03 is a broken intermediate commit: captures_only param was added to
    # generate_movelist but callers weren't updated until v04.
    if [ "$hash" = "4a71454" ]; then
        sed -i 's/generate_movelist(&board)/generate_movelist(\&board, false)/g' src/search/negamax.rs src/frontend/mod.rs 2>/dev/null
        sed -i 's/generate_movelist(board)/generate_movelist(board, false)/g' src/search/negamax.rs src/frontend/mod.rs 2>/dev/null
    fi

    # CARGO_INCREMENTAL=0 forces a fresh build — prevents cargo from reusing
    # cached artifacts from a different commit with the same file timestamps.
    if RUSTFLAGS="-A warnings" CARGO_INCREMENTAL=0 cargo build --release --bin uci 2>&1; then
        cp target/release/uci.exe "$out"
        echo "  -> $out"
    else
        echo "  -> FAILED"
        failed+=("$name ($hash)")
    fi
done

# Restore working state
git checkout "$CURRENT_REF" -q
[ $STASHED -eq 0 ] && git stash pop -q 2>/dev/null

echo ""
if [ ${#failed[@]} -gt 0 ]; then
    echo "Failed builds:"
    for f in "${failed[@]}"; do echo "  $f"; done
    echo ""
fi

echo "Binaries in $LEAGUE_DIR/:"
ls "$LEAGUE_DIR/"
