# Rust crates as C libraries for warp programs (card ffi-rust-crate-adapter, topic ffi-libraries-roadmap)

Goal: any Rust crate usable from warp through warp-host's plain C FFI (notes/warp_host.md "C calls"). No Rust bridge
per library, and nothing about a crate in warp-host. The libraries themselves are pure warp over FFI.

## How it works

    lib/crates/regex.warp ──scripts/warp-crate build──▶ ~/.cache/warp/crates/lib/librs_regex.so   (dlopen'd by warp-host)
                                                      ▶ ~/.cache/warp/crates/native/rs_regex.warp (`use rs_regex`)

- A crate manifest is a lib/native file (a library's foreign lines plus warp words over them, like lib/native/sqlite3.warp).
  Each foreign line also ends with `= <rust item>`:

      crate: "regex@1"                      the dependency, a cargo version requirement
      uses: "sha2::Digest"                  optional: traits the items need in scope
      features: "unicode"                   optional, plus `default_features: no`
      library: "rs_regex"                   optional, default rs_<crate>
      regex_new(pattern: text): handle(regex::Regex) from 'rs_regex' = regex::Regex::new
      regex_find(re: handle, haystack: text): text from 'rs_regex' = regex::Regex::find?.as_str
      regex_free(re: handle(regex::Regex)): void from 'rs_regex' = drop

- `scripts/warp-crate build <manifest>…` generates a cdylib crate: the fixed runtime scripts/crate_adapter/runtime.rs,
  and one `#[no_mangle] extern "C"` wrapper per line, named as the line. It builds it with cargo and caches the result.
  Then it installs the .so and writes the native file: the manifest with the `= rust` parts cut and settings commented
  out. warp-host needs no change: the pure compiler (card pure-c-imports) turns the lines into imports of module
  `rs_regex` with a warp.c_functions section, and warp-host dlopens `librs_regex.so`.
- Other commands:
  - `warp-crate source <manifest>`: the generated Rust
  - `warp-crate scan regex@1`: draft manifest lines for the crate's root functions and its structs' inherent methods,
    read from rustdoc JSON (stable cargo with RUSTC_BOOTSTRAP=1, so no nightly toolchain). A line that doesn't map is
    printed as a `// skipped:` comment.

## Type mapping

| manifest type | C (shim side) | foreign line | Rust side |
|---|---|---|---|
| i32 i64 f32 f64 | same | same | `as _` casts in; any int/float out |
| bool | i32 in, C bool out | bool | `!= 0` |
| text | `const char *` (NUL-terminated copy) | text | `&str` (`string`: `String`) |
| bytes | `const char *` | text | `&[u8]` up to the NUL |
| hex (result) | `const char *` | text | `AsRef<[u8]>` as lowercase hex (digests) |
| list (result) | handle to a text list | handle | any iterator of texts/numbers, collected at once |
| handle(T) | `T *` (Box) | handle | result: T, Option<T> or Result<T, E> boxed; parameter: `&mut T` |
| void | void | void | `()` or `Result<(), E>` |

- The first handle parameter of a method path (`regex::Regex::is_match`) is that type. Elsewhere write `handle(path)`.
- `?.m` maps m over the Option or Result an item returns. `*.m` maps it over an iterator. `.m` calls it.
- Failures:
  - None, an Err, a panic or a NULL handle return the C null: -1 for numbers, NaN for floats, false, NULL for texts
    and handles. A NULL text is ø in warp; a NULL handle is 0.
  - An Err's Display, or the panic message, goes into `<library>_last_error()` (ø after a call that worked).
  - Panics are caught; unwinding into warp-host would abort it.
- Text results live in a ring of 16 per thread inside the shim. warp-host copies each one into a warp text at once.
- Every shim also exports `<library>_last_error`, `<library>_list_count/_list_text/_list_free`.

## Decisions (made on the card, 2026-10-23)

- The generator is a Python script, not Rust in src/ or crates/: those are frozen, and the work is text templating
  plus rustdoc JSON reading. It could become warp once warp has a process and JSON story stable enough.
- Item selection is a manifest by default, not everything rustdoc lists:
  - Most useful APIs need a small chain (`find?.as_str`) or a trait in scope (`Sha256::digest`).
  - A crate's whole surface would pull in generic and borrowed items that can't cross.
  - `scan` drafts the manifest from rustdoc, so starting one is automatic.
- Conversions are Rust traits (`Out<C>`, `HandleOut<T>`) in the fixed runtime. rustc picks the conversion from the
  item's real type (Option, Result, String, &str, Cow, any integer), so the generator never needs that type.
- Library names are `rs_<crate>`. That avoids clashes with system libraries (libregex) and with warp's std modules
  (`use regex` stays lib/regex.warp until it is switched over).
- bytes cross as NUL-terminated texts for now. The pure lane's foreign lines have no ptr+len type, so a byte 0 ends
  the data (a `bytes` foreign type is in Open below).
- No new warp-host primitive was needed or added (shared FFI primitives: none). Library lookup uses pure-c-imports'
  WARP_LIBRARY_PATH (or LD/DYLD_LIBRARY_PATH).

## Status (measured on the box, Linux x86-64)

- regex 1.12, sha2 0.10.9 and crc32fast 1 build into shims:
  - first build ~1-8 min on the loaded box; a manifest change rebuilds in ~9 s (shared target dir)
  - a cache hit takes no build
- Run through warp-host and compiler.wasm from the pure-c-imports branch: sha256 by a foreign line, `use rs_regex`
  is_match / first_match / find_all / replace_all / find_start / free.
- `warp-crate scan semver@1` drafts 19 lines (constructors, parse, VersionReq::matches, as_str, free lines). They
  build unedited, and a warp program then checks version requirements (`versionreq_matches(req, version_parse("1.4.0"))`).
  scan on regex builds too.
- tests/host/test_crates.sh: see the card for counts. It needs a host and compiler with C imports (pure-c-imports).

## Open

- `use crate "regex@1"` in the compiler, building on demand: needs lib/compiler (c_imports.warp, Warp Wrapper's).
  Today a program says `use rs_regex` and the native file must be in `<lib>/native/`. Proposal: c_imports also reads
  `$WARP_CRATES_CACHE/native/` (default ~/.cache/warp/crates/native).
- A `bytes` foreign type (ptr + length) for binary data, and byte results (Vec<u8>) as warp bytes.
- A one-letter text from C prints as a codepoint ('1' in find_all's list): a pure-lane printing quirk, not the shim's.
- bool results print as 1/0 on the pure lane (c_parameter_kinds maps bool to i32).
- Retiring src/modules/std_adapters.rs hash/regex: switch lib/hash.warp and lib/regex.warp to `use rs_sha2` /
  `use rs_regex` once shims ship precompiled (below) and the browser keeps its JS twin.
- Callbacks (closures into Rust), structs by value, generic items: out of scope. Wrap them in a tiny helper crate of
  your own and point a manifest at it (a path dependency is a manifest line away; not added yet).

## Plan: precompiled crates (card precompiled-crates)

Goal: `use rs_regex` works on a fresh install without cargo, so warp gains libraries without a Rust toolchain.
1. Key: crate, resolved version, features, manifest hash, target triple. The toolchain drops out of the key for
   shipped builds: only the C ABI crosses, so any rustc works.
2. CI job (GitHub Actions matrix: linux-x86_64, linux-aarch64, macos-arm64, macos-x86_64; windows later) runs
   `warp-crate build lib/crates/*.warp`. It uploads `lib<name>.{so,dylib}` plus the native file as release assets,
   with a `crates.json5` index of name, key, sha256 and url.
3. `warp-crate fetch <name>` (and warp-host/launcher on a `use rs_x` whose library is missing):
   - download the asset for this target, check its sha256, put it in ~/.cache/warp/crates/lib
   - fall back to a local build when cargo is present, else an error naming the crate and target
4. The packaging tarballs, deb/rpm, wheels and brew formula ship the std shims (rs_regex, rs_sha2, rs_crc32fast,
   later serde_json) in `<prefix>/lib/warp/crates/`. warp-host adds that folder to its library path (one generic
   lookup folder, not per crate).
5. Then retire the std_adapters.rs bridges: lib/regex.warp and lib/hash.warp become `use rs_regex`-style warp over
   FFI, with the browser's host.js twins kept for wasm in the browser.
6. Third-party crates: users write a manifest (`warp-crate scan` drafts it) and build locally. A community index of
   manifests could later get the same CI treatment.
