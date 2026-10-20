#!/usr/bin/env python3
"""The functions a WARP_PROFILE profile (Firefox processed format) spent its CPU time in: self and total, the top N.
usage: profile_top.py <profile.json>… [--top N]   (several profiles: one run's stage programs, summed)"""
import json
import sys
from collections import Counter

TOP_DEFAULT = 30


def thread_times(thread):
    samples, stacks, frames, functions = (thread[key] for key in ("samples", "stackTable", "frameTable", "funcTable"))
    names = [thread["stringArray"][index] for index in functions["name"]]
    cpu = samples.get("threadCPUDelta") or samples.get("weight") or [1] * samples["length"]
    own, total = Counter(), Counter()
    for stack, spent in zip(samples["stack"], cpu):
        spent = spent or 0
        seen = set()
        first = True
        while stack is not None:
            name = names[frames["func"][stacks["frame"][stack]]]
            if first:
                own[name] += spent
                first = False
            if name not in seen:
                total[name] += spent
                seen.add(name)
            stack = stacks["prefix"][stack]
    return own, total


def main():
    arguments = sys.argv[1:]
    top = TOP_DEFAULT
    if "--top" in arguments:
        at = arguments.index("--top")
        top = int(arguments[at + 1])
        del arguments[at : at + 2]
    own, total = Counter(), Counter()
    for file in arguments:
        profile = json.load(open(file))
        file_time = 0
        for thread in profile["threads"]:
            thread_own, thread_total = thread_times(thread)
            own.update(thread_own)
            total.update(thread_total)
            file_time += sum(thread_own.values())
        print(f"{file_time / 1e6:8.2f} s  {file} ({profile['meta'].get('product', '')})")
    all_time = sum(own.values()) or 1
    print(f"{all_time / 1e6:.2f} s sampled")
    for title, table in (("self", own), ("total", total)):
        print(f"\n{title}:")
        for name, spent in table.most_common(top):
            print(f"{spent / 1e6:8.2f} s {100 * spent / all_time:5.1f}%  {name}")


main()
