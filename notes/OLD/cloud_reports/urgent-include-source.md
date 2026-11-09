done
# include-source: the bug button's "include source" checkbox

(asked in the warp-urgent session, 2026-10-17) The bug form (web/playground/index.html) has a checkbox
`include source`, ticked by default, before `express`. Unticked, playground.js bugReport sends the comment without
the program and its output (version and page still go along). scripts/bug_report.py card_of leaves the program and
the Output block out of the card when they're empty.

direct to main 4d427c68f

needs the Mac for the server half: `python3 scripts/bug_report.py install` puts the new card_of on pannous.com; until
then a report without source makes a card with empty program and output blocks. The page half deploys with Playground.

Tests: none run (express mode); `node --check`, `py_compile`, card_of printed with and without source.

Try: the playground's bug button: untick "include source", send: the card holds the comment alone.
