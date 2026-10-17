#!/usr/bin/env python3
"""Rewrite calls `path::name(arg)` / `name(&arg)` into `arg.method()` across files:
probes/cleanup/call_to_method.py <name> <method> <file>...   (merging a free helper into a Node method)"""
import re, sys
from pathlib import Path

SIMPLE = re.compile(r"[\w.:]+(\([^()]*\))?$")


def rewrite(source, name, method):
    pattern = re.compile(r"(?<![\w.])(?:[\w]+::)*" + name + r"\(")
    out, position = [], 0
    for match in pattern.finditer(source):
        if match.start() < position or source[max(0, match.start() - 3):match.start()] == "fn ":
            continue
        depth, end = 1, match.end()
        while depth:
            depth += {"(": 1, ")": -1}.get(source[end], 0)
            end += 1
        argument = source[match.end():end - 1].strip().removeprefix("&")
        receiver = argument if SIMPLE.match(argument) else f"({argument})"
        out.append(source[position:match.start()] + f"{receiver}.{method}()")
        position = end
    return "".join(out) + source[position:]


name, method = sys.argv[1:3]
for file in sys.argv[3:]:
    path = Path(file)
    before = path.read_text()
    after = rewrite(before, name, method)
    if after != before:
        path.write_text(after)
        print(f"{file}: {before.count(name + '(') - after.count(name + '(')} calls")
