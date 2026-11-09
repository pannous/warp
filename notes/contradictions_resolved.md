# Contradictions in decisions.md: the Interviewer's screen (11-09)

Rule used: the current code and the later decision win; the losing entry is deleted from decisions.md (or reworded
where both halves stay true). warp-decisions-tidy applies these. "later wins" means: compare the two entries' commit
dates (`git log -S'<key>' -- notes/decisions.md`) and keep the newer.

Process and platform
- quick-merges vs no-fixpoint: no-fixpoint wins (CLAUDE.md: smoke + self test, a fixpoint is only reported).
- quick-merges vs self-check-soft: quick-merges wins: core/ changes merge on smoke + self test (CLAUDE.md Testing).
- self-landing vs Handoff: Handoff wins (workers hand to the integrator). Delete self-landing.
- stages-passes-dirs vs layout: the subfolders (core/parsing, passes, analysis, emitter) are the code; delete stages-passes-dirs.
- pure-vs-self / compiler-build / wasm-commands vs drop-pure-word: drop-pure-word wins; delete the `warp pure` entries.
- P102 / build-three-outputs / build-includes-native: what `warp build` does today wins; Supervisor confirms if unclear.
- Rust-only entries (P7, P8, cargo-fmt, rust-*, P151, cdylib, sound-dsp, cloud-tests, speed-local-builds): delete (Rust is gone).
- release-2-0 vs rust-gone-in-1-5: later wins.
- pipeline-first / core-pipeline-first / self-hosting-back-end: all three superseded (samples-focus, the tests roadmap); delete.
- self-hosting / self-hosting-minimal vs passes-first-principles: passes-first-principles wins (no twins).
- core-hygiene / reshuffle entries vs restructuring-hold: the Supervisor says whether the hold still stands; until then the hold wins.
- library-zero-bake-plan vs the baking entries: zero-bake wins (newer); delete built-in-lib, baked-*, slice-parses-rederive.
- built-in-lib vs runtime-prepended: prebuilt modules beside the compiler win (module-link-hard-error).
- no-lib-reads vs library-on-disk: the Supervisor checks the code; the code's behaviour wins.
- module-link-hard-error vs method-clash-from-source: hard error wins (no fallback exists any more).
- outcome-tests-only family vs never-delete-tests: no conflict: tests pinning intermediates or esoteric syntax may go with
  the user's rule (CLAUDE.md "Outcome tests only"); outcome tests are never deleted or edited without consent.
- cli-words vs `--flags`: cli-words wins; the entries are reworded with words.

Numbers, types
- nat-and-int vs D16 / c-int-names / primitive-int-width / exact-decimals / int-list / type-shows-nat: nat-and-int wins,
  as the code does (`type(5)` is nat, `x: int = 2^70` warns that it wraps); delete "Int is unbounded" wording.
- type-of-small-local: follows nat-and-int.
- cast-checks vs primitive-int-width: both stay: `as int` checks the fit, int arithmetic wraps (with the warning).
- exact-roots vs exact-decimals: exact-roots wins, as the code does (`sqrt(2)` is √2).
- floor-division family: core-int-division wins: `÷` canonical, `/!` an alias.
- hash-comments vs floor-division: no conflict: `#` preferred, `//` still a comment with a hint.
- marks-survive-arithmetic: superseded by cheap-classes-everywhere (plain int; typed arithmetic is card arithmetic-keeps-type).
- marks-unboxed-first: superseded by int-types-are-classes / cheap-classes-everywhere.
- fixed-time-units / one-duration vs time-word / instant-vs-duration / local-now: later wins.
- dates-later vs time-word: later wins.
- unit-after-number / units-opt-in / litre-suffix / expected-type-names: the code wins: `3km` works without `use units`
  (units-back); units-opt-in is reworded as historic.
- empty-is-not-missing vs Truthiness: empty-is-not-missing wins, as the code does (`x=ø; x.size` is 0).
- optional-result vs kind-kept family: later wins.
- or-fallback vs nullish-default: no conflict (`or` replaces falsy, `default` only ø/error); `otherwise` is an alias
  with a note (canonical-output).
- glued-negative / operator-even-spacing vs list-sign: operator-even-spacing wins; delete list-sign.
- path-after-operand vs division: no conflict under operator-even-spacing (`x /y` uneven = a path item, `x / 2` division).
- dot-glyphs vs middot-scalars: no conflict: resolved by operand types.
- named-args-colon vs `f(b = 1)`: the colon wins (named-argument-phrases).

Text, data, markup
- data-names vs data-stays-data: no conflict: `data` parameters splice bound names (data-parameters), data files and
  `data e` keep bare words.
- data-parameter / markup-elements-functions vs data-tag-words: data-tag-words wins (latest).
- markup-auto-render vs web-words; instances-not-plural vs web-outside-state: later wins.
- spaced-tag vs glued-tag-mark: both stay: same meaning, printing keeps the glued form.
- shown-spaced vs separator entries: shown-separator-wins wins.
- format-as-text-class vs show-hook / text-not-str: format-as-text wins (`show` dropped), `text(x)` canonical.
- of-in-selectors / count-x-in-y-held vs drops: later wins.

Functions, binding, classes
- one-method-form vs declared-belonging; P141 vs override-keyword; P84 vs function-words-consume; library-word-visibility
  vs host-callback-exports; enum-case vs kinds-as-cases; declared-class vs D4; foreign lowerings; play-one-word vs
  bizarre-global; shader-hash vs color-show-variants; sound backends; Matrix; no-slow-tests vs slow-lane;
  use-js-tests vs P183: later wins.
- P82 vs words-as-values: open with the user (composition-value-fails, discuss later).
- juxtaposition-one-rule vs P68 / P143: no conflict: one rule by fit; where two fits remain, refused (refuse-never-guess).
- param-as-type vs as-is-convert: no conflict: in a definition head `as T` declares the result, elsewhere it converts.
- go-unified vs captures / inner-assignment-refused: no conflict: `n += 1` in a lambda reads n, so it is the outer one
  (shared-captures); only a plain `n = …` that could be either is refused.
- prefix-symbols-alias-functions vs operators-are-functions: operators-are-functions wins (`√` a prefix operator,
  per-type overloads).
- key-op-spelling vs CLAUDE.md, immutable-tuple vs Round: technical; the code wins, the Supervisor rewords.

Decided by the Interviewer on the user's "resolve" (to present as one keep-or-change batch):
- `===`: the code's meaning wins: same value and same kind of thing (`0 === false` no, `1 === 1.0` yes: both numbers);
  `≡` spells it; P208 identity is deleted (identity, if ever needed, gets its own word).
- range-dots vs to-inclusive / lazy-range-value: the code wins: `a..b` excludes b (`(1..3) + [4]` is [1, 2, 4]),
  `a...b` includes it, for ranges and for `xs#a..b` alike. `xs#2..3` printing nothing is a bug: card it.
- use-require-import vs implicit-standard-words / D15 / packages-by-name: the code wins: standard modules (lib/) load
  on demand by their words, `3km` without `use units`; `use` is needed for non-standard modules (lib/extra, paths,
  packages), and a bare word of such a module is refused with the `use` fix-it.
- def-brace-body vs keywordless-def: refuse-never-guess wins: `g(n) { … }` with g defined is a call with a block; with g
  defined nowhere it is refused, fix `g(n) := { … }` (no definition by absence).
- dollar-interpolation vs P232 / $dir / rule holes: contexts decide, no conflict: `$x` in a text interpolates, `$name`
  in a rewrite rule is a hole, `$dir` is a word; P232's implicit-subject `$` is deleted where it clashes with these.
