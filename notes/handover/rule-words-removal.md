# rule-words-removal (parked, 2026-10-30)

Goal: rule_words.warp and wgsl.warp leave warp.wasm, running as a prebuilt module that the compiler calls
through the native driver.

State of the branch:
- core/extensions/compiler_rule_words.warp: the rule words that need the compiler (reflection, spelled,
  declared_local, module_named, dir). core/extensions/reflection.warp: the shared function-reading helpers.
- lib/rule_words.warp: the module words (routes, signals, frames, gpu, wgsl) as a program,
  `rule_words.wasm <word> <given> <answer>`.
- core/passes/rewrites.warp computed: compiler words are called directly; any other word goes through
  std_io("rule_words", word, [code]). core/native/driver.warp rule_word_answered runs modules/rule_words.wasm.
- parse_words.warp: code_encoding(n, exact) adds a `p` tag for positions; code_decoded decodes it, and an `x`
  tag raises its message (checked in a probe).

Why it is parked (measured):
- The module words need parser, walk, analyzer, uses and prelude_words (~9.4k lines) plus the baked tables,
  so the module would be a second compiler minus the emitter (≥4–5 min per build). That saves only ~113 of
  3958 functions (~3%).
- The prelude program rule gpu_lists_packed fires on every compile that reaches the prelude rules (the
  compiler build's runtime/modules step included): every such compile would need a host round trip, and an
  older driver fails the build.
- No module build exists yet: the core files do not compile standalone (each try finds one more missing
  `use`).
Cheaper paths: gate the gpu_lists_packed program rule, or rewrite the module words in plain warp with no
compiler slice.
