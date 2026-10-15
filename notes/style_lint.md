# Style linter

`scripts/style_lint.py [--fix] [--base REF] [path...]`: the style rules that rustfmt and clippy don't cover (card
style-linter). Reviews no longer polish style by hand (user, 2026-10-15); a rule the reviewer keeps seeing goes into
this script.

- Runs on the .rs files changed since the merge base of HEAD with `--base` (default origin/main): workers before a merge
  request (notes/agents/common.md), the Integrator's gate.sh after clippy, which commits what `--fix` repaired as
  `chore: style_lint --fix` and prints `STYLE n fixed, m left`.
- Prints `path:line: [rule] message`, exits 1 while a violation is left.
- A new rule needs the user's yes first unless AGENTS.md, notes/decisions.md or the wiki already states it (user,
  2026-10-15): ask through warp-interviewer.

## Rules

- **doc-moved** (user, 2026-10-15): a `///` line that documents item X at the base now sits on a new item Y: Y was
  inserted between X and its doc (four review fixes on 2026-10-14/15). Found by mapping each doc line that occurs
  once in the base file to its item (scope path + name, so `impl A`'s `new` and `impl B`'s `new` differ). `--fix` moves
  the lines back above X when X has no doc now; otherwise it only reports. Doc lines shorter than 12 characters are
  not tracked (they repeat).

Proposed and declined by the user (2026-10-15): `use` lines at the top only, `// card K` on the mod line of the module
naming card K, a blank line between items. Don't re-propose them.

## How it reads Rust

No parser crate: a lexer blanks comments, strings, raw strings and char literals (lifetimes stay), then braces at
parenthesis depth 0 open scopes. Items are tracked in file, `mod`, `impl`, `trait` and `extern ""` scopes; `fn` bodies,
structs and enums are skipped. An item ends at `;` or at its closing `}` (consts, statics, uses and type aliases only at
`;`). Probe: probes/style_lint_doc_moved.sh replays three of the review cases and checks `--fix` equals the hand fix.
