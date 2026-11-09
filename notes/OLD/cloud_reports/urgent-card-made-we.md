done
# card-made-we: the bug button links the board card too (#1758)

On top of card-made (claude/urgent-card-made): web/playground/playground.js sendBugReport reads `board` of the server's
answer (scripts/bug_report.py on main, e4f30bb73: `{card, board}`) and showBugStatus shows
`card made: #1755 (board)`, #1755 linking the issue and `board` the board card, both in a new tab. An answer without
`board` (an older server) shows `card made: #1755` as before.

Tests: `node --check web/playground/playground.js`; showBugStatus rendered in headless Chromium:
`card made: <a href=".../issues/1755">#1755</a> (<a href=".../projects/1?pane=issue&itemId=42">board</a>)`, and
without board `card made: <a …>#1755</a>`. The browser suite's BUG_BUTTON check (refused token) is unchanged, not run
here.

Try: the playground's bug button with a comment: the status ends in `#<issue> (board)`.
