# Inspiration: mutation, scope, bounds

Group `mutation-scope-bounds`. Claims marked (verified) were run locally on 2026-09-26 in
`probes/footguns/inspiration/scratch/msb_*` (Swift 6.0.3, Go 1.27.1, rustc, node, python3, java).

### Aliasing
Solved elsewhere: **Swift**: `var a=[1]; var b=a; b[0]=9; print(a)` → `[1]` (verified) — Array/String/Dictionary are value types with copy-on-write: the buffer is shared until a write, `isKnownUniquelyReferenced` decides whether to copy.
**Rust**: `let mut b = a; b[0]=9; a` → *compile error E0382, use of moved value* — assignment moves ownership; sharing needs an explicit `.clone()` or `&`/`Rc`.
**Clojure / Immutable.js**: `(let [a [1] b (assoc a 0 9)] a)` → `[1]` — persistent data structures with structural sharing, mutation returns a new value.
Adopt in Warp: Swift's model is exactly DESIGN.md's ownership rule 5: lists/text/records are values, `b=a` shares the GC ref, and a write through `b#1=` copies unless the ownership pass proves `b` unique (then mutate in place). This keeps `=` cheap and the observable semantics identical whichever plan is chosen.

### Mutable default arguments
Solved elsewhere: **Swift**: `func f(_ xs:[Int]=[]) {var ys=xs; ys.append(1); return ys}; f(); f()` → `[1] [1]` (verified) — default expressions are re-evaluated at every call site, and arrays are values anyway.
**Kotlin / C++ / JS**: `fun f(a: MutableList<Int> = mutableListOf())` → fresh list per call — defaults are call-site expressions, not objects stored on the function.
(**Python** workaround `def f(a=None): a = [] if a is None else a` → `[1] [1]` (verified) shows the cost of the wrong default.)
Adopt in Warp: define a default as an expression elaborated at the call site (inserted into the argument list in semantic IR), never a value stored once with the function; with value semantics the footgun is doubly impossible. The current panic on `def f(a=())` must become a type/elaboration error with span if it is not fixed.

### Closures capturing loop variables
Solved elsewhere: **Go ≥ 1.22**: `for i:=0;i<3;i++ { fs=append(fs, func()int{return i}) }` → `0 1 2` (verified) — each iteration gets a fresh `i` (loopvar semantics change).
**JS `let`**: `for (let i=0;i<3;i++) fs.push(()=>i)` → `[0,1,2]` (verified) — per-iteration binding, unlike `var`.
**Rust / Swift**: `(0..3).map(|i| move || i)` → `[0,1,2]` (verified) — loop variables are immutable per-iteration bindings; `move` captures by value, capturing a mutable by reference that outlives it is a borrow error.
Adopt in Warp: loop variables are fresh immutable bindings per iteration and closures capture by value (a reassignable outer local captured by a closure is either copied at capture time or rejected with a diagnostic, never shared by reference silently). This fits the "immutable local bindings" core and makes closures pure by default.

### Index out of range / negative index
Solved elsewhere: **Rust**: `vec![1,2,3].get(3)` → `None`, `.last()` → `Some(3)` (verified); `a[3]` panics with the index and length — checked indexing is the default, the unchecked form is `unsafe get_unchecked`.
**Go**: `a[3]` → `runtime error: index out of range [3] with length 3` (verified) — always bounds checked, message carries index and length.
**JS `Array.at`**: `[1,2,3].at(-1)` → `3` (verified) — negative indexing only through an explicit method; plain `a[-1]` / `a[3]` stay `undefined` (the footgun). **Swift**: `arr.indices.contains(3)` / `arr.last` → optional, `arr[3]` traps.
Adopt in Warp: two spellings, both checked: `x#i` traps/returns an error value carrying span, index and length (1-based, so `#0` is always an error), and a total variant (e.g. `x#?i` or `x.get i`) returns `Option<T>` for code that wants to branch. Negative counting only via an explicit `last`/`from end` form, never by wrap-around of `#-1`.

### Variance
Solved elsewhere: **Kotlin / Scala / C#**: `List<out E>` read-only is covariant, `MutableList<E>` is invariant — declaration-site variance checked by the compiler, so Java's `Object[] a = new String[1]; a[0]=1` (verified: `ArrayStoreException` at runtime) cannot type check.
**Rust**: `&T` covariant, `&mut T` invariant, inferred from usage: pushing a short-lived `&str` into a `Vec<&'static str>` via `&mut` → `E0597 s does not live long enough` (verified).
**Java generics** (use-site `List<? extends Number>`) show the verbose alternative.
Adopt in Warp: infer variance from use like Rust instead of annotating it: immutable values (the default) are covariant, a parameter or place that is written through (a mutation effect on it) is invariant. Since collections are values with copy-on-write, most code never meets invariance at all.

### Parameter shadowing
Solved elsewhere: **Swift**: `let x=1; func g(x:Int)->Int{x*2}; g(x:5)+x` → `11` (verified) — lexical scoping, parameter wins, outer untouched.
**Haskell (`-Wname-shadowing`), Go (`go vet -vettool shadow`), Kotlin ("Name shadowed" warning)** — shadowing is legal but the compiler reports it, catching the "meant the outer one" bug.
**Rust**: `let x = x + 1` shadowing is idiomatic, but an unused outer binding triggers `unused_variables`.
Adopt in Warp: keep lexical parameter shadowing (already Solved) and add a lint-level diagnostic with span when a parameter or local shadows an outer binding that is then used in the same function, with a fix-it rename; never an error, since data formats reuse names.

### Memory-safety classics: use-after-free, double free, dangling pointers
Solved elsewhere: **Rust**: `let v=vec![1]; drop(v); v` → *compile error E0382 borrow of moved value* (verified) — ownership + borrow checker, no GC.
**Java / Go / JS / WASM GC**: tracing GC, no `free`, no pointer arithmetic, bounds-checked arrays. **Swift**: ARC + exclusivity checks, `Unsafe*Pointer` quarantined by name.
Adopt in Warp: already safe by construction via WASM GC; keep any linear-memory or FFI access behind the `Unsafe` effect (DESIGN.md effect set) so it is visible in signatures, and make ownership-pass optimisations (stack/unique allocation) never able to produce a dangling reference: fall back to GC when escape analysis is unsure.

### Data races
Solved elsewhere: **Rust**: `let mut n=0; thread::spawn(|| n+=1); n+=1;` → *compile error E0373 / E0503* (verified) — `Send`/`Sync` traits plus borrow rules: shared XOR mutable across threads.
**Swift 6 strict concurrency**: actors and `Sendable` checking make cross-actor mutable sharing a compile error. **Erlang / Elixir, Pony**: share-nothing processes with message passing (Pony's reference capabilities prove race freedom statically).
Adopt in Warp: stay share-nothing: parallelism only over values proved pure (DESIGN.md Cautions), threads/components communicate by copying or moving values (WASM components already share no memory), and a future shared-memory mode requires an explicit `Sendable`-like capability checked by the ownership pass.
