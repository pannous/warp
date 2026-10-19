#!/usr/bin/env python3
"""The slots the twin test (tests/common/twins.rs covers) fails on, per differing source of the last twin_loop.py run:
(slot, warp kind, rust kind) where warp's kind is missing, extra or narrower than Rust's.
Usage: probes/self_hosting/analyzer/narrower_slots.py [path-substring…]"""
import json, re, sys

DIFFERING = "data/self_analyzer/differing.jsonl"


def slots(lines):
    found = {}
    for line in lines.split(" | ") if isinstance(lines, str) else lines:
        if line.startswith("= "):
            found["="] = line[2:]
            continue
        call = re.match(r"(.*?)\((.*)\) -> (.*)", line)
        if call:
            for index, kind in enumerate(k for k in call[2].split(", ") if k):
                found[f"{call[1]}({index})"] = kind
            found[call[1] + "()"] = call[3]
        elif ": " in line:
            name, kind = line.split(": ", 1)
            found[name] = kind
    return found


def general(warp, rust):
    return warp == rust or warp == "empty" or (warp == "data" and rust in ("int", "float"))


wanted = sys.argv[1:]
for line in open(DIFFERING):
    case = json.loads(line)
    if wanted and not any(w in case["path"] for w in wanted):
        continue
    warp, rust = slots(case["warp"]), slots(case["rust"])
    bad = [(s, warp.get(s), k) for s, k in rust.items() if not (s in warp and general(warp[s], k))]
    bad += [(s, warp[s], None) for s in warp if s not in rust]
    if bad:
        print(case["path"], bad)
