#!/usr/bin/env python3
"""Fast loop for the analyzer twin (card inferred-kinds): runs lib/compiler/analyzer.warp `inferred_kinds` with the
checkout's own warp (scratch/warp) over the cases the survey test wrote (data/self_analyzer/cases.jsonl: path, Rust
kinds, program view) and lists the differences, without a cargo test run.
Usage: probes/self_hosting/analyzer/twin_loop.py [path-substring] [--limit N] [--chunk N]"""
import json, subprocess, sys, os, collections
from concurrent.futures import ThreadPoolExecutor

JOBS = 4

CASES = "data/self_analyzer/cases.jsonl"
CASE_FOLDER = "scratch/analyzer_cases"
WARP = "scratch/warp"
SEPARATOR = " | "
# the sources whose kinds match, written by each run
MATCHING = "data/self_analyzer/matching.txt"


def arguments():
    args = sys.argv[1:]
    option = lambda name, default: int(args[args.index(name) + 1]) if name in args else default
    words = [a for i, a in enumerate(args) if not a.startswith("--") and (i == 0 or not args[i - 1].startswith("--"))]
    return (words[0] if words else ""), option("--limit", 10**9), option("--chunk", 48)


def warp_kinds(chunk):
    calls = [f'print(inferred_kinds(node_of(json(read("{CASE_FOLDER}/{index}.json")))).join("{SEPARATOR}"))'
             for index, _ in chunk]
    program = 'use "lib/compiler/analyzer.warp"\n' + "\n".join(calls) + "\n0"
    run = subprocess.run([WARP, program], capture_output=True, text=True, timeout=600,
                         env={**os.environ, "WARP_NO_WINDOW": "1"})
    lines = [line for line in run.stdout.splitlines() if SEPARATOR in line or line.startswith("= ")]
    if len(lines) == len(chunk):
        return lines
    if len(chunk) == 1:
        return ["warp failed: " + (run.stderr or run.stdout).strip()[-300:]]
    half = len(chunk) // 2
    return warp_kinds(chunk[:half]) + warp_kinds(chunk[half:])


def main():
    only, limit, chunk_size = arguments()
    os.makedirs(CASE_FOLDER, exist_ok=True)
    cases = [json.loads(line) for line in open(CASES)]
    cases = [case for case in cases if only in case["path"]][:limit]
    for index, case in enumerate(cases):
        with open(f"{CASE_FOLDER}/{index}.json", "w") as out:
            json.dump(case["program"], out)
    differing = []
    chunks = [list(enumerate(cases))[start:start + chunk_size] for start in range(0, len(cases), chunk_size)]
    with ThreadPoolExecutor(JOBS) as pool:
        results = list(pool.map(warp_kinds, chunks))
    for chunk, values in zip(chunks, results):
        for (index, case), got in zip(chunk, values):
            expected = SEPARATOR.join(case["kinds"])
            if got != expected:
                differing.append((case["path"], got, expected))
    differing_paths = {path for path, _, _ in differing}
    with open(MATCHING, "w") as out:
        out.writelines(case["path"] + "\n" for case in cases if case["path"] not in differing_paths)
    words = collections.Counter()
    for path, got, expected in differing:
        print(f"## {path}\n  warp: {got[:400]}\n  rust: {expected[:400]}")
        got_lines, expected_lines = set(got.split(SEPARATOR)), set(expected.split(SEPARATOR))
        words.update(line for line in expected_lines - got_lines)
    print(f"{len(differing)} of {len(cases)} differ; most missed Rust lines:")
    for line, count in words.most_common(25):
        print(f"  {count:4} {line}")


main()
