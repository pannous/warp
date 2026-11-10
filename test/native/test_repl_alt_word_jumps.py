#!/usr/bin/env python3
"""Alt+←/→ move the cursor a word back and a word on (card alt-arrow-navigation), in the escape sequences macOS
Terminal (Option as Meta), xterm and iTerm (Option as Esc+) send, on `warp repl` in a pseudo terminal
(test_repl_keys.py drives it). Run: python3 test/native/test_repl_alt_word_jumps.py [native warp]"""
import sys
import test_repl_keys as console

ALT_LEFTS = ["\x1bb", "\x1b[1;3D", "\x1b\x1b[D"]
ALT_RIGHTS = ["\x1bf", "\x1b[1;3C", "\x1b\x1b[C"]
console.CASES = [case for left, right in zip(ALT_LEFTS, ALT_RIGHTS) for case in [
    ("1 + 2" + left + "1", "13"),
    ("10 + 20" + left + left + left + "3", "330"),
    ("1 + 2" + left + left + left + right + "0", "12"),
]]

if __name__ == "__main__":
    sys.exit(console.main())
