# Reviewer handover

- Log: ~/Library/Logs/warp/reviews/<date>.md
- Last reviewed: 864b03d554. Queue: `git log --merges --first-parent --oneline 864b03d554..origin/main`.
- Open findings (cards): comment-after (`if c // note` + indented body: the comment is read as the body, now
  "unknown name"), expected-type (spaced and method calls skip names-from-the-expected-type, silently wrong; an
  ambiguous case name is silent), pi-power-text, conversion-target (restructuring), enum-results-table (plain_enums,
  class_methods and the analyzer each guess static types early; one table).
- Night triage 2026-10-28 is done: notes/test_triage.md (all folders), cards triage-* (1-10), silent-wrong-results,
  silent-wrong-results-2 (warp-separate-modules), module-hygiene (warp-tables-as-constants). Holds restore per the note's
  header; data/triage/hold.py in the main checkout's data/ applied them. Questions for 10:00 sit in the note per folder.
