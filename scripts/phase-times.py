#!/usr/bin/env python3
"""scripts/phase-times.py <log>…: the phases of a compile run with WARP_TIMES=1, from its `§T <milliseconds> <label>`
lines (a build record's build.log, notes/compiler_rebuilds.md "Build records"): each label's wall seconds until the
next mark, summed over its marks, the biggest first; a log's marks stop at its first `emit done`. Several logs (a
build and its stage two) are reported one after the other."""
import re, sys

MARK = re.compile(r"§T (\d+) (.*)")
LAST_LABEL = "emit done"

def marks_of(path):
    marks = []
    for line in open(path, errors="replace"):
        found = MARK.search(line)
        if not found: continue
        marks.append((found.group(2).strip(), int(found.group(1))))
        if marks[-1][0] == LAST_LABEL: break
    return marks

def report(path):
    marks = marks_of(path)
    spans = {}
    for (label, start), (_, stop) in zip(marks, marks[1:]):
        spans[label] = spans.get(label, 0) + (stop - start) / 1000
    total = (marks[-1][1] - marks[0][1]) / 1000 if marks else 0
    print(f"{path}: {total:.1f} s wall from the first mark to the last")
    for label, seconds in sorted(spans.items(), key=lambda item: -item[1]):
        print(f"{seconds:8.1f} s {100 * seconds / max(total, 1e-9):5.1f}%  {label}")

for path in sys.argv[1:]: report(path)
