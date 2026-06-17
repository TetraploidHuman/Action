#!/usr/bin/env python3
"""Compare benchmark results against a baseline; fail on large regressions."""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LINE_RE = re.compile(r"^(bench_\S+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(PASS|FAIL)")


def parse_results(path: Path) -> dict[str, tuple[int, int, int, str]]:
    results: dict[str, tuple[int, int, int, str]] = {}
    if not path.is_file():
        raise FileNotFoundError(f"benchmark results not found: {path}")
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = LINE_RE.match(line)
        if m:
            name = m.group(1)
            results[name] = (int(m.group(2)), int(m.group(3)), int(m.group(4)), m.group(5))
    return results


def compare(
    baseline: dict[str, tuple[int, int, int, str]],
    current: dict[str, tuple[int, int, int, str]],
    threshold: float,
) -> list[str]:
    warnings: list[str] = []
    for name, (_, base_avg, _, base_status) in sorted(baseline.items()):
        if base_status != "PASS":
            continue
        if name not in current:
            warnings.append(f"{name}: missing in current results")
            continue
        _, cur_avg, _, cur_status = current[name]
        if cur_status != "PASS":
            warnings.append(f"{name}: baseline PASS but current {cur_status}")
            continue
        if base_avg <= 0:
            continue
        ratio = (cur_avg - base_avg) / base_avg
        if ratio > threshold:
            pct = ratio * 100.0
            warnings.append(
                f"{name}: avg {cur_avg} ms vs baseline {base_avg} ms (+{pct:.1f}%)"
            )
    return warnings


def main() -> int:
    parser = argparse.ArgumentParser(description="Detect benchmark regressions vs baseline")
    parser.add_argument(
        "baseline",
        nargs="?",
        default=str(ROOT / "benchmark_results_aot_o2.txt"),
        help="Baseline results file (default: benchmark_results_aot_o2.txt)",
    )
    parser.add_argument(
        "current",
        nargs="?",
        default=str(ROOT / "benchmark_results.txt"),
        help="Current results file (default: benchmark_results.txt)",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=0.20,
        help="Max allowed avg slowdown fraction (default: 0.20 = 20%%)",
    )
    args = parser.parse_args()

    baseline_path = Path(args.baseline)
    current_path = Path(args.current)

    try:
        baseline = parse_results(baseline_path)
        current = parse_results(current_path)
    except FileNotFoundError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if not baseline:
        print(f"error: no benchmark rows in baseline {baseline_path}", file=sys.stderr)
        return 2
    if not current:
        print(f"error: no benchmark rows in current {current_path}", file=sys.stderr)
        return 2

    regressions = compare(baseline, current, args.threshold)
    if not regressions:
        print(
            f"OK: {len(current)} benchmarks within {args.threshold * 100:.0f}% of "
            f"baseline ({baseline_path.name})"
        )
        return 0

    print(f"WARNING: {len(regressions)} benchmark(s) exceeded {args.threshold * 100:.0f}% threshold:")
    for msg in regressions:
        print(f"  - {msg}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
