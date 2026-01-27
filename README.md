# 🌀Warp

<!-- 🌀 𖦹 𓏲 w 𓍢 𐀸 we ꩜ CHAM PUNCTUATION SPIRAL -->

𖦹 **Warp** is a new **data format** and **programming language** that is wasm-first and written in Rust.
It's a rewrite of [Wasp](https://github.com/pannous/wasp) which was written in c⁺⁺ and [Angle](
https://github.com/pannous/angle) which was a python experiment.

## Features
- **Wasm-first**: Designed to compile to WebAssembly efficiently, as structs.
-  JS/JSON/XML/YAML (de)serialization built-in

## Example

### as data format
```warp
contact{
		name: James
		age: 33
}
```
That's it. Similar to JSON but no quotes around keys and symbols.

### as programming language
```warp
use math
to square(number){
 return number²
}
```
or simpler `square:=it²` showing optional return types, it keyword, type and parameter inference.

### Wasm interop
 🎉  The wasp programming language has FULL WebAssembly roundtrip support for structs:
 ```
let alice = Person { name: "Alice".into(), age: 30 };
is!("class Person{name:String age:i64}; Person{name:'Alice' age:30}", alice);
```

either with prior declaration of the struct type: 
```
wasm_struct! {
	Person {
		name: String,
		age: i64,
	}
}

// or directly inline:
let alice = wasm_object! { Person { name: String = "Alice", age: i64 = 30 } };
```
What happens is that the WASP compiler emits

```
(module
  (type $String (;0;) (struct (field $ptr i32) (field $len i32)))
  (type $Person (;1;) (struct (field $name (ref $String)) (field $age i64)))
  (type (;2;) (func (result (ref $Person))))
  (memory (;0;) 1)
  (export "memory" (memory 0))
  (export "main" (func $main))
  (func $main (;0;) (type 2) (result (ref $Person))
    i32.const 0
    i32.const 5
    struct.new $String
    i64.const 30
    struct.new $Person
  )
  (data (;0;) (i32.const 0) "Alice")
)
```

### Unstructured Data: Node

The above example or any object can be expressed in the general universal Data type node:
```
(type $Node (struct 
	(field $kind i64) 
	(field $data anyref) 
	(field $value (ref null $Node))
	))
```

Which has a host equivalent of:
```
pub enum Node {
	// Kind(i64), enum NodeKind in serialization via external map
	// Id(i64), // unique internal(?) node id for graph structures (put in metadata)
	Empty, 
	Text(String),
	Symbol(String),
	Number(Number),
	Key(Box<Node>, Op, Box<Node>), 
	List(Vec<Node>, Bracket, Separator),
	Type { name: Box<Node>, body: Box<Node> }, 
	Meta { node: Box<Node>, data: Box<Node> }, // general node extension
	Data(Dada), // most generic container for any kind of data not captured by other node types
}
```

The `Key` type is the most important node and used as Pair e.g. for html{input(type=text)} … and most AST types
Most other are atoms. Type name as node to allow meta info.

## Develop
`git checkout --single-branch --branch main https://github.com/pannous/warp`

## Build & Test
`cargo test --all`
[tests](https://github.com/pannous/warp/tree/main/tests)

289 passed, 0 failed, 289 total tested
