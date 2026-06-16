#!/usr/bin/env python3
"""Full performance report: benchmark.sh, phase split, AOT runtime, step deltas."""
from __future__ import annotations

import os
import re
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target/release/action"
EXAMPLES = ROOT / "examples"
CWD = str(ROOT)
RESULTS = ROOT / "benchmark_results.txt"


def mean_run(cmd: list[str], runs: int = 3, warmup: int = 1) -> tuple[float, float, float]:
    for _ in range(warmup):
        subprocess.run(cmd, capture_output=True, cwd=CWD)
    times: list[float] = []
    for _ in range(runs):
        t0 = time.perf_counter()
        subprocess.run(cmd, capture_output=True, cwd=CWD)
        times.append(time.perf_counter() - t0)
    return statistics.mean(times), min(times), max(times)


def bench_run(file: Path, opt: int = 0, runs: int = 3) -> tuple[float, float, float]:
    return mean_run([str(BIN), "run", f"-O{opt}", str(file)], runs=runs)


def bench_build(file: Path, runs: int = 3) -> tuple[float, float, float]:
    out = Path(f"/tmp/_b_{file.name}.ll")
    return mean_run([str(BIN), "build", "-o", str(out), str(file)], runs=runs, warmup=0)


def bench_aot(file: Path, opt: int = 2, runs: int = 3) -> tuple[float, float, float] | None:
    exe = file.with_suffix("")
    subprocess.run(
        [str(BIN), "run", f"-O{opt}", "--emit", "exe", str(file)],
        capture_output=True,
        cwd=CWD,
    )
    if not exe.is_file():
        return None
    result = mean_run([str(exe)], runs=runs)
    exe.unlink(missing_ok=True)
    return result


def run_benchmark_sh() -> None:
    print("=== BENCHMARK.SH (run mode, warmup=1, n=3) ===")
    r = subprocess.run(
        ["bash", "benchmark.sh", "-n", "3"],
        capture_output=True,
        cwd=CWD,
        text=True,
    )
    print(r.stdout)
    if r.returncode != 0:
        print("STDERR:", r.stderr[-2000:])


def parse_results() -> dict[str, tuple[int, int, int]]:
    results: dict[str, tuple[int, int, int]] = {}
    if not RESULTS.is_file():
        return results
    for line in RESULTS.read_text(encoding="utf-8", errors="replace").splitlines():
        m = re.match(r"^(bench_\S+)\s+(\d+)\s+(\d+)\s+(\d+)\s+PASS", line)
        if m:
            results[m.group(1)] = (int(m.group(2)), int(m.group(3)), int(m.group(4)))
    return results


def phase_split() -> None:
    print("\n=== PHASE SPLIT (ms, 3-run avg) ===")
    print(f"{'benchmark':24s} {'check':>7s} {'build':>7s} {'run':>7s} {'jit+rt':>7s}")
    names = [
        "bench_step1.at",
        "bench_step3.at",
        "bench_step6.at",
        "bench_all.at",
        "bench_insert_bisect.at",
        "bench_for_method.at",
        "bench_cow.at",
    ]
    for name in names:
        f = EXAMPLES / name
        tc = statistics.mean(
            [
                (
                    lambda: (
                        t := time.perf_counter(),
                        subprocess.run([str(BIN), "check", str(f)], capture_output=True, cwd=CWD),
                        time.perf_counter() - t,
                    )[2]
                )()
                for _ in range(3)
            ]
        )
        tb, _, _ = bench_build(f, runs=3)
        tr, _, _ = bench_run(f, opt=0, runs=3)
        print(f"{name:24s} {tc*1000:7.0f} {tb*1000:7.0f} {tr*1000:7.0f} {(tr-tb)*1000:7.0f}")


def aot_runtime() -> None:
    print("\n=== AOT PURE RUNTIME (-O2, ms, 3-run avg) ===")
    for name in (
        "bench_step1.at",
        "bench_step3.at",
        "bench_step6.at",
        "bench_all.at",
        "bench_insert_bisect.at",
        "bench_cow.at",
    ):
        f = EXAMPLES / name
        r = bench_aot(f, opt=2, runs=3)
        if r:
            print(f"  {name:24s} avg={r[0]*1000:.0f} min={r[1]*1000:.0f} max={r[2]*1000:.0f}")
        else:
            print(f"  {name:24s} AOT FAILED")


def step_deltas() -> None:
    print("\n=== BENCH_STEP JIT+RT DELTA (run-build, ms) ===")
    prev = 0.0
    for i in range(1, 7):
        f = EXAMPLES / f"bench_step{i}.at"
        tb, _, _ = bench_build(f, runs=2)
        tr, _, _ = bench_run(f, runs=2)
        rt = (tr - tb) * 1000
        print(f"  step{i}: jit+rt={rt:.0f}  delta=+{rt-prev:.0f}")
        prev = rt


def summary(results: dict[str, tuple[int, int, int]]) -> None:
    if not results:
        return
    avgs = sorted([(k, v[1]) for k, v in results.items()], key=lambda x: -x[1])
    print("\n=== TOP 5 SLOWEST (avg ms) ===")
    for k, v in avgs[:5]:
        print(f"  {k}: {v}")
    print("=== TOP 5 FASTEST (avg ms) ===")
    for k, v in avgs[-5:]:
        print(f"  {k}: {v}")
    print(f"\nTOTAL benchmarks: {len(results)}")


def main() -> None:
    if not BIN.is_file():
        raise SystemExit(f"missing binary: {BIN} (run cargo build --release)")

    run_benchmark_sh()
    results = parse_results()
    phase_split()
    aot_runtime()
    step_deltas()
    summary(results)


if __name__ == "__main__":
    main()
