#!/usr/bin/env python3
"""Run the code of every `is!("…", …)` in the given test files with two warp binaries and print where they differ:
a behaviour diff without the test queue (card one-dispatch). Usage:
  probes/dispatch/compare_binaries.py <old warp> <new warp> tests/types/test_x.rs …"""
import os
import re
import subprocess
import sys

TIMEOUT_SECONDS = 20
RESULT_MARK = '»'
IS_CALL = re.compile(r'is!\(\s*"((?:[^"\\]|\\.)*)"', re.S)
ESCAPES = {'n': '\n', 't': '\t', '"': '"', '\\': '\\', "'": "'", '0': '\0'}


def unescaped(literal):
    return re.sub(r'\\(.)', lambda match: ESCAPES.get(match.group(1), match.group(0)), literal)


def result_of(binary, code):
    environment = dict(os.environ, WARP_NO_WINDOW='1', WARP_HINTS='0', WARP_CRASH_CARDS='0')
    try:
        run = subprocess.run([binary, code], capture_output=True, text=True, timeout=TIMEOUT_SECONDS, env=environment)
    except subprocess.TimeoutExpired:
        return 'TIMEOUT'
    lines = [line for line in (run.stdout + run.stderr).splitlines() if line.strip()]
    # the result line, not a warning printed after it; temporary file names differ on every run
    results = [line for line in lines if line.startswith(RESULT_MARK)] or lines[-1:]
    return re.sub(r'\d+\.wav', 'N.wav', results[-1]) if results else ''


def main():
    old, new, *files = sys.argv[1:]
    differing = 0
    for path in files:
        for literal in IS_CALL.findall(open(path).read()):
            code = unescaped(literal)
            before, after = result_of(old, code), result_of(new, code)
            if before != after:
                differing += 1
                print(f'{path}\n  code:   {code!r}\n  before: {before}\n  after:  {after}')
    print(f'{differing} differing')


main()
