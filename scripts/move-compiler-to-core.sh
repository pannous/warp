#!/usr/bin/env bash
# scripts/move-compiler-to-core.sh: the compiler-to-core flip (card compiler-to-core, step 2; notes/agents/common.md
# "Layout"). Moves lib/compiler to core/ and lib/sugar to core/sugar with git mv, leaves lib/compiler -> ../core and
# lib/sugar -> ../core/sugar as compatibility symlinks (step 3 retires them), makes the reviewed code edits (exact
# strings, counts checked) and rewrites the path text lib/compiler -> core, lib/sugar -> core/sugar in the reviewed
# FILES below only (no global sed). Idempotent: a second run changes nothing. Fails loudly when a path form naming the
# old folders is left outside the allowlist; `--check` runs only that check, `--candidates` lists every tracked file
# naming the old paths (to review files added since the list was made).
# usage: scripts/move-compiler-to-core.sh [--check | --candidates]
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# the forms that name the compiler's or the sugar's old place
PATTERN='lib/compiler|lib/sugar|join\("compiler[/"]|COMPILER_FOLDER|"/compiler"|"/compiler/|/ "compiler"|lib" */ *"compiler"'
# lines allowed to keep one: the compatibility symlinks' note, this script, the test constant's name, and the reviewed
# exceptions (files.txt "needs judgment")
ALLOWED='compatibility symlink|^scripts/move-compiler-to-core\.sh:|^tests/common/twins\.rs:[0-9]+:const COMPILER_FOLDER|^tests/common/twins\.rs:[0-9]+:.*\{COMPILER_FOLDER\}|^tests/parser/test_self_hosted_library\.rs:[0-9]+:.*named\.join\("compiler/passes\.warp"\)'

tracked() { git ls-files -s | awk '$1 != "120000" { print $4 }'; }
left_over() { tracked | tr '\n' '\0' | xargs -0 grep -HnE "$PATTERN" 2>/dev/null | grep -vE "$ALLOWED" || true; }
check() {
	local left; left=$(left_over)
	if [ -n "$left" ]; then
		echo "move-compiler-to-core: FAILED, the old paths are still named outside the allowlist:" >&2
		echo "$left" >&2
		exit 1
	fi
	echo "move-compiler-to-core: check ok, nothing names lib/compiler or lib/sugar outside the allowlist"
}
case "${1-}" in
	--check) check; exit ;;
	--candidates) tracked | tr '\n' '\0' | xargs -0 grep -HcE "$PATTERN" 2>/dev/null | grep -v ':0$' || true; exit ;;
	"") ;;
	*) echo "usage: $0 [--check | --candidates]" >&2; exit 2 ;;
esac

# 1. the layout: step 1's links go first (core -> lib/compiler, lib/compiler/sugar -> ../sugar), then the moves
if [ -L core ]; then git rm -q --cached core; rm core; fi
if [ -L lib/compiler/sugar ]; then git rm -q --cached lib/compiler/sugar; rm lib/compiler/sugar; fi
if [ -d lib/compiler ] && [ ! -L lib/compiler ]; then git mv lib/compiler core; fi
if [ -d lib/sugar ] && [ ! -L lib/sugar ]; then git mv lib/sugar core/sugar; fi
[ -L lib/compiler ] || ln -s ../core lib/compiler
[ -L lib/sugar ] || ln -s ../core/sugar lib/sugar
if [ ! -e core/extensions/README.md ]; then
	mkdir -p core/extensions
	echo '# Compiler extensions: one new capability per file (tuples.warp, varargs.warp, …), a subfolder only for a big topic (concurrency/).' > core/extensions/README.md
fi
git add lib/compiler lib/sugar core/extensions

