# Interviewer handover

Read notes/agents/common.md + notes/agents/interviewer.md first; they beat this file.

## Open with the user
- Handler shape, a prose discussion the user opened ("let's discuss": answer in prose, no AskUserQuestion until it
  converges). Proposed: `on(event: Event, body: block)` in the prelude, `listeners[event] += [body]`, `emit(event,
  data)` runs them, host contract = one exported emit + listen; `on frame` finds Event.frame via expected-type-names;
  shared captures make `x -= 4` change main's x; no handler code in core; host values to lib (card whole-table).
  Three questions put to the user, unanswered: (1) event data as `it`, a chosen name (`on key k {…}`), or both;
  (2) must user events (`ask`) be declared (`enum Event += ask`) or any symbol counts; (3) does `on frame` run once
  headless or never. Parked branch event-handlers-unknown (a4d0dde771) only as reference; warp-samples builds it after.
- notes/open_decisions.md: Pending empty, To present empty; Parked has the event-handlers line and older items.

## Waiting on workers (no user question)
- warp-text-field: expected-type-names part 1, OK given with the change "index class → bundled module, no
  typed_modules list"; enum-identity-per-enum card (Level.Error == ValueKind.Error must become no).
- warp-samples: data-parameters (user OK) landing; class-operator-overloads (OK, under overloads-before-reals) then
  lib/reals.warp; try lines for block-parameters-captures ran green with a worktree build (main's binary was stale).
- warp-fixer: declared-class stages 1–2 (user kept), condition: structs and Node values convert automatically.

## Today's user decisions (all in notes/decisions.md)
elegance-first (in every role file), enum-raw-values, kinds-as-cases, enum-identity, declared-class, class-field-slots,
instance-position (+ card class-position-index Later), file-words-implicit, to-inclusive, fixity-operators-only,
handler-blocks-share, handlers-wait-for-blocks, shared-captures, units-as-values, expected-type-names,
data-parameters, overloads-before-reals, built-in-lib refinement, n-times-prelude-now.

## Tools
Notes worktree: /Volumes/Worktrees/interviewer-notes (detached; `git pull --rebase origin main`, push HEAD:main).
Its lib/warp.wasm is built from a150d5f2b9 (scripts/rebuild.sh pure <tree>) for running try lines.
