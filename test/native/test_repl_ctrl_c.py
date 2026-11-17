#!/usr/bin/env python3
"""Ctrl+C in the native console (cards ctrl-native, playground-ls-interrupt): on a line being typed it drops the line,
on an empty line it is never kept for the next one (the line after it runs whole); a second one there ends the
console (test_repl_keys.py sends Ctrl+D after the cases). Run: python3 test/native/test_repl_ctrl_c.py [native warp]"""
import sys
import test_repl_keys as console

CTRL_C = "\x03"
console.CASES = [
    ("1 + 2" + CTRL_C + "40 + 2", "42"),
    (CTRL_C + "6 * 8", "48"),
]

if __name__ == "__main__":
    sys.exit(console.main())
