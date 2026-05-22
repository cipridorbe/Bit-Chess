#!/usr/bin/env python3
"""
Download lichess openings TSVs and convert to EPD for league play.
Keeps only lines with >= 3 full moves, deduplicates, and writes N random entries.
"""
import urllib.request
import chess
import chess.pgn
import io
import random
import sys

TARGET = int(sys.argv[1]) if len(sys.argv) > 1 else 50
MIN_FULL_MOVES = 3
OUT = "league/openings.epd"

epds = set()

for letter in "abcde":
    url = f"https://raw.githubusercontent.com/lichess-org/chess-openings/master/{letter}.tsv"
    print(f"Fetching {url} ...", end=" ", flush=True)
    with urllib.request.urlopen(url) as r:
        lines = r.read().decode().splitlines()
    print(f"{len(lines)-1} openings")

    for line in lines[1:]:  # skip header
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        pgn_text = parts[2].strip()

        # Count full moves: last move number in PGN
        game = chess.pgn.read_game(io.StringIO(pgn_text))
        if game is None:
            continue
        board = game.end().board()
        if board.fullmove_number - 1 < MIN_FULL_MOVES:
            continue  # fewer than 3 full moves played

        epds.add(board.epd())

epds = list(epds)
random.shuffle(epds)
chosen = epds[:TARGET]

with open(OUT, "w") as f:
    for epd in chosen:
        f.write(epd + "\n")

print(f"\nWrote {len(chosen)} openings to {OUT}")
