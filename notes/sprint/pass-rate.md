# Pass-rate sprint (user 2026-11-06)

After the Grok integration, the sprint stops new features and raises the test pass rate. Baseline:
test/results.txt of 03:46, 2630 red lines of 15269.

| Worker | Folders (red lines) |
|---|---|
| warp-sprint-runtime-cleanup | control (551) |
| warp-sprint-types | types (540) |
| warp-sprint-spec | functions (444) |
| warp-sprint-units-time | numbers (178), time (2) |
| warp-sprint-tooling | modules (172), scope (7), sweeps (3) |
| warp-sprint-text-collections | lists (109), text (83) |
| warp-sprint-web | ffi (99), wasm (70), web (7) |
| warp-sprint-media-samples | welcoming (92), programs (69) |
| warp-markup-handler-spacing | parser (90), operators (80), node (34), after its Duration and visibility landings |

## How
- Cluster first: run your folders (`nice -n 19 warp test <folder>`, at most two folders a run), group the red lines by
  their error message, and fix the biggest cluster first. One cause fixed often clears dozens of lines.
- Question every card and red line first (user, praised the rejections of Rust-era examples and esoteric edge cases
  nobody needs): is it really worth it? Unsure → warp-interviewer with a recommendation; never build it just because
  it is listed.
- Judge each red line (notes/agents/common.md, memory judge-failing-tests): a line pinning esoteric old syntax or a
  known flaw goes to warp-interviewer as a question, never a quiet test edit. Never modify an existing test without
  the user's OK; a test may be upgraded when the code now does better.
- Fix in lib/ or extensions/ where the cause lives there. A core cause: a small generic fix (a few lines, no new
  form) is an FYI to warp-interviewer and needs self-check.sh; anything bigger asks first.
- One owner per card and per cluster: `todo take` the card of a cluster you work on (add one if none exists).
- Report `fixed: <cluster> <commit> <red lines before → after>`; land as before (smoke on the merge, push).
