# Self-hosting: warp's compiler written in warp

## Value layout
Typed values inside, a boxed $Node only at the edges. Where emitter.warp knows a value's kind it stays plain:

| kind | wasm value | |
|---|---|---|
| int | i64 | |
| float | f64 | |
| yes/no | i32 | |
| character | i32 code point | a one-character literal, as warp holds `"a"` |
| text | `(ref null $text)`, `$text = (array (mut i8))` | UTF-8; literals from one passive data segment (array.new_data) |
| list | `(ref null $list)`, `$list = (struct (mut i32 length) (mut (ref $items)))`, `$items = (array (mut (ref null $Node)))` | a vector: `xs#i` O(1); items boxed |
| any kind (a value of mixed branches, a list item, ø) | anyref holding a `$Node` | |

Boxing (`boxed` in emitter.warp) makes a `$Node(kind i64, data anyref, value ref null $Node)` ("Node serialization"
below): an int or yes/no in an `$i64box`, a float in an `$f64box`, a character as an i31, a text with data =
its `$text`, a list with data = its `$list`. A value is boxed only where its kind is not known statically: a list item,
a value of branches of different kinds, an `any` parameter, and main's result unless it is a number. Texts are GC
arrays; linear memory holds one only at the host/FFI edge.
Readers of the layout (each place a module's result or a value crossing the boundary is read):
- web/playground/values.js reads a module's values in the browser through its exports (shown_at, item_at …);
  core/native/driver.warp reads them natively through the same exports.
- Host imports and FFI take $String (ptr, len) texts in linear memory. emitter.warp imports
  only WASI's fd_write, and only for a program that says `print` (imports shift function indices, so it is decided
  before compiling): one page of memory, exported as "memory", where the runtime word print_line copies a text's bytes
  a chunk at a time (iovec at 0, written count at 8, bytes from 16).
Runtime words are warp: core/runtime.warp (an index of `use "runtime/<part>.warp"` lines, one idea per part
in core/runtime/), written in idiomatic warp (for loops and all sugar) and lowered by
the same pipeline as any program before emitter.warp sees them (`wasm_module(program, runtime)`).
emitter.warp handles only lowered forms, never hand-lowered source. The words use a
handful of intrinsics, one or two instructions each and callable only from runtime words: texts (new_text, copy_text,
byte_count, byte_at, set_byte), lists (new_list, copy_items, adopt_items), numbers (int_quotient, int_remainder,
truncating; code_of, char_of), boxed values (kind_code, marked_kind, unboxed_int/float/text/char, is_null), output
(output_byte, write_output) and fail (a trap). Each word is compiled on its first call like the program's own
functions: text_join and list_join (`+`), list_append (`+=` on a list, in place, so aliases see it), text_count (`#`
on a text: code points), text_char (`t#i`), text_order (text comparisons), print_line, int_text, char_text, node_text
(print and `+` of a value known only at run time).

## Runtime calls into prelude words
A word whose code can only be written once the module's closures and classes are known (the closure dispatchers, the
instance words of `late_words`) is made at its first call with a placeholder body and filled in afterwards
(`made_late`, `late_code` in core/emitter/closures.warp); behind runtime.wasm such a word is runtime.wasm's, written
there once.

## Node serialization

=== Node Serialization Workflow (Current Spec) ===
node.warp via emitter.warp in notes/self_hosting.md "Value layout"

1. Core GC types emitted to WASM:

   (type $Text (array (mut i8)))      ;; type 0: a text's UTF-8 bytes

   (type $Node (struct
     (field $kind i64)            ;; Kind tag + extra info in high bits
     (field $data anyref)         ;; payload (boxed number, $Text, i31ref, list or node)
     (field $value (ref null $Node)))) ;; secondary payload (node or null)

   (type $i64box (struct (field $value i64)))
   (type $f64box (struct (field $value f64)))

   Text literals come from one passive data segment via array.new_data 
   Linear memory holds a text only at the host/FFI edge, or in optimized algorithms, as 
   (type $String (;0;) (struct (field $ptr i32) (field $len i32))) 
   <!-- `text_address` copies a $Text there for a host
   or C call, `text_from_memory` copies host bytes into a new $Text (`new_text(ptr, len)` makes a Text node so).
   Lists held as arrays: `$IntList`, `$FloatList`, `$NodeList` (length, items array), maps `$NodeMap`; a program that
   quotes code gets `$MetaNode`, a $Node subtype with line, column and comment. -->

2. Kind encoding:

   - Lower 8 bits are `Kind` (Empty=0, Int=1, Float=2, Text=3, Codepoint=4, Symbol=5, Key=6, Block=7, List=8, Data=9, Meta=10, Error=11, TypeDef=12, ... Time=18)
   - For Key and List, extra info is stored in the high bits:
     - key: `kind = (op_info << 8) | Kind::Key`
     - list: `kind = (bracket_info << 8) | Kind::List`
       bracket_info: Curly=0, Square=1, Round=2, Less=3, Other=4, None=5

3. Data/value payload mapping:

   - Empty: `data = null`, `value = null`
   - Int: `data = ref $i64box(value)`, `value = null`
   - Float: `data = ref $f64box(value)`, `value = null`
   - Codepoint: `data = i31ref(char)`, `value = null`
   - Text/Symbol: `data = ref $Text`, `value = null`
   - Key: `data = left node`, `value = right node`
   - List/Block: `data = $list` vector (length, items array)
   - TypeDef: `data = name node`, `value = body node`
   - True/False: a bool, the Int 1/0 marked BOOL_INFO above the kind bits (kind = BOOL_KIND); prints yes/no (wiki/bool.md)
   - Meta is dropped during emission; Error currently emits the inner node
   - Data nodes currently serialize as `Symbol(type_name)` in the emitter

4. Round-trip:

   code -> parse -> Node -> wasm emitter -> WASM GC object -> Node -> value


## complete roundtrip test

is! <code> <value>   in lib/prelude.warp

is! invokes the whole machinery, to parse, analyze, emit to wasm, read back, run / convert to Node again

is!("3+3",6); => parse("3+3") -> Node -> wasm_node -> test.wasm -> wasm_node -> Node == 6
```
is!("def fib:=it<1 ? 1 : fib(it-1) + fib it-2; fib(10)",55); 
```
