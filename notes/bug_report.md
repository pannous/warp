# The playground's bug button (card bug-report)

🐞 in the playground toolbar opens a form: a comment, an `express` checkbox, send. The comment travels with the
program, its output, the version and the page address, and becomes a card on the board.

    browser ──POST json──▶ https://pannous.com/warp-bug-report (Ferron location, ~/dev/pannous-lockdown ferron.kdl)
            ──▶ 127.0.0.1:5691, unit warp-bug-report: scripts/bug_report.py serve (DynamicUser, token via LoadCredential)
            ──▶ /var/lib/warp-bug-report/<id>.json ──▶ at once: issue + board item in Next (express: Now; a comment saying "later": Later), GitHub token
                /etc/warp/github-token (LoadCredential github; needs Issues + Account→Projects write) ──▶ done/
    fallback, Mac cron (every 2 min): scripts/bug_report.py pull ── ssh list ─▶ todo add (Next; express: --urgent → Now) ─▶ ssh ack
    The card's key comes from the Mac's `todo keys` (urgent-card-watch, every 2 min).

- Why the server: the browser can't hold a GitHub secret. A refused card (journalctl -u warp-bug-report says why)
  leaves the report queued for the Mac's pull, which has the board's token (~/.config/gh/cron-token).
- Token: one shared secret, made by `scripts/bug_report.py install` in ~/.config/warp/bug-report-token (Mac) and
  /etc/warp-bug-report.token (server). `scripts/bug_report.py token` prints the playground address
  `?bug_token=…` that stores it in a browser (localStorage) and strips it from the address bar. playground.js
  carries the token itself (BUG_TOKEN; the user accepts the abuse risk), so the parameter is only needed for another
  one: a new token from `install` means updating BUG_TOKEN. A refused token (403) is forgotten and the form shows a
  token field.
- Limits: with the shared token an address may send 10 reports an hour (429 after that); past 100
  attempts within an hour it is banned for good (/var/lib/warp-bug-report/banned.txt; delete its line and restart the
  unit to lift it). The address is the last X-Forwarded-For entry Ferron adds. The user's own owner token
  (~/.config/warp/bug-report-owner-token, /etc/warp-bug-report.owner-token) has no limit: `scripts/bug_report.py token`
  prints the playground address that stores it in a browser. An owner token rather than an IP range: the user's home
  address changes, and a German ISP's range is shared with everyone else on it.
- Redeploy the server side after editing the script: `scripts/bug_report.py install` (copies itself, the unit, the
  token; restarts).
- Test: test_in_browser.py check "(the bug button …)" sends a wrong token to the real endpoint (no card is made) and
  checks the report's fields; `scripts/browser-dry-run.sh --chrome "(the bug button sends a comment …)"`.
