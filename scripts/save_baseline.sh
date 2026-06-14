#!/usr/bin/env bash
cd "$(dirname "$0")/.." || exit 1
# Build a specific commit and save its binary as a named baseline.
#
# Usage:
#   ./save_baseline.sh                  # saves HEAD as baselines/default.exe
#   ./save_baseline.sh -c 6152e71       # saves that commit
#   ./save_baseline.sh -c HEAD -n v2250 # saves with a custom name

COMMIT="HEAD"
NAME=""

while getopts "c:n:" opt; do
    case "$opt" in
        c) COMMIT="$OPTARG" ;;
        n) NAME="$OPTARG" ;;
        *) echo "Usage: $0 [-c commit] [-n name]"; exit 1 ;;
    esac
done

HASH=$(git rev-parse --short "$COMMIT")
DEST_NAME="${NAME:-default}"
mkdir -p baselines

TMPWORK=".tmp_baseline_${HASH}"
git worktree add --detach -q "$TMPWORK" "$COMMIT"

pushd "$TMPWORK" > /dev/null
cargo build --release --bin uci 2>&1
BUILD_OK=$?
popd > /dev/null

if [ $BUILD_OK -eq 0 ]; then
    cp "$TMPWORK/target/release/uci.exe" "baselines/${DEST_NAME}.exe"
    echo "Saved: baselines/${DEST_NAME}.exe  (commit $HASH)"
else
    echo "Build failed for $COMMIT"
fi

git worktree remove --force "$TMPWORK"
exit $BUILD_OK
