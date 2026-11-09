done
# card-made-issue: the bug button names the issue and the card, both as links

The status after a report reads `card made: issue #1762 · card`: `#1762` links the issue, `card` the board card
(the server's `board`, scripts/bug_report.py), no raw address. web/playground/playground.js showBugStatus; the branch
carries card-made and card-made-we (claude/urgent-card-made-we, 048b9f3ef) merged in, so those land with it.

direct to main 59551f0b6

Tests: `node --check web/playground/playground.js`; showBugStatus rendered in headless Chromium:
`card made: issue <a href=".../issues/1762">#1762</a> · <a href=".../projects/1?pane=issue&itemId=274490218">card</a>`.

Try: the playground's bug button with a comment: the status ends in `issue #<n> · card`.
