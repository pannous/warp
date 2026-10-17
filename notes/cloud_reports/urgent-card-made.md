done
# card-made: the playground's "card made: …" should be a link

The bug button's status showed the card as plain text ("card made: https://github.com/pannous/warp/issues/1755").
Now web/playground/playground.js showBugStatus takes the card and appends a link after the text, named `#1755`
(cardName; any other address shows as it is), opening in a new tab:
`card made: <a href="https://github.com/pannous/warp/issues/1755" target="_blank" rel="noopener">#1755</a>`.

card-made-2 (#1757) was the same report as card-made (#1756), already fixed in 11636d7a4 (scripts/bug_report.py
queues a report for the Mac's pull only when its card failed). The user's other report #1758 ("can we (also) link to
the project card instead of the issue?") is not done here: the server answers only the issue url; a project item link
needs scripts/bug_report.py to answer the board item too.

Tests: `node --check web/playground/playground.js`; showBugStatus and cardName rendered in headless Chromium give the
link above. The browser suite (web/playground/test_in_browser.py, BUG_BUTTON) checks only the refused-token path, which
is unchanged; not run here (needs pannous.com and the playground build).

Try: the playground's bug button with a comment: the status ends in a link #<issue>.
