"""
Generate realistic chess positions by having Stockfish play against itself.
Samples positions at random plies across different game phases.
Outputs Rust array entries ready to paste into the bonus distribution test.

Usage: python scripts/gen_positions.py
"""

import subprocess, random, sys
try:
    import chess
    import chess.engine
except ImportError:
    subprocess.check_call([sys.executable, "-m", "pip", "install", "chess"])
    import chess
    import chess.engine

STOCKFISH = r"C:\Program Files\stockfish\stockfish-windows-x86-64-avx2.exe"
NUM_GAMES   = 30
DEPTH       = 12        # Stockfish search depth per move
TIME_LIMIT  = 0.05      # seconds per move (fast self-play)
SAMPLES_PER_GAME = 3    # positions sampled per game

# Sample at least one position from each ply window to spread across game phases
PLY_WINDOWS = [
    (10, 25),   # early middlegame
    (26, 50),   # middlegame
    (51, 90),   # late middlegame / endgame transition
    (91, 160),  # endgame
]

def material_phase(board):
    """Rough phase value: 24 = full middlegame, 0 = endgame (mirrors engine)."""
    weights = {
        chess.KNIGHT: 1, chess.BISHOP: 1,
        chess.ROOK: 2,   chess.QUEEN: 4,
    }
    phase = 0
    for pt, w in weights.items():
        phase += len(board.pieces(pt, chess.WHITE)) * w
        phase += len(board.pieces(pt, chess.BLACK)) * w
    return min(phase, 24)

def play_game(engine):
    board = chess.Board()
    positions = []   # (ply, fen, phase)
    ply = 0
    while not board.is_game_over(claim_draw=True):
        result = engine.play(board, chess.engine.Limit(depth=DEPTH, time=TIME_LIMIT))
        board.push(result.move)
        ply += 1
        phase = material_phase(board)
        positions.append((ply, board.fen(), phase))
    return positions

def sample_positions(positions, n):
    """Pick n positions spread across phase windows; fall back to random if sparse."""
    chosen = []
    for (lo, hi) in PLY_WINDOWS:
        candidates = [(ply, fen, ph) for (ply, fen, ph) in positions if lo <= ply <= hi]
        if candidates:
            chosen.append(random.choice(candidates))
    # fill remainder randomly, avoiding duplicates
    remaining = [p for p in positions if p not in chosen]
    random.shuffle(remaining)
    for p in remaining:
        if len(chosen) >= n:
            break
        if p not in chosen:
            chosen.append(p)
    return chosen[:n]

def main():
    random.seed(42)
    engine = chess.engine.SimpleEngine.popen_uci(STOCKFISH)
    engine.configure({"Skill Level": 20})

    all_samples = []  # (label, fen, phase)

    for game_idx in range(1, NUM_GAMES + 1):
        print(f"Game {game_idx}/{NUM_GAMES}...", file=sys.stderr)
        positions = play_game(engine)
        sampled = sample_positions(positions, SAMPLES_PER_GAME)
        for (ply, fen, phase) in sampled:
            label = f"G{game_idx}p{ply}ph{phase}"
            all_samples.append((label, fen, phase))

    engine.quit()

    # Deduplicate by FEN
    seen = set()
    unique = []
    for (label, fen, phase) in all_samples:
        if fen not in seen:
            seen.add(fen)
            unique.append((label, fen, phase))

    # Sort by phase descending (middlegame first, endgame last)
    unique.sort(key=lambda x: -x[2])

    print(f"\n// {len(unique)} positions (phase 24=middlegame, 0=endgame)")
    print("let positions = [")
    for (label, fen, phase) in unique:
        # strip move clocks for brevity, keep ep/castling
        print(f'    ("{label}", "{fen}"),  // phase~{phase}')
    print("];")

if __name__ == "__main__":
    main()
