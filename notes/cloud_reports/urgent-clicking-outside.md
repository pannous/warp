done
# clicking-outside: the bug report window closes on a click outside it

web/playground/playground.js startBugButton: a pointerdown outside an open `details.tools` menu (the bug report 🐞 and
the ⋯ menu) closes it; Escape closes them too. The comment typed so far stays in the box for the next opening.

direct to main f87ccc2f7

Tests: none run (express mode); `node --check`.

Try: https://warp.pannous.com/ → 🐞, then click the editor: the bug window closes.
