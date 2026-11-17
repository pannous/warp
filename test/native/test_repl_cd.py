#!/usr/bin/env python3
"""`cd` in the native console moves the console itself, which a shell's own builtin cd in a child sh never could
(card pwd-bug): the folder `cd` gives, then the commands after it running there. Run: python3
test/native/test_repl_cd.py [native warp] (test_repl_keys.py drives it)"""
import sys
import test_repl_keys as console

console.CASES = [
    ("cd /usr", '"/usr"'),
    ("pwd", "/usr"),
    ("cd ..", "'/'"),
    ("pwd", "/"),
]

if __name__ == "__main__":
    sys.exit(console.main())
