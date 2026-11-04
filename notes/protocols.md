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

## Next landings
- B, map: `map` a protocol with keys/get/has/set/remove required, lib/map.warp's words its defaults; the five gaps
  (bare keys(p), p["k"] / p.k / p["k"] = v, `k in p`, `for k in p`, get_or) close by routing those to the class's
  methods; `Map{…}` / `map{…}` build the default class, `map default ListMap` switches it; implementations convert
  transparently (`Map x = {a: 1}`, `f(a: map)`); ListMap measured against NodeMap before it ships.
- C, matrix: `matrix` a protocol (rows, columns, the element at i j required), lib/matrix.warp's list of lists the
  default class.
