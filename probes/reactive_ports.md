# Reactive and event cases ported from other systems, in wasp's signal syntax (notes/signals.md): "code ||| expected"
# run: probes/function_calls.sh probes/reactive_ports.md (scratch/warp of this checkout)
# Svelte: $: derivations and stores
count = 0; doubled := count * 2; count = 3; doubled ||| 6
count = 0; alerts = 0; whenever count >= 10 {alerts += 1}; count = 5; count = 10; count = 11; alerts ||| 2
count = 0; log = 0; on change count {log = log*10 + value}; count = 1; count = 2; log ||| 12
count = 1; seen = 0; on set count {seen = value}; count += 1; seen ||| 2
count = 1; seen = 0; on set count {seen = value}; count++; seen ||| 2
# Vue: computed, watch (with the old value), watchEffect
first = "Ada"; last = "L"; full := first + " " + last; first = "Grace"; full ||| Grace L
x = 1; diff = 0; on change x {diff = value - old}; x = 5; diff ||| 4
p = {age: 1}; n = 0; on change p.age {n += 1}; p.age = 2; n ||| 1
p = {age: 1}; seen = 0; on set p.age {seen = value}; p.age = 7; seen ||| 7
a = 1; runs = 0; whenever a > 0 {runs += 1}; a = 2; a = 3; runs ||| 2
# SolidJS: createEffect, createMemo
a = 1; b = 2; runs = 0; whenever a + b > 0 {runs += 1}; a = 5; b = 6; runs ||| 2
a = 2; square := a*a; sum = 0; on change square {sum += value}; a = 3; a = 3; a = 4; sum ||| 25
a = 2; square := a*a; seen = 0; on change square {seen = value}; a = -2; seen ||| 0
# RxJS: Subject next/subscribe, filter, unsubscribe
total = 0; on price {total += event.value}; raise price{value: 5}; raise price{value: 7}; total ||| 12
evens = 0; on number {if event.n % 2 == 0 {evens += 1}}; for i in 1 to 6 {raise number{n: i}}; evens ||| 3
x = 0; n = 0; s = on set x {n += 1}; x = 1; remove s from listeners of x; x = 2; n ||| 1
# C# events: += and -= of handlers
x = 0; n = 0; h = on set x {n += 1}; x = 1; listeners of x -= h; x = 2; n ||| 1
log = 0; on tick {log = log*10 + 1}; on tick {log = log*10 + 2}; raise tick; log ||| 12
# Node EventEmitter: on, once, off, emit with arguments, listenerCount
n = 0; once alarm {n += 1}; raise alarm; raise alarm; n ||| 1
n = 0; h = on alarm {n += 1}; raise alarm; remove h from listeners of alarm; raise alarm; n ||| 1
sum = 0; on add {sum += event.a + event.b}; raise add{a: 1, b: 2}; sum ||| 3
on tick {1}; on tick {2}; count listeners of tick ||| 2
n = 0; raise alarm; n ||| Error("alarm")
# Qt: connect a signal to a slot, chained signals, valueChanged only on change
seen = 0; def show(v){ global seen; seen = v }; x = 0; on set x : show(value); x = 9; seen ||| 9
n = 0; on pressed {raise clicked}; on clicked {n += 1}; raise pressed; n ||| 1
x = 1; n = 0; on change x {n += 1}; x = 1; x = 1; x = 2; n ||| 1
# JS addEventListener: once option, removeEventListener
n = 0; once ready {n += 1}; raise ready; raise ready; n ||| 1
n = 0; h = on ready {n += 1}; remove h from listeners of ready; raise ready; n ||| 0
