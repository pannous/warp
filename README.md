# 🌀 Warp

**A data format that is also a programming language, compiled straight to WebAssembly GC.**

```warp
contact{ name: James  age: 33 }
assert contact.age + 1 == 34
```

The same notation holds your config, your markup and your program. Warp compiles it to WebAssembly GC structs (no linear-memory runtime, no JS glue), and the compiler is itself a WebAssembly module written in warp!

▶ **[Try it in your browser](https://warp.pannous.com/)**: nothing to install. The compiler runs locally in a Web
Worker.

## Why Warp
Even when 99% of code is written by AI, we can aim for the perfect programming language: ideal for humans and AI
alike, type safe but not verbose, universal and fast (compiled ahead of time to WebAssembly), highly structured and
expressive.
- **Data is code.** Like Lisp, but with blocks, lists, key-value pairs and tags instead of s-expressions.
  JSON-like, but no quotes around keys or symbols. Reads and writes JSON, XML and YAML.
- **Wasm GC first.** A `class Person{name:String age:i64}` becomes a real `(struct …)` type, and the module carries
  full name sections, so any Wasm GC host sees typed objects (see below).
- **No silent footguns.** When code could mean two things, the compiler doesn't guess quietly. It warns with the
  reading it took and the explicit form, or it refuses when a wrong guess would corrupt results
  ([Footguns](https://github.com/pannous/warp/wiki/Footguns)):
  ```
  for i in 1 upto 4 { x += i }
  warning: does `upto 4` include 4? (`..<` or `..` exclude it, `to` or `...` include it) (taking exclusive); fix: ..<
  ```
- **Little ceremony.** Implicit `it`, inferred parameter and return types, Unicode operators, and English words for
  operators where they read better:
  ```warp
  square := it²
  square(3) + square(4)              // 25

  fib := if it < 2 then it else fib(it - 1) + fib(it - 2)
  fib(20)                            // 6765
  ```
- **Markup is just data.**
  ```warp
  html{ body{ h1: "Welcome"  p: "made of data" } }
  ```

## Install

**Nothing to install:** [warp.pannous.com](https://warp.pannous.com/) runs the compiler in your browser.

**Homebrew** (macOS, Linux):
```bash
brew install pannous/tap/warp
```

<!--
**Debian, Ubuntu, Fedora** (glibc 2.39 or newer: Ubuntu 24.04, Debian 13, Fedora 40): the `.deb` or `.rpm` of the
[latest release](https://github.com/pannous/warp/releases/latest), e.g.
-->
**Linux**:  
`sudo apt install ./warp-lang_*_amd64.deb` or `sudo dnf install ./warp-lang-*.x86_64.rpm`.

**Prebuilt binaries** (macOS arm64 and x86_64, Linux x86_64 and arm64): download the archive for your platform from the
[latest release](https://github.com/pannous/warp/releases/latest) and put `warp` on your `PATH`.

**From source** (warp builds itself, so it needs an installed `warp` first):
```bash
git clone https://github.com/pannous/warp && cd warp
warp rebuild                              # warp.wasm, the compiler written in Warp (core/)
warp build native core/native/warp.warp   # the native warp command
```

## Run

```bash
warp               # interactive console "repl"
warp help                         # all commands
warp sample.warp   # run a file, server, webapp or gui
warp build sample.warp # the standalone executable (aot native and .wasm)
warp test              # the tests under test/
```

`samples/` contains classics (game of life, Dijkstra, Levenshtein, a JSON parser, a neural net) in both a
Python-like style and an idiomatic Warp style, one folder per chapter of the guide (samples/graphics, samples/sound …).

## Packages

`use` loads a local module, or else a package of the registry
[warp-packages](https://github.com/pannous/warp-packages) by name, fetched once into `packages/`:
```warp
use colors
to_hex(mix(from_hex("#ff0000"), from_hex("#0000ff"), 0.5))   # "#800080"
```
([use](https://github.com/pannous/warp/wiki/use)).

## Wasm GC objects

```warp
class Person{ name: text  age: int }
alice = Person{ name: "Alice"  age: 30 }
alice.age + 1                      // 31
```
A class is a real GC struct, not a map in linear memory. The compiler emits (`warp compile`, simplified):
```wat
(type $Text (array (mut i8)))
(type $Person (struct (field $name (ref $Text)) (field $age i64)))
(func $main (result (ref $Person))
  i32.const 0  i32.const 5  array.new_data $Text 0   ;; "Alice" from the data segment
  i64.const 30
  struct.new $Person)
(data "Alice")
```
Texts are GC byte arrays; linear memory is used only to hand bytes to the host or C.
<!-- and for bootstrapping the native warp via wasm2c -->

Untyped data travels as one universal GC type, `$Node {kind: i64, data: anyref, value: (ref null $Node)}`. It mirrors
the `Node` enum (`Text`, `Symbol`, `Number`, `Char`, `Key`, `List`, `Type`, `Meta`, `Data`, `Error`) in
[core/node.warp](core/node.warp).

## Status

Proven to be (almost) **production ready** by being able to compile the whole language written in warp [itself](https://en.wikipedia.org/wiki/Self-hosting_(compilers))! Experimental and moving fast. Tests live in test/: Warp files run by `warp test` through the Warp-written compiler. The language spec lives in the
[wiki](https://github.com/pannous/warp/wiki), and parts of it are not implemented yet. Warp began as a Rust rewrite of
[Wasp](https://github.com/pannous/wasp) (C++), which follows the Python experiments
[Angle](https://github.com/pannous/angle) and [english-script](https://github.com/pannous/english-script); its compiler is now written in Warp itself ([core](core)), and so is the native `warp` command, which runs programs through wasmtime's C API ([core/native](core/native)). The old Rust compiler is preserved on the branch [rust-bootstrap](https://github.com/pannous/warp/tree/rust-bootstrap).

The compiler has three layers:
- **core**: just enough for warp to compile itself
- **extensions**: every other language feature
- **libraries**: modules programs import

Feedback, issues and especially "this surprised me" reports are welcome.

## License
The compiler is licensed under the [GNU AGPL v3 or later](LICENSE); the runtime `crates/warp-runtime`, which goes into every program built with warp, under the [MIT license](crates/warp-runtime/LICENSE), so your programs are yours. Commercial licenses of the compiler: info@pannous.com. Contributing: [CONTRIBUTING.md](CONTRIBUTING.md).
