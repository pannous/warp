# Emitter instruction helpers

Repeated instruction runs belong beside `emit_list` in `wasm_emitter/function_builder.rs` when they express one
runtime operation. `emit_new_text` constructs a text from pointer and length locals; `emit_new_text_to` constructs one
from a buffer start through its current cursor.

Callers keep unusual pointer or length arithmetic explicit, such as a substring offset or a duration that excludes its
last separator. Closure entry-field loads similarly go through `emit_closure_entry`, so the closure layout field is
named in one place while casts and calls remain visible at each use.

Check changes here with focused text and closure tests plus clippy and the no-default-features wasm32 library check.
