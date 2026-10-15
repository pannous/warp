# CI (Rust CI, ubuntu-latest)

- FFI signatures come from parsing system headers. glibc's math.h declares libm via
  `__MATHCALL` macros, so on Linux nothing parses → `use m` imported nothing.
  Fix: `LIBM_F64_FUNCTIONS` table in src/ffi/mod.rs is the fallback (same set the linker hardcodes).
- glibc's math.h includes bits/mathcalls.h from the multiarch dir /usr/include/x86_64-linux-gnu (card use-cmath,
  2026-10-09): without it in ffi_parser.rs INCLUDE_DIRS cbrt had no signature, the emitter's m.cbrt need was never
  provided and each rerun asked again, without end (a stack overflow even with 32 MB stacks). A rerun whose need is
  still missing now stops with a type error naming it (wasm_emitter/mod.rs inherited_needs). Reproduce on macOS with
  glibc headers from a libc6-dev .deb: WARP_INCLUDE=<deb>/usr/include[:<deb>/usr/include/x86_64-linux-gnu].
- Law proofs shell out to `lean`; CI installs it via elan (`LEAN_TOOLCHAIN` in rust.yml, keep in sync with local `lean --version`).
- `test_law_overflow_found_by_property_tests` fails by design since unbounded Int (fcbd300b).
  The Lean export still models Int as `BitVec 64`, so Lean still "finds" overflow that runtime no longer has.

# Browser dry run on pannous.com (user 2026-10-15)

Browser-related changes run `scripts/browser-dry-run.sh` green first, then dispatch the Playground workflow once: trial
runs on GitHub mailed "Run failed" each time. GitHub then runs only the final check, and Safari (macOS runner only).
No browser on the Mac, headless included (user 2026-10-09).

- `scripts/browser-dry-run.sh` (whole workflow), `--tours [example…]`, `--chrome […]`, `--firefox […]`,
  `--suite [libtest filter…]`; `--again` before them reruns on the server without building or copying.
- Builds here as pages.yml does: `build.sh` then `build.sh site <dir>` (the one site collection, shared with the
  workflow); the suite's tests.wasm by `cargo build --test tests` for wasm32-wasip1 (only built here, so no turn in tests/queue.sh) plus `build.sh components`.
- rsyncs the checkout's files (tracked and new, as they are), the site, the built components/packages and tests.wasm to
  `/srv/browser-checks/runs/<branch>` (runs older than 2 days are removed), then runs the workflow's commands
  (`test_in_browser.py --examples [--firefox] --site _site`, `test_in_browser.py tests.wasm`) with the workflow's
  AGENT_BROWSER_ARGS and FIREFOX_PREFS (read from pages.yml), CI=1 and the local `gh auth token` as GITHUB_TOKEN (sent
  on stdin). WARP_COMPONENTS_BUILT=1 tells the runner the components are built (the server has no cargo).
- Server setup (2026-10-15, Ubuntu 26.04, 2 cores, 3.7 GB, no swap): system user `browsercheck`, home
  /srv/browser-checks; `npm install -g agent-browser` with prefix ~/npm, `agent-browser install` (Chrome for Testing
  in ~/.agent-browser); Mozilla's Firefox tarball in ~/firefox (Ubuntu's firefox package is a snap, no snapd there);
  apt `--no-install-recommends`: libatk1.0-0t64 libatk-bridge2.0-0t64 libcups2t64 libxkbcommon0 libasound2t64
  libxdamage1 libatspi2.0-0t64 libgbm1 libgtk-3-0t64 libdbus-glib-1-2 libx11-xcb1 libxtst6 mesa-vulkan-drivers
  fonts-liberation fonts-dejavu-core. No chromedriver/geckodriver: agent-browser speaks CDP, firefox_driver.mjs BiDi.
- Protecting the server's sites (warp.pannous.com via ferron, warp-lambda, …): one run at a time
  (flock /srv/browser-checks/run.lock, a second run waits and says so), each check in a systemd scope
  `browser-dry-run-<pid>` with MemoryMax=2500M, no swap, CPUWeight/IOWeight 20, nice 15, ionice idle, 45 min limit;
  the scope is stopped when the run ends; a dropped SSH connection ends the checks once they print (SIGPIPE).
- Pitfall: the browsers start with `env -i`: root's XDG_RUNTIME_DIR made agent-browser fail with "Failed to create
  socket directory: Permission denied".
