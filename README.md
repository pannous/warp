# 𖦹 Warp

<!-- 𓏲 w 𓍢 𐀸 we ꩜ CHAM PUNCTUATION SPIRAL -->

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
		name: James,
		age: 33,
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

## Develop
`git checkout --single-branch --branch main https://github.com/pannous/warp`

## Build & Test
`cargo test --all`