# 2. the code edits: exact strings, their counts checked
touched=$(mktemp)
TOUCHED=$touched python3 - <<'PY'
# exact-string code edits of the compiler-to-core flip (card compiler-to-core step 2); run by move-compiler-to-core.sh
# from the checkout's root, after the git mv. Each edit: (file, old, new, expected count of old). Idempotent: an edit
# whose `new` is already in the file and whose `old` is gone is skipped; any other count fails loudly.
import os, sys
EDITS = [
 # --- warp: the compiler finds lib/ beside its own core/, and core/ beside lib/ ---
 ("core/parser.warp",
  "// The lib/ a run uses: WARP_LIB, else the one above this file.",
  "// The lib/ a run uses: WARP_LIB, else the one beside this file's core/ (card compiler-to-core).", 1),
 ("core/parser.warp",
  'def run_library(): text := env("WARP_LIB") ?? folder_of(module_directory)\n',
  'def run_library(): text := env("WARP_LIB") ?? folder_of(module_directory) + "/lib"\n'
  '// core/ beside that lib/: the compiler, its runtime, the sugar rules and the compiler extensions\n'
  'def run_core(): text := folder_of(run_library()) + "/core"\n', 1),
 ("core/parser.warp", 'read(run_library() + "/compiler/" + ENTITY_FILE)', 'read(run_core() + "/" + ENTITY_FILE)', 1),
 ("core/diagnostics.warp", 'global message_folder = run_library() + "/compiler"', 'global message_folder = run_core()', 1),
 ("core/compiler.warp", "# lib/: the fourth argument (`warp pure` gives it), else WARP_LIB, else the folder above this file's\n"
  "library_folder = if #args > 3 then args#4 else run_library()\n",
  "# lib/: the fourth argument (`warp pure` gives it), else WARP_LIB, else the one beside this file's folder\n"
  "library_folder = if #args > 3 then args#4 else run_library()\n"
  "# core/ beside that lib/: the compiler's own files\n"
  'core_folder = folder_of(library_folder) + "/core"\n', 1),
 ("core/compiler.warp", 'messages_from(library_folder + "/compiler")', "messages_from(core_folder)", 1),
 ("core/compiler.warp", 'lowered_program(read(library_folder + "/compiler/runtime.warp"), library_folder + "/compiler", prelude_source)',
  'lowered_program(read(core_folder + "/runtime.warp"), core_folder, prelude_source)', 1),
 ("core/uses.warp", "# lib/<name>.warp, a standard module's file (the std modules are the files of lib/, the folder above this one)\n"
  "std_module_path(name: text): text := run_library() + \"/\" + name + module_extension\n",
  "# lib/<name>.warp, a standard module's file (the std modules are the files of lib/, beside core/)\n"
  "std_module_path(name: text): text := run_library() + \"/\" + name + module_extension\n"
  "# core/<name>.warp, a file of the compiler's own folder: the sugar rules (`sugar/ranges`)\n"
  "core_module_path(name: text): text := run_core() + \"/\" + name + module_extension\n", 1),
 ("core/rewrites.warp", 'module_rules_of(file_text(std_module_path(module)) ?? "", "")', 'module_rules_of(file_text(core_module_path(module)) ?? "", "")', 1),
 # --- Rust: core/ beside lib/ marks a checkout; stage programs, the build file and the hash read it ---
 ("src/modules/program_file.rs",
  "/// The folder of lib/ that marks a warp checkout's library: the warp-written compiler\npub const COMPILER_FOLDER: &str = \"compiler\";\n",
  "/// The folder beside lib/ that marks a warp checkout: the warp-written compiler, its runtime, the sugar rules and the\n"
  "/// compiler extensions (card compiler-to-core)\npub const CORE_FOLDER: &str = \"core\";\n", 1),
 ("src/modules/program_file.rs", "candidate.join(STD_FOLDER).join(COMPILER_FOLDER).is_dir()", "candidate.join(CORE_FOLDER).is_dir()", 1),
 ("src/modules/program_file.rs", ".find(|library| library.join(COMPILER_FOLDER).is_dir());", ".find(|library| core_of(library).is_dir());", 1),
 ("src/modules/program_file.rs", ".filter(|library| library.join(COMPILER_FOLDER).is_dir())", ".filter(|library| core_of(library).is_dir())", 1),
 ("src/modules/program_file.rs", "/// the nearest lib/ holding compiler/ above", "/// the nearest lib/ with core/ beside it above", 1),
 ("src/modules/program_file.rs", "pub(crate) fn folder_of(file: &Path) -> PathBuf {",
  "/// core/ beside `library`, a lib/: <checkout>/core for <checkout>/lib\n"
  "pub fn core_of(library: &Path) -> PathBuf {\n"
  "\tlet library = std::path::absolute(library).unwrap_or_else(|_| library.to_path_buf());\n"
  "\tlibrary.parent().unwrap_or(Path::new(\"/\")).join(CORE_FOLDER)\n}\n\n"
  "pub(crate) fn folder_of(file: &Path) -> PathBuf {", 1),
 ("src/compiler/self_hosted.rs", '"use \\"{lib}/compiler/', '"use \\"{core}/', 3),
 ("src/compiler/self_hosted.rs", '/// Where a stage program names lib/, the folder library() resolves\nconst LIBRARY_MARK: &str = "{lib}";\n',
  '/// Where a stage program names lib/, the folder library() resolves\nconst LIBRARY_MARK: &str = "{lib}";\n'
  '/// Where a stage program names core/, the compiler\'s folder beside lib/ (compiler_folder)\nconst CORE_MARK: &str = "{core}";\n', 1),
 ("src/compiler/self_hosted.rs", "\t\tself.program.replace(LIBRARY_MARK, &library().display().to_string())\n",
  "\t\tlet program = self.program.replace(CORE_MARK, &compiler_folder().display().to_string());\n"
  "\t\tprogram.replace(LIBRARY_MARK, &library().display().to_string())\n", 1),
 ("src/compiler/self_hosted.rs", "\tlibrary().join(crate::modules::COMPILER_FOLDER)\n", "\tcrate::modules::core_of(library())\n", 1),
 ("src/compiler/self_hosted.rs", "\tlet program = program.replace(&library.display().to_string(), LIBRARY_MARK);\n",
  "\tlet program = program.replace(&compiler_folder().display().to_string(), CORE_MARK);\n"
  "\tlet program = program.replace(&library.display().to_string(), LIBRARY_MARK);\n", 1),
 ("src/compiler/self_hosted.rs",
  "\tlet library = library();\n\tlet mut files: Vec<PathBuf> = files_below(library.to_path_buf());\n\tfiles.sort();\n\tfor file in files {\n"
  "\t\thasher.update(format!(\"{}\\n\", file.strip_prefix(library).unwrap_or(&file).display()));\n",
  "\tlet library = library();\n\tlet core = compiler_folder();\n"
  "\t// lib/ and core/ beside it, each file once: a symlink (lib/compiler, lib/sugar until step 3) is no second copy\n"
  "\tlet mut files: Vec<PathBuf> = files_below(library.to_path_buf());\n\tfiles.extend(files_below(core.clone()));\n\tfiles.sort();\n\tfor file in files {\n"
  "\t\tlet inside = file.strip_prefix(library).map(Path::to_path_buf);\n"
  "\t\tlet inside = inside.or_else(|_| file.strip_prefix(&core).map(|path| Path::new(\"../core\").join(path)));\n"
  "\t\thasher.update(format!(\"{}\\n\", inside.unwrap_or_else(|_| file.clone()).display()));\n", 1),
 ("src/compiler/self_hosted.rs",
  "\tentries.flat_map(|path| if path.is_dir() { files_below(path) } else { vec![path] }).collect()\n",
  "\tlet entries = entries.filter(|path| !path.is_symlink());\n"
  "\tentries.flat_map(|path| if path.is_dir() { files_below(path) } else { vec![path] }).collect()\n", 1),
 ("src/compiler/standalone.rs", 'const COMPILER_FILE: &str = "compiler/compiler.warp";', 'const COMPILER_FILE: &str = "compiler.warp";', 1),
 ("src/compiler/standalone.rs", "library().join(COMPILER_FILE)", "crate::modules::core_of(library()).join(COMPILER_FILE)", 2),
 ("crates/warp-runtime/src/warp_host/main.rs", '.find(|lib| lib.join("compiler").is_dir())', '.find(|lib| core_beside(lib).is_dir())', 2),
 ("crates/warp-runtime/src/warp_host/main.rs", '"no lib/ with compiler/ found: WARP_LIB names one"', '"no lib/ with core/ beside it found: WARP_LIB names one"', 1),
 ("crates/warp-runtime/src/warp_host/main.rs", "cache::fingerprint((&bytes, cache::folder_fingerprint(&lib)))",
  "cache::fingerprint((&bytes, cache::folder_fingerprint(&lib), cache::folder_fingerprint(&core_beside(&lib))))", 1),
 ("crates/warp-runtime/src/warp_host/main.rs", "/// lib/ for compiler.warp, as warp-rust finds it: WARP_LIB, else the nearest lib/ holding compiler/ above",
  "/// core/ beside a lib/: the compiler's own folder (card compiler-to-core)\n"
  "fn core_beside(lib: &Path) -> PathBuf {\n\tstd::path::absolute(lib).unwrap_or_else(|_| lib.to_path_buf()).with_file_name(\"core\")\n}\n\n"
  "/// lib/ for compiler.warp, as warp-rust finds it: WARP_LIB, else the nearest lib/ with core/ beside it above", 1),
 ("Cargo.toml", '"lib/**", "README.md"', '"lib/**", "core/**", "README.md"', 1),
 # --- packaging and CI: core/ ships beside lib/ ---
 ("packaging/bin/warp", '[ -d "$folder/compiler" ] && echo "$folder"', '[ -d "$folder/../core" ] && echo "$folder"', 1),
 ("packaging/bin/warp", "no lib/ with compiler/ found", "no lib/ with core/ beside it found", 1),
 ("packaging/bin/warp", "the nearest lib/ holding compiler/ above", "the nearest lib/ with core/ beside it above", 1),
 (".github/workflows/release.yml", "cp -R lib stage/lib\n", "cp -R lib stage/lib && cp -R core stage/core\n", 1),
 (".github/workflows/release.yml", "cp -R lib dist/lib\n", "cp -R lib dist/lib && cp -R core dist/core\n", 1),
 (".github/workflows/release.yml", "compiler-self.wasm lib host-cache", "compiler-self.wasm lib core host-cache", 1),
 ("packaging/homebrew/Formula/warp.rb", '"compiler-self.wasm", "lib" if', '"compiler-self.wasm", "lib", "core" if', 1),
 ("packaging/linux/nfpm.yaml", "  - src: ./lib\n    dst: /usr/share/warp/lib\n    type: tree\n",
  "  - src: ./lib\n    dst: /usr/share/warp/lib\n    type: tree\n  - src: ./core\n    dst: /usr/share/warp/core\n    type: tree\n", 1),
 ("probes/emitter_split.py", 'REPOSITORY / "lib" / "compiler"', 'REPOSITORY / "core"', 1),
 # --- tests: path strings only (a copy of lib/ now sits in a folder of its own, its core/ beside it) ---
 ("tests/parser/test_self_pure.rs", 'join(format!("{name}_{}", std::process::id()))', 'join(format!("{name}_{}/lib", std::process::id()))', 1),
 ("tests/parser/test_self_pure.rs", 'linked_folder(&shipped.join("compiler"), &library.join("compiler"), "emitter.warp");',
  'linked_folder(&shipped.join("../core"), &library.join("../core"), "emitter.warp");', 1),
 ("tests/parser/test_self_pure.rs", 'std::fs::write(library.join("compiler/emitter.warp"), emitter)', 'std::fs::write(library.join("../core/emitter.warp"), emitter)', 1),
 ("tests/parser/test_self_pure.rs", 'output.contains("compiler/emitter.warp failed")', 'output.contains("core/emitter.warp failed")', 1),
 # --- the layout note: the flip happened (common.md path strings, asked by the user through Supervisor) ---
 ("notes/agents/common.md",
  "Until the flip, existing paths are still named lib/compiler/... and\n  lib/sugar/... (core is a symlink to lib/compiler, lib/compiler/sugar one to lib/sugar).\n",
  "Since the flip the files live in core/; until step 3 retires them,\n  lib/compiler -> ../core and lib/sugar -> ../core/sugar remain as compatibility symlinks.\n", 1),
]
bad = 0
for file, old, new, count in EDITS:
    text = open(file).read()
    found = text.count(old)
    if new in text and (found == 0 or old in new):
        print(f"done  {file}: already edited")
    elif found == count:
        open(file, "w").write(text.replace(old, new)); print(f"edit  {file}: {count}x")
        open(os.environ["TOUCHED"], "a").write(file + "\n")
    else:
        print(f"FAIL  {file}: {found}x (expected {count}) of {old[:70]!r}", file=sys.stderr); bad += 1
