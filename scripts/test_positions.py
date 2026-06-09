#!/usr/bin/env python3
"""
Tests the engine against standard test suites (BK, WAC, Kaufman).
Usage: python scripts/test_positions.py [depth]
Requires: pip install chess
"""

import subprocess, sys, time, chess

DEPTH = int(sys.argv[1]) if len(sys.argv) > 1 else 8
ENGINE = r".\target\release\uci.exe"
STOCKFISH = r"C:\Program Files\stockfish\stockfish-windows-x86-64-avx2.exe"

# (fen, [best_moves_san], description, suite)
POSITIONS = [
    # --- Bratko-Kopec Test (1982) ---
    ("1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 w - - 0 1",
     ["Rd7"], "BK.01"),
    ("3r1k2/4npp1/1ppr3p/p6P/P2PPPP1/1NR5/5K2/2R5 w - - 0 1",
     ["d5"], "BK.02"),
    ("2q1rr1k/3bbnnp/p2p1pp1/2pPp3/PpP1P1P1/1P2BNNP/2BQ1PRK/7R b - - 0 1",
     ["f5"], "BK.03"),
    ("rnbqkb1r/p3pppp/1p6/2ppP3/3N4/2P5/PPP1QPPP/R1B1KB1R w KQkq - 0 1",
     ["e6"], "BK.04"),
    ("r1b2rk1/2q1b1pp/p2ppn2/1p6/3QP3/1BN1B3/PPP3PP/R4RK1 w - - 0 1",
     ["Nd5", "a4"], "BK.05"),
    ("2r3k1/pppR1pp1/4p3/4P1P1/5P2/1P4K1/P1P5/8 w - - 0 1",
     ["Kf3"], "BK.06"),
    ("4b3/p3kp2/6p1/3pP2p/2pP1P2/4K1P1/P3N2P/8 w - - 0 1",
     ["f5"], "BK.08"),
    ("3rr1k1/pp3pp1/1qn2np1/8/3p4/PP1RPPPP/2Q1BK2/1NB3R1 b - - 0 1",
     ["Ne5"], "BK.10"),
    ("2r1nrk1/p2q1ppp/bp1p4/n1pPp3/P1P1P3/2PBB1N1/4QPPP/R4RK1 w - - 0 1",
     ["f4"], "BK.11"),
    ("r3r1k1/ppqb1ppp/8/4p1NQ/8/2P5/PP3PPP/R3R1K1 b - - 0 1",
     ["Bf5"], "BK.12"),
    ("r1b2rk1/pp2ppbp/2np1np1/q5B1/3PP3/2N2N2/PP2BPPP/R2QK2R w KQ - 0 1",
     ["d5"], "BK.15"),
    ("3r2k1/1p3ppp/2pq4/p1n5/P6P/1P6/3Q1PP1/1R3RK1 b - - 0 1",
     ["Qxd2", "Nxb3"], "BK.16"),
    ("1r1r1qk1/p2n1p1p/bp1Pn1p1/2pNp3/P1B1P3/2P2QBP/5PP1/1R2R1K1 w - - 0 1",
     ["Nf6+"], "BK.20"),
    ("r2q1rk1/ppp1b1pp/2n1p3/3pP1n1/3P2b1/2PB1NN1/PP4PP/R1BQR1K1 w - - 0 1",
     ["Nxg5"], "BK.21"),
    ("8/p1ppkp2/b3p3/4P1pp/4K3/8/PPP2PPP/8 w - - 0 1",
     ["f4"], "BK.23"),
    ("1r4k1/7p/5np1/3p3n/8/2NB2P1/7P/R5K1 b - - 0 1",
     ["Nf4"], "BK.24"),

    # --- Win At Chess (WAC) - selection ---
    ("2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - 0 1",
     ["Qg6"], "WAC.001"),
    ("8/7p/5k2/5p2/p1p2P2/Pr1pPK2/1P1R3P/8 b - - 0 1",
     ["Rxb2"], "WAC.002"),
    ("r1b1kb1r/pppp1ppp/2n5/4P3/2Bn4/5N2/PPP2PPP/RNBQK2R w KQkq - 0 1",
     ["Bxf7+"], "WAC.003"),
    ("r2q1rk1/pb3p1p/2n3p1/2B5/8/2N2Q2/PP3PPP/3R1RK1 w - - 0 1",
     ["Rxd8", "Bxf7+"], "WAC.004"),
    ("3r1rk1/p1p2p1p/2pb2p1/8/q3P3/2NR4/PPQ2PPP/5RK1 b - - 0 1",
     ["Bxh2+"], "WAC.005"),
    ("r1bqkb1r/pp3ppp/2nppn2/6B1/3NP3/2N5/PPP2PPP/R2QKB1R w KQkq - 0 1",
     ["Nxc6", "Bxf6"], "WAC.006"),
    ("1r2k1r1/pbppnp1p/1b3P2/8/Q7/B1PB2N1/P4PPP/3R1RK1 w - - 0 1",
     ["Qxd7+"], "WAC.007"),
    ("r4rk1/p1pnqppp/b1p5/3p4/3P4/B1N2N2/PP3PPP/R2QR1K1 w - - 0 1",
     ["Nxd5"], "WAC.008"),
    ("r3r1k1/1pq2rpp/p1pp1p2/4nP2/2P1p1N1/P3Q2P/1PB3P1/3RR2K w - - 0 1",
     ["Nxf6+"], "WAC.009"),
    ("rn2kb1r/pp2pppp/1qP5/8/6b1/1Q6/PP1NPPPP/R1B1KB1R b KQkq - 0 1",
     ["Qxb3"], "WAC.010"),

    # --- Kaufman Test (selection) ---
    ("r2qkb1r/1b1n1ppp/p3pn2/1p6/3NP3/1BN5/PPP2PPP/R1BQ1RK1 w kq - 0 1",
     ["Nc6"], "KF.02"),
    ("r1bq1rk1/pppn1ppp/3bpn2/3p4/2PP4/5NP1/PP1NPPBP/R1BQ1RK1 b - - 0 1",
     ["dxc4"], "KF.03"),
    ("r4rk1/pp1nppbp/2p3p1/q7/3P4/2N1BN2/PP2QPPP/2R2RK1 b - - 0 1",
     ["Qxa2"], "KF.04"),
    ("r1b1kb1r/1pqn1ppp/p3pn2/8/3NP3/2N1B3/PPP1BPPP/R2QK2R w KQkq - 0 1",
     ["Nd5"], "KF.05"),
]


