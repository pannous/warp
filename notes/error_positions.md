# Where errors name their source position

- **Compile time**: `Diagnostic::at(node, …)` renders ` at line:col`; emitter errors use the statement being emitted
  (`WasmGcEmitter::located` → `diagnostic::located_at`, which keeps a position already named and goes before `; fix:`).
- **Parser errors** (`Unexpected character '¶'`) are bare `Error` nodes inside the position `Meta` the parser wraps
  them in. The pipeline reports them via `diagnostic::first_located_error`, which takes the nearest enclosing
  position; `Node::first_error` strips it and is for "is there an error" checks.
- **Run time** (card runtime-parser): every statement of a statement sequence (`emit_statement_sequence`) stores its
  position, `line << 12 | column`, in the exported i32 global `trap_position`. A user function call stores the caller's
  statement again after it returns, because the callee's statements overwrote it. After a trap the runner reads it:
  `wasm_reader::with_trap_detail` natively, host.js `failed` → `"position"` → web.rs `run_outcome` in the playground.
  It reaches the trace as `trap position: <n>`, and `trap_error` appends ` at line:col`.
  - Statement granularity: a failure in a nested block's later part names the last statement that started.
  - Module code stores none, because the position would be read as the program's.
  - Not wired yet: served routes (`host/native.rs route_failure`) and task workers (host-tasks.js).
