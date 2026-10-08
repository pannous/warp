# Reactive and event cases ported from other systems, in warp's signal syntax (notes/signals.md): "code ||| expected"
# run: probes/function_calls.sh probes/reactive_ports.md (scratch/warp of this checkout)
# Svelte: $: derivations and stores
count = 0; doubled := count * 2; count = 3; doubled ||| 6
count = 0; alerts = 0; whenever count >= 10 {alerts += 1}; count = 5; count = 10; count = 11; alerts ||| 1
count = 0; log = 0; on change count {log = log*10 + value}; count = 1; count = 2; log ||| 12
count = 1; seen = 0; on set count {seen = value}; count += 1; seen ||| 2
count = 1; seen = 0; on set count {seen = value}; count++; seen ||| 2
# Vue: computed, watch (with the old value), watchEffect
first = "Ada"; last = "L"; full := first + " " + last; first = "Grace"; full ||| Grace L
x = 1; diff = 0; on change x {diff = value - old}; x = 5; diff ||| 4
p = {age: 1}; n = 0; on change p.age {n += 1}; p.age = 2; n ||| 1
p = {age: 1}; seen = 0; on set p.age {seen = value}; p.age = 7; seen ||| 7
a = 1; runs = 0; whenever a > 0 {runs += 1}; a = 2; a = 3; runs ||| 1
# SolidJS: createEffect, createMemo
a = 1; b = 2; runs = 0; whenever a + b > 0 {runs += 1}; a = 5; b = 6; runs ||| 1
a = 2; square := a*a; sum = 0; on change square {sum += value}; a = 3; a = 3; a = 4; sum ||| 25
a = 2; square := a*a; seen = 0; on change square {seen = value}; a = -2; seen ||| 0
# RxJS: Subject next/subscribe, filter, unsubscribe
total = 0; on price {total += event.value}; emit price{value: 5}; emit price{value: 7}; total ||| 12
evens = 0; on number {if event.n % 2 == 0 {evens += 1}}; for i in 1 to 6 {emit number{n: i}}; evens ||| 3
x = 0; n = 0; s = on set x {n += 1}; x = 1; remove s from listeners of x; x = 2; n ||| 1
# C# events: += and -= of handlers
x = 0; n = 0; h = on set x {n += 1}; x = 1; listeners of x -= h; x = 2; n ||| 1
log = 0; on tick {log = log*10 + 1}; on tick {log = log*10 + 2}; emit tick; log ||| 12
# Node EventEmitter: on, once, off, emit with arguments, listenerCount
n = 0; once alarm {n += 1}; emit alarm; emit alarm; n ||| 1
n = 0; h = on alarm {n += 1}; emit alarm; remove h from listeners of alarm; emit alarm; n ||| 1
sum = 0; on add {sum += event.a + event.b}; emit add{a: 1, b: 2}; sum ||| 3
on tick {1}; on tick {2}; count listeners of tick ||| 2
n = 0; raise alarm; n ||| Error("alarm")
# Qt: connect a signal to a slot, chained signals, valueChanged only on change
seen = 0; def show(v){ global seen; seen = v }; x = 0; on set x : show(value); x = 9; seen ||| 9
n = 0; on pressed {emit clicked}; on clicked {n += 1}; emit pressed; n ||| 1
x = 1; n = 0; on change x {n += 1}; x = 1; x = 1; x = 2; n ||| 1
# JS addEventListener: once option, removeEventListener
n = 0; once ready {n += 1}; emit ready; emit ready; n ||| 1
n = 0; h = on ready {n += 1}; remove h from listeners of ready; emit ready; n ||| 0
# Round 2: Solid batch, MobX reaction/when, Angular effect, Kotlin StateFlow, RxJS distinct/scan/take
a = 0; b = 0; runs = 0; whenever a + b == 3 {runs += 1}; a, b = 1, 2; runs ||| 1
a = 0; b = 0; seen = 0; whenever a + b > 0 {seen += 1}; a, b = 1, 2; seen ||| 1
temp = 20; alarms = 0; once temp > 30 {alarms += 1}; temp = 31; temp = 35; alarms ||| 1
x = 0; sum = 0; on change x {sum += value}; x = 1; x = 1; x = 2; x = 2; sum ||| 3
clicks = 0; total = 0; on click2 {total += 1}; for i in 1 to 3 {emit click2}; total ||| 3
n = 0; taken = 0; h = on tick {taken += 1; if taken == 2 {remove h from listeners of tick}}; for i in 1 to 5 {emit tick}; taken ||| 2
items = []; on add {items = items + [event.x]}; emit add{x: 1}; emit add{x: 2}; count items ||| 2
x = 0; doubled := x * 2; quad := doubled * 2; last = 0; on change quad {last = value}; x = 3; last ||| 12
user = {name: "a"}; greeting := "hi " + user.name; seen = ""; on change greeting {seen = value}; user.name = "b"; seen ||| hi b
count = 0; log = ""; whenever count > 2 {log = log + "!"}; for i in 1 to 5 {count = i}; log ||| !
# Round 3: Vue deep watch of a list, Svelte reactive statements over lists, Qt disconnect all, EventEmitter removeAllListeners, prependListener order
xs = [1]; n = 0; on change xs {n += 1}; xs = xs + [2]; n ||| 1
xs = [1, 2]; total := sum xs; seen = 0; on change total {seen = value}; xs = [3, 4]; seen ||| 7
x = 0; n = 0; on set x {n += 1}; on set x {n += 10}; clear listeners of x; x = 1; n ||| 0
n = 0; on alarm {n += 1}; on alarm {n += 10}; clear listeners of alarm; emit alarm; n ||| 0
log = 0; on step {log = log*10 + 1}; on step {log = log*10 + 2}; on step {log = log*10 + 3}; emit step; log ||| 123
x = 0; out = 0; on change x {out = value * 2}; x = 21; out ||| 42
x = 5; y := x + 1; z := y * 2; whenever z > 20 {x = 0}; x = 10; x ||| 0
s = "a"; n = 0; on change s {n += 1}; s = s + "b"; s = s + "c"; n ||| 2
f = 1.5; seen = 0.0; on change f {seen = value}; f = 2.5; seen ||| 2.5
# Round 4: listeners in functions and loops, events from functions, payload kinds, handler order across definitions
n = 0; def bump() { emit bumped{by: 2} }; on bumped {n += event.by}; bump(); bump(); n ||| 4
name = ""; on greet {name = event.who}; emit greet{who: "Ada"}; name ||| Ada
total = 0.0; on pay {total += event.amount}; emit pay{amount: 1.5}; emit pay{amount: 2.25}; total ||| 3.75
xs = []; on item {xs = xs + [event]}; emit item 1; emit item 2; count xs ||| 2
hits = 0; for i in 1 to 3 { on tick2 {hits += 1} }; emit tick2; hits ||| 3
x = 0; log = 0; def setx(v) { global x; x = v }; on set x {log += 1}; setx(1); setx(2); log ||| 2
x = 0; seen = 0; def setx(v) { global x; x = v }; whenever x > 1 {seen += 1}; setx(1); setx(5); seen ||| 1
a = 1; b := a + 1; c := b + 1; last = 0; on change c {last = value}; a = 10; last ||| 12
# Round 5: derived chains with text and lists, whenever with else-like toggles, once vs whenever ordering, listener on a loop counter
temp = 10; state = "cold"; whenever temp > 25 {state = "hot"}; whenever temp <= 25 {state = "cold"}; temp = 30; temp = 20; state ||| cold
n = 0; log = 0; once n == 2 {log = log*10 + 1}; whenever n == 2 {log = log*10 + 2}; n = 2; log ||| 12
total = 0; i = 0; on set i {total += value}; while i < 3 {i += 1}; total ||| 6
names = ["a"]; size := count names; seen = 0; on change size {seen = value}; names = names + ["b"]; seen ||| 2
price = 2; qty = 3; total := price * qty; history = []; on change total {history = history + [value]}; price = 3; qty = 4; count history ||| 2
ready = false; n = 0; whenever ready {n += 1}; ready = true; ready = true; n ||| 1
ready = false; n = 0; on change ready {n += 1}; ready = true; ready = true; n ||| 1
