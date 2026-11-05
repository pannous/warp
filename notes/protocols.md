# Protocols (card protocol-default-implementation; decisions map-protocol, lowercase-protocols, matrix-protocol)

`protocol shape { area(); perimeter() := 2 * area() }` (`trait` the same word) names what its values can do.
Spec: wiki/trait.md ("Protocol and default class"). One general mechanism, extensions/protocols.warp; no core change.

- Required word: a bare name, a signature `area()`, or a body of `pass`. Any other member is a default word.
- A default word becomes a function of the program, `perimeter(self) := 2 * self.area()`; the protocol's own words
  inside its body are called on `self`. A class giving the word itself keeps its own: the class methods route a
  method named like a program function (extensions/class_methods/methods.warp, `perimeter·method`).
- Conformance is structural: a class conforms when it, or a class it extends, has each required word as a method,
  or a function takes it first (`area(q: sq) := …`). `x is shape` becomes an `is` test over the conforming classes.
- A place typed by the protocol, `f(x: shape)`, takes any value (typed as `any`); dispatch is the methods'.
- The pass runs before definition_heads, so the class-method lifting sees the default functions as program functions.

## map, a built-in protocol (landing B, decision map-protocol)
- extensions/protocols.warp `built_in_protocols()`: map with keys/get/has/set/remove required. Only when a class of the
  program conforms does it act: `x is map` tests the built-in maps and those classes, `f(m: map)` takes any value.
- The map forms on such a class call its methods (extensions/class_methods/methods.warp map_form_call): `keys(p)`,
  `values(p)` (keys mapped through get when it has no values), `p[k]`, `p.k` (no field k), `p[k] = v`, `k in p`,
  `for k in p` (keys, when it has no next()); lib/map.warp's words reach them through the dispatchers. A bare call
  only when the method takes the arguments after the first (`remove(x)` in a class stays its own call).
- A name whose class is known only at run time (`f(m: map) := m["a"]`) gets an instance test per text-keyed form.
- A class of a linked library module (ListMap in lib/collections.warp) is seen through module.linked_words.
- Default class: `Map{…}` / `map{…}` / `Map x = v` build the hashed map; after `map default ListMap` they call
  `ListMap(the hashed map)`. The early sugar makes `map default C` `map ?? C`, which the pass reads.
- ListMap vs the hashed map, 2M reads (net of start-up; collections adds ~0.17 s start-up): 8 keys 0.53 vs 0.33 µs,
  200 keys 1.97 vs 0.36 µs (a linear scan). An opt-in, never the bare `{…}`.

## Next landings
- C, matrix: `matrix` a protocol (rows, columns, the element at i j required), lib/matrix.warp's list of lists the
  default class: one more entry of built_in_protocols(), `matrix default DenseMatrix` and `Matrix{…}` the same way.
