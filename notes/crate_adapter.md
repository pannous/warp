# Rust crates as C libraries for warp programs (card ffi-rust-crate-adapter, topic ffi-libraries-roadmap)

Goal: any Rust crate usable from warp through the native warp's plain C FFI (core/native/driver.warp). No Rust bridge
per library, and nothing about a crate in the native warp. The libraries themselves are pure warp over FFI.

## How it works

    lib/crates/regex.warp ──scripts/warp-crate build──▶ ~/.cache/warp/crates/lib/librs_regex.so   (dlopen'd by the native warp)
                                                      ▶ ~/.cache/warp/crates/native/rs_regex.warp (`use crate "regex@1"`)

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
  Then it installs the .so into `~/.cache/warp/crates/lib`. It writes the native file to
  `~/.cache/warp/crates/native`: the manifest with the `= rust` parts cut and settings commented out.
- Where the two folders are found:
  - the native warp (lib/native/foreign.c LIBRARY_PATHS) does not search the cache's lib/: the library is found by
    LD_LIBRARY_PATH / DYLD_LIBRARY_PATH
  - extensions/c_imports/imports.warp reads `<lib>/native/<name>.warp`, else the cache's native/<name>.warp
  - `use crate "regex@1"` is `use rs_regex`
  - The compiler turns the lines into imports of module `rs_regex` with a warp.c_functions section, and the native
    warp dlopens `librs_regex.so`. Nothing about a crate is in the native warp.
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
| bytes (parameter) | `const uint8_t *, size_t` | `bytes, <name>_size: length` | `&[u8]` (`bytes_vec`: `Vec<u8>`): a text's bytes or a list of 0 to 255, zero bytes too |
| bytes (result) | handle to a byte buffer | handle + warp word | `AsRef<[u8]>` (Vec<u8>, a digest), a warp list of 0 to 255 |
| hex (result) | `const char *` | text | `AsRef<[u8]>` as lowercase hex (digests) |
| list (result) | handle to a text list | handle + warp word | any iterator of texts/numbers, collected at once; a warp list |
| handle(T) | `T *` (Box) | handle | result: T, Option<T> or Result<T, E> boxed; parameter: `&mut T` (`&T` too) |
| clone(T) (parameter) | `T *` | handle | `T` by value: a clone of the handle's value (T: Clone), the handle stays the caller's |
| void | void | void | `()` or `Result<(), E>` |

- The first handle parameter of a method path (`regex::Regex::is_match`) is that type. Elsewhere write `handle(path)`.
- `?.m` maps m over the Option or Result an item returns. `*.m` maps it over an iterator. `.m` calls it.
- Config structs (card use-cargo-config-structs): `= crate::Config::default` makes one (a path call like any other);
  `= crate::Config.width` reads a public field (cloned: a struct field comes back as a handle of its own), and
  `= crate::Config.width=` sets it from the line's second parameter, a number cast to the field's number type, a text
  as String or PathBuf, each also into an Option field (runtime.rs FieldIn), a `clone(T)` by `.into()`:

      versionreq_default(): handle(semver::VersionReq) from 'rs_semver' = semver::VersionReq::default
      config_set_width(config: handle, width: i64): void from 'rs_x' = x::Config.width=
      version_pre(version: handle): handle(semver::Prerelease) from 'rs_semver' = semver::Version.pre

  `scan` drafts these: a `_default` line for a struct implementing Default, a getter and a `_set_` line for each public
  field, `clone(T)` for a struct taken by value (a builder's `self` too).
- Failures:
  - None, an Err, a panic or a NULL handle return the C null: -1 for numbers, NaN for floats, false, NULL for texts
    and handles. A NULL text is ø in warp; a NULL handle is 0.
  - An Err's Display, or the panic message, goes into `<library>_last_error()` (ø after a call that worked).
  - Panics are caught; unwinding into the native warp would abort it.
- Text results live in a ring of 16 per thread inside the shim. The native warp copies each one into a warp text at once.
- How a list or bytes result comes back:
  - The C function is `<name>_handle`. The native file adds a warp word `<name>` that reads the handle into a warp list
    and frees it, through `<library>_take_texts` / `_take_bytes`.
  - The host has no byte or list result type, so this goes through generic shim accessors, one call per item:
    `<library>_list_count/_list_text/_list_free` and `<library>_bytes_count/_bytes_at/_bytes_free`.
  - Every shim exports these accessors, plus `<library>_last_error`.

## Decisions

- The generator is a Python script: the work is text templating plus rustdoc JSON reading. It could become warp once warp has a process and JSON story stable enough.
- Item selection is a manifest by default, not everything rustdoc lists:
  - Most useful APIs need a small chain (`find?.as_str`) or a trait in scope (`Sha256::digest`).
  - A crate's whole surface would pull in generic and borrowed items that can't cross.
  - `scan` drafts the manifest from rustdoc, so starting one is automatic.
- Conversions are Rust traits (`Out<C>`, `HandleOut<T>`) in the fixed runtime. rustc picks the conversion from the
  item's real type (Option, Result, String, &str, Cow, any integer), so the generator never needs that type.
- Library names are `rs_<crate>`. That avoids clashes with system libraries (libregex) and with warp's std modules
  (`use regex` stays lib/regex.warp until it is switched over).
- bytes parameters use the `bytes` foreign type: a pointer, plus the `length` the host fills.
- rs_* names carry no version (`librs_regex.so`), so no versioned soname search is needed.

## Open

- `use crate "regex@1"` doesn't build on demand: the shim must be built (`warp-crate build`) or fetched
  (`warp-crate fetch regex`) first. The version after @ isn't checked against the built one.
- Byte and list results cost one FFI call per item. A host result type for bytes (pointer + length out) would make
  them one call.
- A one-letter text from C prints as a codepoint ('1' in find_all's list): a pure-lane printing quirk, not the shim's.
- bool results print as 1/0 on the pure lane (c_parameter_kinds maps bool to i32).
- Switching lib/hash.warp and lib/regex.warp (today std_pure, answered by core/native/driver.warp) to `use rs_sha2` /
  `use rs_regex` once shims ship precompiled (below) and the browser keeps its JS twin.
- Callbacks (closures into Rust), generic items: out of scope (structs by value: `clone(T)`). Wrap them in a tiny helper crate of
  your own and point a manifest at it (a path dependency is a manifest line away; not added yet).

## Plan: precompiled crates (card precompiled-crates)

Goal: `use rs_regex` works on a fresh install without cargo, so warp gains libraries without a Rust toolchain.
1. Key: crate, resolved version, features, manifest hash, target triple. The toolchain drops out of the key for
   shipped builds: only the C ABI crosses, so any rustc works.
2. CI job (GitHub Actions matrix: linux-x86_64, linux-aarch64, macos-arm64, macos-x86_64; windows later) runs
   `warp-crate build lib/crates/*.warp`. It uploads `lib<name>.{so,dylib}` plus the native file as release assets,
   with a `crates.json5` index of name, key, sha256 and url.
3. `warp-crate fetch <name>` (and the native warp on a `use rs_x` whose library is missing):
   - download the asset for this target, check its sha256, put it in ~/.cache/warp/crates/lib
   - fall back to a local build when cargo is present, else an error naming the crate and target
4. The packaging tarballs, deb/rpm, wheels and brew formula ship the std shims (rs_regex, rs_sha2, rs_crc32fast,
   later serde_json) in `<prefix>/lib/warp/crates/`. The native warp adds that folder to its library path (one generic
   lookup folder, not per crate).
5. Then retire the std_pure bridges: lib/regex.warp and lib/hash.warp become `use rs_regex`-style warp over
   FFI, with the browser's host.js twins kept for wasm in the browser.
6. Third-party crates: users write a manifest (`warp-crate scan` drafts it) and build locally. A community index of
   manifests could later get the same CI treatment.

Status: steps 1-3 are in. `warp-crate publish <dir> <manifests>` builds per target (library and native
file named `<library>-<target>`, entries in `crates-<target>.json`: name, library, crate, resolved version, key, target,
file, sha256, native, native_sha256); `warp-crate index <dir>` merges them into crates.json (JSON, not json5: python's
own reader); .github/workflows/crates.yml runs both over the four targets and replaces the assets of the pre-release
`crates` when lib/crates or the generator changes on main. `warp-crate fetch <name>` downloads this target's entry whose
key is the local manifest's (WARP_CRATES_URL names another place, also file://), checks both sha256, installs as
`build` does, and builds with cargo where nothing matching is shipped; without cargo it names the crate, the target and
why. Open: the native warp fetching on a `use rs_x` itself (step 3's launcher half), packaging (step 4), retiring the
bridges (step 5); and the native warp's FFI still refuses the shims' handle words (`sha256_bytes_handle`), shipped or
built.
