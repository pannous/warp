# Brief for footgun inspiration agents

Repo: /Users/me/dev/angles/warp (Rust implementation of the Wasp/Warp language). Read Footguns.md, DESIGN.md, CLAUDE.md first.
Footguns.md lists footguns of other languages and Warp's status (Solved / NOT YET / "Impossible").

Your job, for each footgun heading of your group in Footguns.md:
1. Find programming languages (or formats/tools) that solve it WELL, and how exactly
   (e.g. Scheme/Racket exact rationals, Rust checked/wrapping_* ops, Swift grapheme-cluster String, Kotlin null safety,
   Python chained comparisons, Go 1.22 per-iteration loop vars, TOML 1.0 vs YAML 1.1, Temporal API for JS dates, ...).
2. Give a one-line example in that language showing the right answer.
3. Say which idea Warp should adopt (1-2 sentences), consistent with DESIGN.md.
4. Where cheap, verify a claim by running the language locally (python3, node, rustc, swift, go if installed) in
   probes/footguns/inspiration/scratch/ (never /tmp).

Write ONLY to probes/footguns/inspiration/<your-group>.md, format per entry:

### <exact heading from Footguns.md>
Solved elsewhere: **Language**: `example` → right answer — how (mechanism).  (1-3 languages)
Adopt in Warp: ...

Do NOT edit Footguns.md, source code or tests (the coordinating session merges your notes).
Commit only your own file by explicit path (never git add -A), conventional commit `docs(footguns): inspiration for <group>`,
no AI attribution lines, `git pull --rebase` then `git push`. Other agents work in the same checkout.
