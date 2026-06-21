#!/usr/bin/env python3
"""Phase split: check / build / run / JIT+runtime for key benchmarks."""
from __future__ import annotations

import os
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target/release/action"
EXAMPLES = ROOT / "examples"
CWD = str(ROOT)


def timed(args: list[str], runs: int = 3) -> float:
    times: list[float] = []
    for _ in range(runs):
        t0 = time.perf_counter()
        subprocess.run(args, capture_output=True, cwd=CWD)
        times.append(time.perf_counter() - t0)
    return statistics.mean(times)


def row(name: str) -> None:
    f = EXAMPLES / name
    out = Path(f"/tmp/{name.replace('.ac', '')}.ll")
    if out.exists():
        out.unlink()
    tc = timed([str(BIN), "check", str(f)])
    tb = timed([str(BIN), "build", "-o", str(out), str(f)])
    tr = timed([str(BIN), "run", "-O0", str(f)])
    print(f"{name:24s} {tc*1000:7.0f} {tb*1000:7.0f} {tr*1000:7.0f} {(tr-tb)*1000:7.0f}")


def main() -> None:
    if not BIN.is_file():
        raise SystemExit(f"missing binary: {BIN} (run cargo build --release)")

    print(f"{'benchmark':24s} {'check':>7s} {'build':>7s} {'run':>7s} {'jit+rt':>7s}")
    for i in range(1, 7):
        row(f"bench_step{i}.ac")
    print()
    for name in (
        "bench_all.ac",
        "_dev/bench_insert_bisect.ac",
        "bench_for_method.ac",
        "bench_funcall.ac",
        "bench_cow.ac",
    ):
        row(name)


if __name__ == "__main__":
    main()
