#!/usr/bin/env python3
"""Classifies the differences between the warp analyzer and the Rust one (card lib-compiler, the flow inference):
per function parameter, result, variable and value, the Rust kind → the warp kind. warp more general (a node where
Rust has a number, a run-time number where Rust has an int or float) keeps every value and only costs speed; warp
narrower may be unsound (a bug on either side); int → float can change printed values; a name only one side has is
listed apart.
Usage: probes/self_hosting/analyzer/generality.py [data/self_analyzer/differing.jsonl] [--examples N]"""
import json, re, sys, collections

DIFFERING = "data/self_analyzer/differing.jsonl"
SEPARATOR = " | "
# a node holds any value; data any number
NODE, NUMBER = "empty", "data"
NUMBERS = {"int", "float"}


def slots(text):
    """{slot name: kind} of one side's kinds text"""
    found = {}
    for line in text.split(SEPARATOR):
        function = re.match(r"^(.*)\((.*)\) -> (.*)$", line)
        if function:
            name, params, result = function.groups()
            for index, kind in enumerate(filter(None, params.split(", "))):
                found[f"{name}({index})"] = kind
            found[f"{name}()"] = result
        elif line.startswith("= "):
            found["="] = line[2:]
        elif ": " in line:
            name, kind = line.rsplit(": ", 1)
            found[name] = kind
    return found


def verdict(rust, warp):
    if warp == NODE:
        return "more general"
    if warp == NUMBER and rust in NUMBERS:
        return "more general"
    if (rust, warp) == ("int", "float"):
        return "int → float (values may print differently)"
    return "narrower or other"


def main():
    args = sys.argv[1:]
    examples = int(args[args.index("--examples") + 1]) if "--examples" in args else 3
    path = next((a for a in args if a.endswith(".jsonl")), DIFFERING)
    changes = collections.Counter()
    verdicts = collections.Counter()
    where = collections.defaultdict(list)
    for line in open(path):
        case = json.loads(line)
        if case["warp"].startswith("warp failed"):
            verdicts["warp failed"] += 1
            where["warp failed"].append(case["path"])
            continue
        warp, rust = slots(case["warp"]), slots(case["rust"])
        for name in sorted(set(warp) | set(rust)):
            if name not in rust or name not in warp:
                side = "only warp" if name not in rust else "only rust"
                verdicts[side] += 1
                where[side].append(f"{case['path']} {name}")
                continue
            if warp[name] != rust[name]:
                change = f"{rust[name]} → {warp[name]}"
                changes[change] += 1
                verdicts[verdict(rust[name], warp[name])] += 1
                where[change].append(f"{case['path']} {name}")
    print("verdicts:")
    for name, count in verdicts.most_common():
        print(f"  {count:5} {name}")
    print("changes (rust → warp):")
    for change, count in changes.most_common():
        print(f"  {count:5} {change:24} {'; '.join(where[change][:examples])}")
    for side in ["only warp", "only rust", "warp failed"]:
        if where[side]:
            print(f"{side}: {'; '.join(where[side][:examples * 3])}")


main()
