# Ignored tests (sweep 3, 2026-09-30)

Un-ignored by sweeps 2 and 3: test_types, the twelve test_ffi libc/libm tests (incl. test_ffi_math_pipeline and test_dynlib_import_emit), test_is, test_object_properties_wasm,
test_wasm_logic_unary, test_wasm_logic_unary_variables, test_wasm_logic_negated, test_for_loop_classic.
Sweep 4 (2026-10-06, bare `#[ignore]` only): test_assert and test_array_indices_wasm (test_wasm's `assert_throws` was a
`todo!()` stub; `k#i=4` of an undefined k is now an error), test_function_params (expects Int 9, not text "9").
The other 21 bare ones need features, network, host imports or a decision (listed below or deliberate).

Legend: (a) a small no-decision fix exists or is pending; (b) needs a user decision (named); (c) the test expects what the Decided rules exclude.

## (b) needs a user decision: is a list containing only ø falsy: 1

- test_wasm.rs::test_wasm_logic_on_objects — `not {a:2}`, `not {}`, `not []` work since zero-fill; only the last line `not ({[ø]})` → true fails: a list holding only ø is truthy under is_truthy (the test itself says "might skip")

## (b) needs a user decision: C-style and Go-style typed function declarations (`float addi(int x,int y){…}`, `func add1(x int) int {…}`): 6

- test_types.rs::test_go_types
- test_types.rs::test_return_types
- test_types.rs::test_function_argument_cast
- test_wasm.rs::test_sinus2
- test_wasm.rs::test_sinus
- test_wasm.rs::test_all_wasm

## (b) needs a user decision: D4 `person{…}` vs `person:{…}` (grouping of a name and a block): 2

- test_functions.rs::test_stacked_lambdas
- test_wasm.rs::test_node_data_binary_reconstruction

## (b) needs a user decision: D5 implicit parameter `it` in `f x := it+1`, `fac := it<=0 …`: 4

- test_wasm.rs::test_get_local
- test_wasm.rs::test_wasm_function_definiton
- test_wasm.rs::test_wasm_ternary
- test_wasm.rs::test_lazy_evaluation

## (b) needs a user decision: WIT function declarations and polish-notation .wat/.wast (decision 10): 3

- test_functions.rs::test_wit_function
- test_wast.rs::test_parse
- test_wast.rs::test_wast

## (b) needs a user decision: `import int k` mutable global imports: 1

- test_wasm.rs::test_wasm_mutable_global_imports

## (b) needs a user decision: custom operator definitions and the ⌞ logarithm operator: 2

- test_wasm.rs::test_custom_operators
- test_wasm.rs::test_logarithm2

## (b) needs a user decision: does Data(Vec<i32>) equal a list of ints: 1

- test_operators.rs::test_add_pure_vec_data

## (b) needs a user decision: empty list / ø counting properties (test_footguns.rs:634 pins `x=ø; x.size` as an error): 1

- test_empty_list_count.rs::the_counting_properties_of_an_empty_list_need_no_null_check

## (b) needs a user decision: equality of a range with a value ± tolerance (`1900 - 2000 AD == 1950 AD ± 50`): 1

- test_math.rs::test_hyphen_units

## (b) needs a user decision: libc functions (strlen) without an import: 1

- test_wasm.rs::test_wasm_runtime_extension

## (b) needs a user decision: modifier words before `fun` and what a definition evaluates to: 1

- test_functions.rs::test_modifiers

## (b) needs a user decision: overloading by parameter type (polymorphic dispatch): 1

- test_types.rs::test_polymorphism3

## (b) needs a user decision: program arguments (`#params`): 1

- test_wasm.rs::test_arguments

## (b) needs a user decision: tuple comparison with a cast (`(2 as float, 4.3 as int) == 2.0, 4`): 1

- test_types.rs::test_emit_cast_tuple

## (b) needs a user decision: typed array `x : 100 * int` (only `x : 100 int` and `x:int[100]` are decided): 1

- test_lists.rs::test_array_initialization

## (b) needs a user decision: web host (decision 13): $b.ok, externref, DOM: 3

- test_wasm.rs::test_dom_property
- test_web.rs::test_inner_html
- test_web.rs::test_dom_property

## (b) needs a user decision: wisp `def` dialect: 1

- src/wisp_parser.rs::test_wisp_defn

## (c) test defect: expects what the Decided rules exclude: 22

