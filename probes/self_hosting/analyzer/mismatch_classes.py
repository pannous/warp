#!/usr/bin/env python3
"""Groups the analyzer twin's differences (the output of twin_loop.py) by definition: each line's name with warp's and
Rust's kind, counted over the sources, the most frequent first.
Usage: probes/self_hosting/analyzer/mismatch_classes.py data/self_analyzer/loop_all.txt [top]"""
import collections, re, sys

SEPARATOR = " | "


def by_name(lines):
    named = {}
    for line in lines:
        if line.startswith("= "):
            named["="] = line
        else:
            named[re.split(r"[(:]", line, 1)[0]] = line
    return named


def main():
    text = open(sys.argv[1]).read()
    top = int(sys.argv[2]) if len(sys.argv) > 2 else 40
    pairs = collections.Counter()
    examples = {}
    for path, warp, rust in re.findall(r"## (\S+)\n  warp: (.*)\n  rust: (.*)\n", text):
        warp_lines, rust_lines = by_name(warp.split(SEPARATOR)), by_name(rust.split(SEPARATOR))
        for name in set(warp_lines) | set(rust_lines):
            pair = (warp_lines.get(name, "-"), rust_lines.get(name, "-"))
            if pair[0] != pair[1]:
                pairs[pair] += 1
                examples.setdefault(pair, path)
    for (warp, rust), count in pairs.most_common(top):
        print(f"{count:4}  warp {warp:40.40}  rust {rust:40.40}  {examples[(warp, rust)]}")


main()
