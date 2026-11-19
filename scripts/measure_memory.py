#!/usr/bin/env python3
"""Sample process RSS while running Bellbird's memory probe on Linux."""

from __future__ import annotations

import argparse
import csv
import re
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

METRIC_PATTERN = re.compile(r"\b(import_ms|render_ms|output_bytes)=([0-9.]+)")


def read_status(pid: int) -> dict[str, int] | None:
    try:
        lines = Path(f"/proc/{pid}/status").read_text(encoding="ascii").splitlines()
    except (FileNotFoundError, ProcessLookupError):
        return None

    metrics = {}
    for line in lines:
        if line.startswith("VmRSS:"):
            metrics["rss_kb"] = int(line.split()[1])
        elif line.startswith("VmHWM:"):
            metrics["hwm_kb"] = int(line.split()[1])
    return metrics if "rss_kb" in metrics and "hwm_kb" in metrics else None


def parse_probe_output(output: str) -> dict[str, float | int]:
    values = {name: value for name, value in METRIC_PATTERN.findall(output)}
    required = {"import_ms", "render_ms", "output_bytes"}
    if values.keys() < required:
        raise RuntimeError(f"probe output did not contain {sorted(required)}: {output.strip()}")
    return {
        "import_ms": float(values["import_ms"]),
        "render_ms": float(values["render_ms"]),
        "output_bytes": int(float(values["output_bytes"])),
    }


def probe_workload(command: list[str]) -> str:
    if len(command) < 2:
        raise ValueError("the probe command must include an input path")
    if Path(command[0]).name in {"cargo", "cargo.exe"}:
        raise ValueError(
            "pass the compiled memory_probe executable directly; launching through Cargo "
            "would sample Cargo's RSS"
        )
    return Path(command[1]).name


def sample_run(command: list[str], run_number: int, interval_seconds: float) -> dict[str, int | float | str]:
    started = time.monotonic()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    last_status = None
    max_sampled_rss = 0

    with ThreadPoolExecutor(max_workers=1) as executor:
        output_future = executor.submit(process.communicate)
        while not output_future.done():
            status = read_status(process.pid)
            if status is not None:
                last_status = status
                max_sampled_rss = max(max_sampled_rss, status["rss_kb"])
            time.sleep(interval_seconds)
        output, _ = output_future.result()

    elapsed_ms = (time.monotonic() - started) * 1_000.0
    if process.returncode != 0:
        raise RuntimeError(f"probe exited {process.returncode}: {output.strip()}")
    probe_metrics = parse_probe_output(output)
    final_status = read_status(process.pid) or last_status
    if final_status is None:
        raise RuntimeError("could not sample process RSS; the probe exited before the first sample")

    return {
        "run": run_number,
        "workload": probe_workload(command),
        "peak_rss_kb": max(max_sampled_rss, final_status["hwm_kb"]),
        "vmhwm_kb": final_status["hwm_kb"],
        "settled_rss_kb": last_status["rss_kb"] if last_status else final_status["rss_kb"],
        "elapsed_ms": round(elapsed_ms, 3),
        **probe_metrics,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runs", type=int, default=10)
    parser.add_argument("--interval-ms", type=int, default=25)
    parser.add_argument("--csv", required=True, type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()

    if not sys.platform.startswith("linux"):
        parser.error("RSS sampling requires Linux /proc/<pid>/status")
    if args.runs < 1 or args.interval_ms < 1:
        parser.error("--runs and --interval-ms must be positive")
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    if not command:
        parser.error("provide the probe command after --")
    try:
        probe_workload(command)
    except ValueError as error:
        parser.error(str(error))

    fields = [
        "run",
        "workload",
        "peak_rss_kb",
        "vmhwm_kb",
        "settled_rss_kb",
        "elapsed_ms",
        "import_ms",
        "render_ms",
        "output_bytes",
    ]
    args.csv.parent.mkdir(parents=True, exist_ok=True)
    with args.csv.open("w", newline="", encoding="utf-8") as output_file:
        writer = csv.DictWriter(output_file, fieldnames=fields)
        writer.writeheader()
        for run_number in range(1, args.runs + 1):
            record = sample_run(command, run_number, args.interval_ms / 1_000.0)
            writer.writerow(record)
            output_file.flush()
            print(",".join(str(record[field]) for field in fields), flush=True)


if __name__ == "__main__":
    main()
