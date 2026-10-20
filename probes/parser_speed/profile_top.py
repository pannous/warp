#!/usr/bin/env python3
"""The functions a WARP_PROFILE profile (Firefox processed format) spent its CPU time in: self and total, the top N.
usage: profile_top.py <profile.json>… [--top N] [--callers NAME]   (several profiles: one run's stage programs, summed)
--callers NAME: the time in NAME by the CALLER_DEPTH frames that called it, nearest first"""
import json
import sys
from collections import Counter

TOP_DEFAULT = 30
CALLER_DEPTH = 3


def frame_names(thread, stack):
    stacks, frames, functions = (thread[key] for key in ("stackTable", "frameTable", "funcTable"))
    names = []
    while stack is not None:
        names.append(thread["stringArray"][functions["name"][frames["func"][stacks["frame"][stack]]]])
        stack = stacks["prefix"][stack]
    return names


def caller_times(thread, callee):
    samples = thread["samples"]
    cpu = samples.get("threadCPUDelta") or samples.get("weight") or [1] * samples["length"]
    callers = Counter()
    for stack, spent in zip(samples["stack"], cpu):
        names = frame_names(thread, stack)
        if callee in names:
            outermost = len(names) - 1 - names[::-1].index(callee)
            callers[" < ".join(names[outermost + 1 : outermost + 1 + CALLER_DEPTH])] += spent or 0
    return callers


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
    callee = None
    if "--callers" in arguments:
        at = arguments.index("--callers")
        callee = arguments[at + 1]
        del arguments[at : at + 2]
    callers = Counter()
    own, total = Counter(), Counter()
    for file in arguments:
        profile = json.load(open(file))
        file_time = 0
        for thread in profile["threads"]:
            thread_own, thread_total = thread_times(thread)
            own.update(thread_own)
            total.update(thread_total)
            file_time += sum(thread_own.values())
            if callee:
                callers.update(caller_times(thread, callee))
        print(f"{file_time / 1e6:8.2f} s  {file} ({profile['meta'].get('product', '')})")
    all_time = sum(own.values()) or 1
    print(f"{all_time / 1e6:.2f} s sampled")
    tables = (("self", own), ("total", total)) + (((f"callers of {callee}", callers),) if callee else ())
    for title, table in tables:
        print(f"\n{title}:")
        for name, spent in table.most_common(top):
            print(f"{spent / 1e6:8.2f} s {100 * spent / all_time:5.1f}%  {name}")


main()