- test_angle.rs::test_function_params — expects the text "9" for a number
- test_lists.rs::test_array_operations — expects `pixel + 4` to be a list (decided: list + int is an error)
- test_lists.rs::test_array_creation — assigns past the end of a list (decided: an error)
- test_lists.rs::test_while_nop_issue — expects a while loop to be worth its counter (decided: the last body value)
- test_lists.rs::test_array_initialization_basics — expects analyze() to expand the typed array (lowering happens at emission)
- test_math.rs::test_primitive_types — expects exact decimals to truncate to 0
- test_node.rs::test_replace — never performs the replace it asserts (the call is commented out)
- test_string.rs::test_string_operations — expects `'say ' + 0.` to convert (decided: no implicit conversion)
- test_wasm.rs::test_wasm_function_calls — needs the C++ test helper `id`
- test_wasm.rs::test_math_primitives — expects `-42.1` to be 42.1
- test_wasm.rs::test_math_operators — exact float compare of an exact decimal (4.00001)
- test_wasm.rs::test_math_operators_runtime — exact float compare of `√3^2`
- test_wasm.rs::test_comparison_id — needs the C++ test helper `id`
- test_wasm.rs::test_comparison_id_precedence — needs the C++ test helper `id`
- test_wasm.rs::test_wasm_logic_primitives — expects `null` to be 0 (null is ø)
- test_wasm.rs::test_wasm_variables0 — expects `i=123.4;i` to be 123
- test_wasm.rs::test_squares — needs the C++ test helper `square`
- test_wasm.rs::test_wasm_stuff — needs the C++ test helper `id`
- test_wasm.rs::test_recent_random_bugs — needs the C++ test helper `id`
- test_wasm.rs::test_import_wasm — needs fourty_two.wasm, which is lost
- test_wasm.rs::test_for_loops — print inside a loop only accepts literals; expected values are C++ cheats
- test_wasm.rs::test_assert — body is todo!()

## host, library, network, filesystem scan or lost files (keep ignored): 16

- test_ffi.rs::test_ffi_sdl_init — requires SDL2 library and warp files
- test_ffi.rs::test_ffi_sdl_window — requires SDL2 library and FFI signatures
- test_ffi.rs::test_ffi_sdl_version — requires SDL2 library and FFI signatures
- test_ffi.rs::test_ffi_sdl_combined — requires SDL2 library and FFI signatures
- test_ffi.rs::test_ffi_sdl_debug — requires SDL2 library and FFI signatures
- test_ffi.rs::test_ffi_sdl_red_square_demo — requires SDL2 library and display
- test_ffi.rs::test_ffi_raylib_combined — requires raylib library and FFI signatures
- test_todo.rs::test_polymorphic_dispatch — requires polymorphic function dispatch
- test_wasm.rs::test_host_download
- test_wasm.rs::test_get_element_by_id
- test_wasm.rs::test_canvas
- test_wasm.rs::test_host_integration
- test_web.rs::test_js
- test_web.rs::test_canvas
- test_web.rs::test_dom
- test_xml.rs::test_parse_all_xml_files — Run with: cargo test -- --ignored

## passes but asserts nothing (keep ignored): 12

- test_ffi.rs::test_extract_function_signature
- test_types.rs::test_typed_functions — TODO: requires complete type system and Signature implementation
- test_types.rs::test_empty_typed_functions — TODO: requires complete type system
- test_types.rs::test_polymorphism — TODO: requires complete type system
- test_types.rs::test_polymorphism2 — TODO: requires complete type system
- test_types.rs::test_generics — TODO: requires Generics implementation
- test_wasm.rs::test_merge_global — LOST files: main_global.wasm, lib_global.wasm
- test_wasm.rs::test_merge_memory — LOST files: main_memory.wasm, lib_memory.wasm
- test_wasm.rs::test_merge_runtime — LOST file: main_memory.wasm
- test_wasm.rs::test_merge_own — Types Module, Code, int not defined
- test_wasm.rs::test_merge_wabt_by_hand — WABT_MERGE types not defined
- test_wasm.rs::test_multi_value — Node constructor syntax not valid in Rust

## also (c) test defects

- test_todo.rs::test_array_type_generics — expects the text "list<int>" (decided output: `list of int`)
- test_todo.rs::test_array_constructor — expects `size(pixels)` to count bytes (decided: size = count). (The `pixels=640000*int` stack overflow is fixed: zero_fill runtime loop.)
- test_wasm.rs::test_array_indices_wasm — expects `puts('ok')` to return 0

(`test_node.rs::test_mark_as_map` is not ignored; the `#[ignore]` the listing script saw sits in a block comment.)
