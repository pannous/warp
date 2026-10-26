# Supervisor handover (2026-10-26, evening)

## Direction from the user
"The main part of the migration to the pure WASM parser is done. We may focus a bit on performance before then
resuming implementing features. Actually, this is a good time to do some quality reassessment and beautify the
compiler by adding more types, classes, enums, and so on." And: "As always, he has to delegate these tasks."

So, in this order:
1. Performance: rebuild speed (card rebuild-time-speedup, warp-fixer) continues.
2. Quality reassessment + beautifying the compiler: core/*.warp gets more types, classes and enums (e.g. the Kind/
   Node tags, pass results, the analyzer's tables, refusal kinds as enums instead of text/number codes; classes for
   the parser state, scopes, emitter context). Delegate: one refactorer-style session per area (parser, passes /
   analyzer, emitter), at most two at once. Each change keeps the compiler compiling itself (self test) and hello.
   Core changes over ~40 lines need the user's OK via warp-interviewer: send a short plan per area first.
3. Then features again (the Sample gaps topic: notes/sample_status.md, 49/224 samples run; 26 sub-cards).

## State
- Milestone commit be4efc400b "victory: the first clean pure warp compiler". No release until the sample gap is
  smaller (user).
- Rebuild (`warp rebuild pure`): 1044 s → 208 s today (starts_at fix, hashed map index, refreshed seeds). The fixer
  is on refresh 4 (LRU slots). Parallel compilation: quick answer given (wasm GC has no shared threads; separate
  per-module units + caching first); a proper investigation only after the next profile.
- Running sessions: warp-integrator (watches main: after-push.sh installs compilers within seconds, cron every 2 min,
  installs a changed harness itself), warp-interviewer, warp-reviewer (reviews merges after they land), warp-fixer.
- warp-fixer's queue, in order: refresh 4 → hDucUM / refused-program-diagnostic (every refused program traps the
  compiler at compiler.warp:118 instead of printing "outside the core: …"; urgent, user-facing) → file-words-implicit
  (user decision: lib/file.warp words without `use file`) → built-compiler (lib/ bundled, on-demand modules,
  WARP_LIB overrides per module; decision built-in-lib) → more rebuild hotspots.

## Rules decided today (all in notes/decisions.md and the role files)
quick-merges, self-landing (workers push their own smoke-tested merges; no Ready column any more), no-slots,
no-fixpoint, built-in-lib, root-files, n-times-prelude-now, file-words-implicit; probes/ and scratch/ are gone
(tests in test/, output in a checkout's data/, kept logs in ~/Library/Logs/warp/); ~/dev/bin keeps only the
installed main-builds/<hash>, never warp-<hash> links.

## Open
- /Volumes/Worktrees/fixpoint-stages and /Volumes/Worktrees/rb-logs: stale plain folders (stage wasm and logs of
  10:00-16:46); a safety check blocks agents from removing top-level folders there, so the user removes them.
- /Volumes/Worktrees/rts-main: the fixer's "before" tree for timings; asked to remove it when done.
- Samples list predates today's fixes: rerun the sweep once the refusal trap is fixed.
