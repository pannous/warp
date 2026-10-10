# Code cleanup (user card `cleanup`: "major code cleanup and simplification")

Behaviour-preserving only: shared helpers for duplication, dead code and stale comments out, flatter code, no test edits.

## cleanup-host (warp-hosting): top-level src/*.rs, src/ffi/, src/gc_traits/
How dead code was found: every `fn` name in the area counted across src, tests, crates and web
(`grep -rwo`); a name seen only at its definition has no caller. `#[test]` functions inside src files and trait
methods (wasm-encoder `Reencode`'s `parse_*_section`, winit's `resumed` / `window_event`) are no dead code.
Exact copies: `npx jscpd --min-lines 4 --min-tokens 40 --format rust` over a copy of the area (it skips ignored
folders, so run it inside the copy); exact duplication is only ~0.1% here, the rest is duplication in meaning.

Done:
- never-called functions removed (function.rs builder leftovers, wasm_reader's unused readers and FromVal trait,
  TypeDef::from_node with its second extract_fields, ffi_parser get_signature / get_library_signatures, …)
- ffi: the libc overrides are one table (LIBC_SIGNATURES)
- `Op::holds(ordering)` replaces the comparison tables of real.rs and time.rs
- GcString's two array readers share array_text; wasm_name_resolver drops the type indices and `shared` flag it
  never read
- main.rs: one `fail(message)` for the CLI's 16 "say it and exit 1" pairs; host.rs: the never-imported
  host.get_result_ptr / get_result_len linker entries removed
- `Node::key(name, value)` replaces hand-built `Node::Key(Box::new(Node::Symbol(…)), Op::Colon, …)` (foreign,
  headless, host, meta_section, shader_holes, web_server)
- site.rs: read_program / program_stem shared by deploy, dev_server and host's served_site; page_html_text for the
  "gave no text" check; items_of reused

Left (bigger, needs care):
- Two C header parsers: ffi_parser.rs (`parse_declaration`, one line at a time, C types → Kind → ValType via
  kind_to_valtype) and ffi/header.rs (`extract_function_signature`, multi-line declarations, C types → ValType via
  map_c_type_to_valtype, pointer classes). get_ffi_signatures uses the first, get_signatures_from_headers the second,
  and get_ffi_signature mixes both. One parser would change some mappings (Kind::Pointer is i64 there, a pointer is
  i32 here), so it needs its own card and tests of both tables first. tests/ffi pins ffi_parser's public API.
- real.rs and time.rs are two small constant evaluators with their own Value types and the same Assign/Define arm;
  sharing a scope/evaluator skeleton would remove more, but they differ in errors (Stop vs String).
- src/extensions.rs is hard-linked with ~/dev/script/rust/extensions.rs (src/extensions.rs.hardlink is the
  hardlink-guard's marker): edit it only knowing it changes that file too.
- host.rs (~170 `#[cfg(feature = "native")]`-gated items) would read better split into host/native.rs, but it is the
  hottest file of the repo (dozens of commits a day): split it in a quiet hour, as its own branch, nothing else in it.
- Two GcObject types: wasm_reader::GcObject (node/mod.rs, re-exported by lib.rs) and gc_traits::GcObject (tests'
  ergonomic reader). One of them could wrap the other; the public API of both is pinned by tests/wasm.
- headless.rs node_of_json and foreign.rs node_of both turn JSON into Nodes; they differ in separator (None vs Space),
  arrays (ø vs list) and a non-integer number's fallback, so one replacing the other changes output.
- markup.rs escapes `<pre>` text without `>`, site.rs `escaped` with it: one escaper would change the error page.