sys.exit(1 if bad else 0)
PY

# 3. the path text, in the reviewed files only (paths as after the move)
FILES=(
	crates/warp-runtime/src/gc_values.rs
	crates/warp-runtime/src/warp_host/main.rs
	crates/warp-runtime/src/warp_host/nodes.rs
	lib/aliases.warp
	core/compiler.warp
	core/definitions.warp
	core/diagnostics.warp
	core/emitter.warp
	core/errors.data
	core/hints.data
	core/library_calls.warp
	core/list_methods.warp
	core/loop_heads.warp
	core/parser.warp
	core/passes.warp
	core/plain_enums.warp
	core/prefix_calls.warp
	core/rewrites.warp
	core/runtime.warp
	core/runtime/intrinsics.warp
	core/runtime/library_words.warp
	core/runtime/nodes.warp
	core/shapes.warp
	core/warnings.data
	core/sugar/ranges.warp
	lib/time.warp
	notes.md
	notes/agents/common.md
	notes/agents/integrator.md
	notes/aliases.md
	notes/beautify_round.md
	notes/c_type_names.md
	notes/classes.md
	notes/compiler_enums.md
	notes/compiler_rewrite.md
	notes/dates_at_run_time.md
	notes/decisions.md
	notes/fixer_log.md
	notes/folds.md
	notes/footguns.md
	notes/host_from_warp_eval.md
	notes/hybrid_lane.md
	notes/index_position.md
	notes/layers.md
	notes/matching.md
	notes/node_cases.md
	notes/node_refinements.md
	notes/open_decisions.md
	notes/optional_result.md
	notes/parse_word.md
	notes/parser_compiles.md
	notes/passes.md
	notes/performance.md
	notes/postmortem_pure_kinds.md
	notes/pure_diagnostics.md
	notes/pure_wrapper.md
	notes/rewrites.md
	notes/roles.md
	notes/run_time_empty.md
	notes/rust_cleanup.md
	notes/rust_hacks.md
	notes/rust_reading.md
	notes/self_hosting.md
	notes/self_hosting_dropped.md
	notes/slow_redundancy.md
	notes/standalone.md
	notes/standalone_compiler.md
	notes/text_numbers.md
	notes/typed_builtins.md
	notes/typed_compiler.md
	notes/warp_host.md
	notes/web_playground.md
	packaging/bin/warp
	packaging/host_cache.sh
	probes/analyzer/kinds_of.warp
	probes/assigned_phrases/parsed.warp
	probes/broadcasting_check.warp
	probes/class_methods/lowered.warp
	probes/colon_forms/views.warp
	probes/comprehensions/if_view.warp
	probes/comprehensions/line_widths.warp
	probes/comprehensions/views.warp
	probes/constants/kinds_in_rust_lane.warp
	probes/crash_card_stale_build.sh
	probes/elvis/lowered.warp
	probes/emitter_split.py
	probes/emitter_split_check.sh
	probes/enum_leftovers/views.warp
	probes/errors/pure_parse_error_message.sh
	probes/fold_lowered_probe.warp
	probes/functions/bottom_up.warp
	probes/global_from_used_file.warp
	probes/host_from_warp/driver.warp
	probes/index_of_bytes/lowered_view.warp
	probes/keyword_rewrites/twin_receivers.warp
	probes/keyword_rewrites/twin_walk.warp
	probes/local_named_like_function.warp
	probes/loop_forms/parsed.warp
	probes/lower_compiler.warp
	probes/lower_program.warp
	probes/match_index_scrutinee.warp
	probes/match_or_fallback.warp
	probes/module_name_clash/uses_compiler.warp
	probes/module_name_clash/uses_directly.warp
	probes/node_patterns.warp
	probes/of_lookups_check.warp
	probes/parser_reads_day/views.warp
	probes/parser_speed/gaps_parts.warp
	probes/parser_speed/list_index.warp
	probes/parser_speed/name_chars.warp
	probes/parser_speed/name_parts.warp
	probes/parser_speed/phases.warp
	probes/parser_speed/pure_analysis_stage.warp
	probes/parser_speed/pure_passes_stage.warp
	probes/parser_speed/stages.warp
	probes/parser_speed/text_length.warp
	probes/perf/parse_emitter.warp
	probes/program_variable_in_module.warp
	probes/pure_enums/pass.warp
	probes/pure_lambdas/show.warp
	probes/pure_speed/entities_same.warp
	probes/pure_speed/entities_speed.warp
	probes/scope_check.warp
	probes/self_hosting/analyzer/twin_loop.py
	probes/self_hosting/comment_apostrophe_lowering.warp
	probes/self_hosting/emitter/core/node_cases.warp
	probes/self_hosting/emitter/core/pending_branches.warp
	probes/self_hosting/emitter/emit_lowered.sh
	probes/self_hosting/enum_cases.warp
	probes/self_hosting/enum_passes.warp
	probes/self_hosting/explicit_empty_tree.warp
	probes/self_hosting/float_text.warp
	probes/self_hosting/for_return_node_block.warp
	probes/self_hosting/hash_comment_spans.warp
	probes/self_hosting/lexer_shader.warp
	probes/self_hosting/lexer_spans.warp
	probes/self_hosting/main_n_beside_parser.warp
	probes/self_hosting/match_data_tree.warp
	probes/self_hosting/method_calls.warp
	probes/self_hosting/next_phase/view_pass.warp
	probes/self_hosting/number_keys.warp
	probes/self_hosting/number_words_twin.warp
	probes/self_hosting/one_compiler.warp
	probes/self_hosting/parse_failures.warp
	probes/self_hosting/parse_refusals.warp
	probes/self_hosting/parser/compare_forms.sh
	probes/self_hosting/parser/parse_forms.warp
	probes/self_hosting/parser/superscripts.warp
	probes/self_hosting/parser/trailing_hash.warp
	probes/self_hosting/parser_compiles.warp
	probes/self_hosting/plain_node_twin.warp
	probes/self_hosting/positioned_report.warp
	probes/self_hosting/readiness/ports/planned/walk.warp
	probes/self_hosting/readiness/walk_speed.warp
	probes/self_hosting/refusals.sh
	probes/self_hosting/replace_text_index_bug.warp
	probes/self_hosting/runtime_lowered.warp
	probes/self_hosting/serializer_views.warp
	probes/self_hosting/sort_bang_tree.warp
	probes/shalom_units.warp
	probes/speed/compiler_build.sh
	probes/speed/joined_runtime.sh
	probes/spelled_case_per_function.warp
	probes/stage_cache.sh
	probes/stage_pair.warp
	probes/star_params/views.warp
	probes/sugar/odd_assignments.warp
	probes/types/signature_shapes.py
	probes/types_front/declared_ops.warp
	probes/types_front/lowered_hello.warp
	probes/types_front/passes_one_by_one.warp
	probes/wasmtime_self_emitted.warp
	scripts/case_tables.py
	scripts/lib_bench.sh
	scripts/lib_bench_parse.warp
	scripts/self-hosted-suite.sh
	src/analyzer/inferred_kinds.rs
	src/analyzer/user_functions/flow.rs
	src/compiler/node_views.rs
	src/compiler/self_hosted.rs
	src/compiler/standalone.rs
	src/errors/crash_card.rs
	src/lowering/collections/broadcasting.rs
	src/lowering/collections/index_positions.rs
	src/lowering/collections/shared_arrays.rs
	src/lowering/definitions/lambdas.rs
	src/lowering/foreign/foreign_lines.rs
	src/lowering/types/node_cases.rs
	src/lowering/types/type_name_matching.rs
	src/lowering/words/keyword_rewrites.rs
	src/lowering/words/library_words.rs
	src/lowering/words/program_names.rs
	src/main.rs
	src/modules.rs
	src/modules/rewrites.rs
	src/text/uniscript_entities.rs
	src/warp_parser/declaration_scan.rs
	src/warp_parser/sugar.rs
	src/warp_parser/types/expressions.rs
	src/wasm_emitter/lists/list_dispatch/typed_list_finding.rs
	src/wasm_emitter/lists/list_emitter.rs
	src/wasm_emitter/lists/list_ops/fields_and_keys.rs
	src/wasm_emitter/lists/list_ops/node_arithmetic.rs
	tests/common/twins.rs
	tests/ffi/test_wasmtime_ffi.rs
	tests/functions/test_bodiless_runtime_words.rs
	tests/launcher/test_launcher.sh
	tests/lists/test_comprehension_rules.warp
	tests/modules/test_rewrite_engine.warp
	tests/modules/test_rewrite_zip_runs.warp
	tests/modules/test_rule_conditions.warp
	tests/node/test_node_cases.rs
	tests/node/test_parsed_node_walks.pairs.warp
	tests/node/test_parsed_node_walks.rs
	tests/numbers/test_duration_words.rs
	tests/operators/test_negated_name_before_operator_word.rs
	tests/parser/mod.rs
	tests/parser/test_element_wise_phrase.rs
	tests/parser/test_entity_table.rs
	tests/parser/test_self_analyzer.rs
	tests/parser/test_self_emitter.rs
	tests/parser/test_self_emitter_times.rs
	tests/parser/test_self_exact_pow.rs
	tests/parser/test_self_float_as_exact.rs
	tests/parser/test_self_hosted_library.rs
	tests/parser/test_self_pure.rs
	tests/parser/test_self_sort.rs
	tests/parser/test_self_text_index_of.rs
	tests/parser/test_self_time_arithmetic.rs
	tests/parser/test_times_phrase.rs
	tests/parser/test_warp_pure.rs
	tests/scope/test_bound_names_shadow_functions.rs
	tests/types/test_fields_of_later_types.rs
	web/playground/build.sh
	web/playground/completion.js
)
missing=()
for file in "${FILES[@]}"; do [ -f "$file" ] || missing+=("$file"); done
if [ ${#missing[@]} -gt 0 ]; then printf 'move-compiler-to-core: not found (renamed or deleted since review): %s\n' "${missing[@]}" >&2; fi
present=()
for file in "${FILES[@]}"; do [ -f "$file" ] && present+=("$file"); done
perl -pi -e 'next if /compatibility symlink/; s#lib/compiler/#core/#g; s#lib/compiler\b#core#g; s#lib/sugar/#core/sugar/#g; s#lib/sugar\b#core/sugar#g' "${present[@]}"
git add -- "${present[@]}"
[ -s "$touched" ] && tr '\n' '\0' < "$touched" | xargs -0 git add --
rm -f "$touched"

# 4. nothing names the old place outside the allowlist
check
