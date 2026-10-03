# Closures: functions as run-time values

Functions and lambdas are values: stored in variables and lists, passed, returned, called later. Capture stays BY VALUE
(decision D7/#33): a closure copies the captured variables when it is made. Compile-time specialisation
(src/function_values.rs, decision #37) stays wherever the function is known; closures cover the rest.

```wasp
make_adder(n) := (x => x+n); add2 = make_adder(2); add2(5)      # 7
apply(f, x) := f(x); k=4; apply(y=>y*k, 3)                       # 12, generic apply with a closure
pick(c) := if c then double else square; f = pick(1); f(5)       # chosen at run time
fs=[x=>x+1, x=>x*2]; f=fs#2; f(5)                                # 10
add(a) := (b => (c => a+b+c)); add(1)(2)(3)                      # 6
sum_with(f, xs) := reduce xs f; k=0; sum_with((a b)->a+b+k, [1 2 3])
```

## Pipeline (wasm_emitter::lower_for_emission)
1. `lambdas::lower`: inlines lambdas where they stand (`f = x=>x*2` is a definition, `map xs {it*2}` a loop). Unchanged.
2. `function_values::lower`: specialises functions taking functions (`apply__double`). New: when an argument is not
   known at compile time (a variable, a capturing lambda, a call), the call stays and the function gets a **generic
   version** beside its specialisations (`make_generic`); `f x` with a function parameter becomes `f(x)` there.
   A literal number or text where a function is expected stays the loud `functions are not first-class values yet`.
3. `closures::lower` (src/closures.rs, new):
   - every remaining arrow lambda is **lifted** to `closure_lambda_n(captured…, params…) := body` (top level) and replaced
     by `closure_new(closure_lambda_n, captured…)`; captured = variables of the enclosing scopes the body reads, in order;
   - a function name where a value is expected (assignment value, list item, argument, function result, if/else
     branch) becomes `closure_new(name)`;
   - `f(args)` where `f` is a parameter or a variable assigned a function value becomes `closure_call_n(f, args…)`;
     `make_adder(1)(2)` / `add(1)(2)(3)` chain closure calls. "Assigned a function value" is judged from the source
     (`FunctionValues::settle`: lambdas, function names, calls of functions whose body ends in one, calls of closures,
     `fs#i` of a list holding function values, the variable of `for f in fs`). `(fs#2)(5)` calls an item directly, and
     `fs#2(5)` (parsed `fs#(2*(5))`) too when fs holds function values.
4. `lambdas::lower_strict`: an iteration word over a function known only at run time (`map xs f`, `reduce xs f`) calls
   the closure in its loop body (`closure_call_1(f, item)`).

## Analysis (src/analyzer.rs)
- `Kind::Function = 16`, a ref kind (stored as `ref $Node`); type words `function`, `closure` (`apply(f:function, x)`).
- `closure_new(…)` infers Kind::Function; a parameter called through `closure_call_n` is used as a Function.
- `closures::register_closure_calls` adds each `closure_call_n` as a user function: params (function, n × Data), body
  empty (the emitter writes it); its return kind is the common return kind of all closure targets of arity n, Data (any
  Node) when they differ (`refine_return_kinds`).
- `infer_closure_parameters`: a lifted lambda is never called by name, so its captured parameters take the kinds of the
  captured variables and its parameters the kinds of literal arguments of closure calls of its arity;
  `infer_forwarded_parameters` reads `closure_new(target, captured…)` as a call of target.

## Run time (src/wasm_emitter/closures.rs)
```
(type $Closure (struct (field $entry (ref func)) (field $captured (ref null $Node))))
(type $closure_entry_n (func (param (ref null $Node)) (param (ref null $Node))×n (result (ref null $Node))))
```
- A closure value is a `$Node` with kind 16, data = `$Closure`, value = the target name as a Symbol (it prints as that name).
- The captured values are a list Node (cons cells), built when the closure is made: that is the by-value copy.
- `closure_entry_<target>` (one per target, declared in an element segment for `ref.func`): unboxes captured values
  and arguments to the target's parameter kinds, calls it, boxes the result. One universal Node -> Node ABI, so any
  closure fits any call of its arity.
- `closure_call_n(f, a1…an)`: `ref.cast $Closure` of f's data, `ref.cast $closure_entry_n` of its entry, `call_ref`,
  unbox the result to the helper's return kind.

## Open
- **Typed fast path**: when every closure a call can reach shares one signature (e.g. Int -> Int), call_ref a typed entry
  without boxing. Today everything boxes through Nodes (correct, slower).
- **Per-site result kinds**: the result kind of `closure_call_n` is joined over ALL closures of arity n in the program;
  mixing e.g. an Int closure and a Function-returning closure of the same arity makes every call return Data (any
  Node), which works for printing and `+` on the final Int but not every use. A flow analysis of which targets a
  variable can hold would fix it.
- A closure called with the wrong arity, or a non-function called as one, traps with `cast failure` instead of a
  wasp error message.
- A lifted lambda prints as `closure_lambda_n`, not its source.
- Float parameters of a closure unbox only from Float nodes (an Int argument traps).
- Variables assigned in another function's body count as capturable everywhere (names, not scopes).
- Mutable capture (counters, `c=c+1` inside a closure) stays by value per D7: each call sees the value at creation.
