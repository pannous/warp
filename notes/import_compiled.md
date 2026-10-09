# Importing a module warp compiled (card import-compiled, 2026-10-08)

`import "shapes.wasm"` works for a module warp compiled, as for any WebAssembly module, and its functions on values
(texts, lists, instances: Nodes) and its classes are the program's too. Sample: samples/import_compiled.warp, test:
tests/modules/test_import_compiled.rs (the browser too).

## How
- The module's `warp.meta` section (lowering/reflection.rs meta_entries) names each function's parameters and
  signature (`functions{f{params signature}}`) and each class's fields, methods and source (`classes{P{… definition}}`,
  sliced from the code by `class P` up to its closing brace, because a class Node serializes lossily).
- wasm_modules.rs read_exports: a function whose signature crosses Nodes is imported with anyref parameters and
  results, its Export carrying `node_result` (the result kind from `-> type`). analyzer/imports.rs takes that kind.
- At a call (wasm_modules.rs link, call_with_values) each value is copied from one instance to the other:
  wasm_reader::node_in reads it, tasks::TaskValue::of and Builders::of(exports of the target).build make it there.
- Classes: modules.rs insert_module_classes parses each class definition into the program (so `P(1, 2)` and
  `p.sum()` are the program's own) and rewrites `shapes.P(…)` to `P(…)` (wasm_modules::with_bare_classes).
- struct_backend.rs: a function nobody in its own program calls keeps the Node parameter (`p:P` as a P·instance
  struct only when every call passes one, and there is at least one call): an exported function is called by importers.

- The browser (card import-compiled-browser): host-foreign.js moduleImports copies each Node argument into the module's
  instance and its result back (reader.js readNode, host.js buildValue, which reads keys and so instances too). A host
  without GC field access needs the module's reflection getters, so `warp compile --wasm` emits them
  (pipeline::for_any_host, about 700 bytes). The sample imports tests/fixtures/compiled/shapes.wasm, which the deploy
  (pages.yml) ships.

- Shared changes (card import-shared): passing the reference itself does not work. Both modules canonicalize to the same
  $Node type, so the module accepts the program's Node, but a Node's texts, field names among them, are offsets into its
  own instance's linear memory. The module would compare "x" against the program's offsets read in the module's memory,
  not find it, and grow a field whose name points into the module's memory: a corrupt object. So an argument still
  crosses as a copy, and what the module changed in that copy is written back: the program's Node takes the fields of
  a copy of it (wasm_modules.rs write_back natively, host-foreign.js writtenBack with the export reflect_assign in the
  browser), so every holder of the instance or list sees the change. A list passed to such a function stays a Node list,
  not a typed array (list_sharing.rs held_elsewhere).
  Not covered: a reference the module keeps (in a global) and changes later, and nested objects: they are new copies.

## Left
- True sharing needs texts outside linear memory (GC arrays of bytes) or one memory for both instances.