def san_to_uci(fen, san_list):
    board = chess.Board(fen)
    result = []
    for san in san_list:
        try:
            move = board.parse_san(san)
            result.append(move.uci())
        except Exception:
            pass
    return result


def query_engine(proc, fen, depth):
    proc.stdin.write("ucinewgame\n")
    proc.stdin.write("isready\n")
    proc.stdin.flush()
    while proc.stdout.readline().strip() != "readyok":
        pass
    proc.stdin.write(f"position fen {fen}\n")
    proc.stdin.write(f"go depth {depth}\n")
    proc.stdin.flush()
    while True:
        line = proc.stdout.readline().strip()
        if line.startswith("bestmove"):
            parts = line.split()
            return parts[1] if len(parts) > 1 else None


def start_engine(path):
    proc = subprocess.Popen(
        [path],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, text=True, bufsize=1,
    )
    proc.stdin.write("uci\n")
    proc.stdin.flush()
    while proc.stdout.readline().strip() != "uciok":
        pass
    proc.stdin.write("isready\n")
    proc.stdin.flush()
    while proc.stdout.readline().strip() != "readyok":
        pass
    return proc


def stop_engine(proc):
    proc.stdin.write("quit\n")
    proc.stdin.flush()
    proc.wait()


def main():
    import os
    proc = start_engine(ENGINE)
    sf = start_engine(STOCKFISH) if os.path.exists(STOCKFISH) else None
    if sf:
        print(f"Stockfish: {STOCKFISH}")
    else:
        print(f"Stockfish not found at {STOCKFISH} — skipping")

    results_by_suite = {}
    correct = total = 0
    start = time.time()

    for fen, expected_san, desc in POSITIONS:
        suite = desc.split(".")[0]
        expected_uci = san_to_uci(fen, expected_san)
        if not expected_uci:
            print(f"? {desc}: could not parse expected moves {expected_san}")
            continue

        best = query_engine(proc, fen, DEPTH)
        sf_best = query_engine(sf, fen, DEPTH) if sf else None

        found = best in expected_uci
        correct += found
        total += 1
        results_by_suite.setdefault(suite, [0, 0])
        results_by_suite[suite][0] += found
        results_by_suite[suite][1] += 1

        status = "OK" if found else "--"
        exp_str = " / ".join(expected_san)
        sf_str = f"  sf={sf_best}" if sf_best else ""
        print(f"{status} {desc:10s}  engine={best}  expected={exp_str}{sf_str}")

    stop_engine(proc)
    if sf:
        stop_engine(sf)

    elapsed = time.time() - start
    print(f"\n" + "-" * 55)
    for suite, (c, t) in sorted(results_by_suite.items()):
        print(f"  {suite}: {c}/{t}")
    print(f"  Total: {correct}/{total} ({100*correct//total}%) in {elapsed:.1f}s")


if __name__ == "__main__":
    main()
