# Samples (samples/)

Every `samples/<chapter>/*.warp` runs, ideally by fixing the language rather than the sample. scripts/samples.sh runs
every sample natively (test/samples.txt); the playground's sweep (web/playground/test_in_browser.py) runs every sample
but those whose `# host: native` (or `none`) line says why they don't run in a browser; `# host: page`
runs in the page only, `# host: none` in neither sweep. A program that needs an outside server is no sample but its
library's example (lib/extra/examples/netbase.warp).

## How to sweep
- `scripts/samples.sh [<tree>]`: each sample under --no-window with a CPU budget; window-library samples (raylib,
  SDL, glfw) are not run, server samples count as served, a sample with a `# expected failure: <why>` line is EXP.
- Output filters: `grep -v '^      '` (to hide hint continuation lines) also hides indented program output.

## Folders
- samples/ has one folder per chapter of the playground guide, the groups of its example menu (web/playground/guide.js
  groupExamplesByChapter): hello numbers text signals conditions matching lists objects loops functions types storage
  errors modules events tasks web servers graphics gpu (Shaders) acceleration sound wasm laws advanced; plus system (crontab, unix_commands:
  native-only processes), golf/, wit/ and native/ (natively built only). A sample sits in the folder of the first
  chapter whose `Examples: …; samples:` line names it; samples no chapter can name (raylib, SDL, errors…) sit by topic.
- A new sample: its chapter's folder + its name in that chapter's line.
- Put a focused example in the earliest chapter whose concept it demonstrates: `simple.warp` is basic arithmetic in
  Numbers, while Advanced is reserved for programs that combine several chapters.
- The menu and the address (#fizzbuzz) name a sample by its file name, so names stay unique across folders
  (build.sh refuses twins); samples.js has SAMPLE_FILES name → path for the runs' neighbour files (play ./tune.mp3).
