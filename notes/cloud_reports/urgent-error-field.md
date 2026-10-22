done
Card error-field: `class Point { z: int y: int }` then `Point{z: 1 y: 2}.x` gave `Error("no field x")` without a position.

Cause: a runtime error takes its position from the `trap_position` global, which only the statement loop
(emit_statement_sequence) set. A program of one expression, or of one statement next to a class
(emit_without_type_definitions' `[only]` branch), never set it.

Changed:
- src/wasm_emitter/mod.rs emit_node_main: sets the root's position before emitting it
- src/wasm_emitter/lists/list_emitter.rs emit_without_type_definitions: the lone statement sets its own position

Now: `Error("no field x at <file>:2:1")` with the source line and caret, in the CLI and (shared trap_error) the browser.
Showing that position in the playground UI is the other card the user mentioned, not done here.

Test added (not run, express mode): tests/types/test_records_classes.rs a_missing_field_next_to_a_class_names_its_position

try: printf 'class Point { z: int y: int }\nPoint{z: 1 y: 2}.x\n' > p.warp && warp p.warp
