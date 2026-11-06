# Importing a module warp compiled

`import shapes` (or `use shapes`) beside a shapes.wasm that warp built links it as a standard module is linked
(decision samples-modules-wasm). Sample: samples/wasm/import_compiled.warp, its module test/fixtures/compiled/shapes.warp.

## Build
`warp build --module shapes.warp` writes shapes.wasm: the file compiled as lib/'s modules are (compiler.warp
write_module), behind runtime.wasm, its words exported by name and its `warp.signature` section carrying their parses,
classes and globals. It is built behind one runtime.wasm and links only behind that one: scripts/samples.sh and
web/playground/build.sh build the fixture anew (it is git-ignored; samples/wasm/shapes.wasm links to it).

## Link
- compile() registers each used name with a `<name>.wasm` carrying a warp.signature beside the program
  (own_module_beside → uses.warp own_module): std_module_path then names its text beside it, and linked_libraries
  reads its wasm from there. Without its text beside it the signature is trusted (built_of_text).
- The program imports its words under the module's path (separate_module.warp module_import_name), which the native
  driver and the page (host-foreign.js moduleImports) instantiate.
- A word the program calls by a variant of its name (arity_overloads: a class's method sum() beside the prelude's
  sum(xs) are sum·1·any and sum·1·list_or_number) is bound to the module word it was written as
  (prebuilt.warp bind_written_variants).

## Left
- With shapes.warp itself beside the program, `import shapes` compiles the text in instead of linking.
