# Reusing a regex engine (card reuse-regex, user policy reuse-giants 2026-10-29)

What we build ourselves today: lib/regex.warp is four words (`matches`, `first_match`, `find_all`, `replace_all`) over
`std_pure("regex", …)`. Natively core/native/driver.warp `regex_answered` answers them with libc's POSIX `regcomp`/
`regexec` (ERE, plus macOS's REG_ENHANCED, which Linux lacks), with our own loops for all matches, group texts and
`$1` replacement; the playground answers with JS RegExp. The two lanes are held "to what both do alike", which is
a promise nothing checks: POSIX ERE has no `\d`, `\w`, lazy `*?` or non-capturing groups on Linux, while JS has all of
them, so the same program matches differently on macOS, Linux and in the browser.

## Verdicts

| Option | Binding | License | Same in browser and native | Notes |
|---|---|---|---|---|
| **Rust `regex` (or `regex-lite`) built to a plain core wasm module** (take) | a wasm module of a few exports, `regex_find(pattern_ptr, pattern_len, subject_ptr, subject_len, from) -> i64` and friends, over linear memory: no component model needed; warp already imports plain wasm modules (`import tests/fixtures/wasm/fourty_two`) | MIT / Apache-2.0 | yes: the very same bytes run in wasmtime and in the browser | linear time (no ReDoS), Unicode classes, `\d \w \s`, lazy quantifiers, named groups; no look-around and no backreferences, which is the honest subset. `regex` is about 600 KB of wasm with its Unicode tables, `regex-lite` about 100 KB (slower, ASCII classes): start with regex-lite, loaded only by programs that `use regex` |
| PCRE2 | C ABI: `pcre2_compile_8`, `pcre2_match_8`, `pcre2_substitute_8` as `from 'pcre2-8'` lines in lib/regex.warp | BSD-3 with exception | no: the browser stays on JS RegExp (close to PCRE2, but not equal) | full Perl syntax with look-around and backreferences, JIT; backtracking, so a hostile pattern can take exponential time. Ships on every Linux and in Homebrew. The fallback if a wasm module beside the program is not wanted natively |
| RE2 | C++; only through a C shim (cre2) or as wasm via emscripten | BSD-3 | only as wasm | the same linear-time subset as Rust regex, but C++ and a heavier wasm build: Rust regex gives the same with less glue |
| Rust `regex` as a WIT component | component model | MIT / Apache-2.0 | yes | the native warp runs modules through wasmtime's C API, whose component support is partial; the plain-module ABI above gives the same without it |

## We stop building
- driver.warp `regex_answered`, `match_from`, `all_matches`, `group_text`, `replaced` and the REG_* constants: about
  80 lines of host code and the POSIX/macOS flag difference.
- The playground's regex host answers: the same module runs there.
- The "held to what both do alike" caveat in lib/regex.warp: there is one engine, so one behavior, documented as Rust
  regex syntax.

## Shape
lib/regex.warp imports the module (`import regex_lite.wasm` beside the library, built once from a tiny Rust crate in
crates/ and published with the binary like runtime.wasm) and copies texts into its memory with the existing
text_address / text_from_memory edge words. A compiled-pattern cache keyed by the pattern text keeps loops cheap.
Open question for core: a plain wasm module imported by a library must reach the playground's loader too (today only
fixtures are imported this way); that is a question for the Interviewer before any work.
