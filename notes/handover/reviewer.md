# Reviewer handover

- Log: ~/Library/Logs/warp/reviews/<date>.md
- Last reviewed: f737f01a16. Queue: `git log --merges --first-parent --oneline f737f01a16..origin/main`.
- Open findings (cards): comment-after (`if c // note` + indented body: the comment is read as the body, now
  "unknown name"), expected-type (spaced and method calls skip names-from-the-expected-type, silently wrong; an
  ambiguous case name is silent), pi-power-text, conversion-target (restructuring).
