# A linked-module find-variant failure (finding only; card lib-yaml-json-tools-edges)

Bundled (precompiled) modules, main 1825b189a:

```warp
use toml
use json_tools
print(parse_toml("a = 1"))
```

fails in either `use` order with

    `(find rest (str next))` fits no single variant of find(text, text), find(list, function), find(text, function) at <program>:942:72

Each module alone prints `{a: 1}`; compiled from source (a WARP_LIB copy with one extra trailing newline in the module) it
works too. 942:72 is `rest.find(str(next))` in `fits_text_cut` in lib/prelude.warp, blamed on the program file.
Making `rest` provably a text (`rest = str(value)`) and rebuilding changed nothing, so the overload choice for linked
modules is core's: card core-linked-module-divergence. It keeps samples/apps/config_validator and site_generator in
test/apps/known_failures.txt.
