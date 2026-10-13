#!/usr/bin/env python3
"""Keep only the hunks of a unified diff whose text contains one of the markers (files without markers keep all hunks).
usage: git diff | impl-text_hunks.py 'file=marker1|marker2' ... > mine.patch"""
import re, sys
rules = dict(arg.split('=', 1) for arg in sys.argv[1:])
text = sys.stdin.read()
out = []
for file_diff in re.split(r'(?m)^(?=diff --git )', text):
    if not file_diff.strip():
        continue
    name = re.match(r'diff --git a/(\S+)', file_diff).group(1)
    parts = re.split(r'(?m)^(?=@@ )', file_diff)
    head, hunks = parts[0], parts[1:]
    if name in rules:
        markers = rules[name].split('|')
        hunks = [h for h in hunks if any(m in h for m in markers)]
    if hunks:
        out.append(head + ''.join(hunks))
sys.stdout.write(''.join(out))
