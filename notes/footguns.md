# Footguns catalogue — how it is maintained

- The file is `Footguns.md` (capital F). The checkout is case-insensitive (`core.ignorecase=true`):
  `git add footguns.md` silently stages nothing for the untracked file; use the real name.
- `probes/footguns/footguns.sh` rebuilds warp and evaluates every case in `probes/footguns/cases.warp`
  (cases separated by `---` lines) into `probes/footguns/results.txt`. Rerun it after compiler changes and diff.
- `tests/probe_footguns.rs`: passing tests = Solved entries; `#[ignore = "next"]` = NOT YET entries with a clear answer.
  When a NOT YET bug gets fixed, un-ignore its test and move the entry to Solved.
- `warp eval` and `is!` use the same `wasm_emitter::eval`, so CLI output is representative.

## Plain bugs found while probing (2026-09-26), good agent tasks
- `1/4+1/4` → 0: comparisons see 0.5, the returned sum is typed Int and truncated.
- `"abc"=="abc"`, `0==""`, `null==false`, `if "" …`, NFC vs NFD compare → compiler panic `Cannot extract numeric value`.
- `x=1;x++;x` → 1 (increment lost); `++i` parse error.
- `f := it*10; 1 + f 3` → 3 (should be 31).
- `100000000000000000000` → string of NUL bytes.
- `1e3`, `1_000_000` parse as two-element lists.
- `x=[1 2 3]; x[3]` returns unevaluated program text; `x#0`, `x[-1]` → 1.
- `'héllo'#2` → 'Ã' (byte index despite wiki promising char-safe `#`).
- strings mutate through aliases: `x="ab";y=x;y#1="z";x` → 'zb'.
- `country: NO` → `country:0` (YAML Norway problem) — needs a design decision on `yes`/`no` aliases.
