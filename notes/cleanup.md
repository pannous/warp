# Cleanup: rules and rounds
Roadmap card: cleanup (board topic; sub-cards are the areas and their leftovers, notes/roadmaps.md). Recurring rounds:
cleanup-stale-notes (notes/), cleanup-web (web/playground/ JS), decisions-prune (notes/decisions.md), consistency
(docs against behaviour).

## Rules
- Behaviour-preserving only: shared helpers instead of duplication, dead code and stale comments out, flatter control
  flow, meaningful names, no test edits.
- The measure is how hard the code is to follow.
- Watch for closed lists: a hard-coded list of words standing in for a mechanism breaks on every word it misses. Derive
  such a list from its source of truth (the type registry, the library's exports, the operator table) instead of adding
  the missing word (common.md: core/ holds mechanisms only).
- A stale note goes (user: outdated information is removed, git keeps it); where a solved problem still matters, one
  line here or in the topic note says how it was solved.

## Stale-notes rounds
- 2026-11-06: the Rust-era cleanup tooling and code map (dead_scan.py, split_module.py, same_wasm.sh, lowering/nodes.rs,
  CompileContext, the Rust leftovers) left this file: the Rust compiler and harness are gone (branches rust-bootstrap,
  rust-harness). Notes that described only Rust internals were deleted: console, delete_list, binaryen,
  error_positions, single_statement_block_calls, annotations, parenthesized_tuples (round brackets are a plain list
  now, decision immutable-tuple), ranges (decision lazy-range-value supersedes it).
