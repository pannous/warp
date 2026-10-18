# Samples (samples/)

Every `samples/<chapter>/*.warp` runs, ideally by fixing the language rather than the sample. Fixed samples get an
`is!("samples/<chapter>/x.warp", …)` line in tests/programs/test_samples.rs; tests/programs/test_playground_samples.rs
runs every sample not listed in web/playground/excluded_samples.txt, which gives each exclusion its reason.

## How to sweep
- Build a private CLI copy: `WARP=$(scripts/own-warp.sh)` then run `"$WARP" …`. The shared `debug/warp` is
  overwritten by other worktrees' builds between your build and your run.
- Skip `raylib_*` / `sdl_*` in sweeps: they open real windows.
- `test.wasm` in the cwd is the last emitted module, written before validation: `wasm-tools print test.wasm` shows the
  function behind an "internal error: WASM validation failed".
- The exit code is the program's Int value, and 1 after an uncaught error, so a sweep for exit code 1 also finds
  samples whose value is 1: check for `Error(` in the output.
- Output filters: `grep -v '^      '` (to hide hint continuation lines) also hides indented program output.
- `samples/loops/life_kotlin_ranges.warp` and `errors` fail on purpose (tests/welcoming/test_welcoming_ask.rs pins the
  explanation).

## Warp habits old samples get wrong (sample-side fixes)
- Lists are shared (P200b): a function changes the list it is passed, as in Python; `+` and `copy()` make new lists.
- A function sees main-level constants; to change (or read a computed) main-level value, declare it `global`.
- `/` is exact division (`7/3` stays a fraction); `/!` is floor division (`7 /! 2`); `//` is always a comment.
- `1e10` is an exact number and cannot later hold a float: `1e10 as float`.
- One-character texts are characters: `'5' as int` is 53; `c as text as int` is the digit.
- Types are declared `type V { x: float }`; construct with `V(1, 2)`, `V{x: 1}` or `V { x: 1 }`.
- `tau`/`τ`, `pi` are constants: a variable cannot be named so.
- `real` is the exact real type; use `float` for IEEE arithmetic.
- No keyboard/mouse input, no `pop`, no `Array2D`: interactive games become self-playing demos.

## Folders
- samples/ has one folder per chapter of the playground guide, the groups of its example menu (web/playground/guide.js
  groupExamplesByChapter): hello numbers text signals conditions matching lists objects loops functions types storage
  errors modules events tasks web servers graphics gpu sound wasm laws advanced; plus system (crontab, unix_commands:
  native-only processes), golf/ and wit/. A sample sits in the folder of the first chapter whose `Examples: …;
  samples:` line names it; samples no chapter can name (raylib, SDL, errors…) sit by topic.
- tests/web/test_guide.rs every_chapter_has_its_own_sample_folder keeps the two in step: a chapter's samples in one
  folder, no folder shared by two chapters. A new sample: its chapter's folder + its name in that chapter's line.
- The menu and the address (#fizzbuzz) name a sample by its file name, so names stay unique across folders
  (build.sh refuses twins); samples.js has SAMPLE_FILES name → path for the runs' neighbour files (play "mozart.mp3").
- Tests find a sample by name with tests/common sample_path / sample_files (never format!("samples/{name}.warp")).
- `use lib`, `use square` and `import shapes` find the module samples through SEARCH_DIRECTORIES "samples/modules"
  and "samples/wasm" (src/modules.rs). A sample naming a repository file climbs two folders (`../../tests/fixtures`).
- probes/samples_paths.py [checkout] rewrites flat paths (samples/x.warp) of a branch or the wiki to the folders;
  `--move x.warp <folder>` first moves a flat sample a merge brought in.
