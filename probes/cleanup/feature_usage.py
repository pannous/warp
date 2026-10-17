"""How much the lowering features are used: per feature its src/lowering lines, and the samples, tour examples, guide
pages and wiki pages whose text matches the feature's syntax (an approximate regex each).
Usage: python3 probes/cleanup/feature_usage.py <checkout root> <wiki dir>"""
import glob
import os
import re
import sys

# feature, lowering files (relative to src/lowering, a folder counts all its files), syntax regex
FEATURES = [
	("compile-time lambdas", ["lambdas.rs"], r"=>"),
	("function values at compile time", ["function_values.rs"], r"\b\w+\(\s*\w+\s*,[^)]*\)\s*:=\s*\w+\("),
	("run-time closures", ["closures.rs"], r"=>"),
	("generators", ["generators.rs", "generator_objects.rs", "generator_consumers.rs"], r"\byield\b"),
	("ruby blocks", ["ruby_blocks.rs"], r"\{\s*\|\w+\|"),
	("return-type overloads", ["overloads.rs"], r"->\s*\w+[^\n]*\n[^\n]*->"),
	("multiple dispatch by class", ["traits/generics.rs"], r"\(\w+\s*:\s*[A-Z]\w*\s*,\s*\w+\s*:\s*[A-Z]"),
	("definitions by cases", ["definition_cases.rs"], r"\b\w+\(\d+\)\s*:="),
	("type-name parameters", ["type_name_matching.rs"], r"\b\w+\((int|float|text|string)\b"),
	("memoization", ["memoization.rs"], r"fib"),
	("constant folding", ["folding.rs"], r"."),
	("inlining", ["inlining.rs"], r"."),
	("lazy ranges", ["lazy_ranges.rs"], r"\d+\s*\.\.\.?\s*\d+"),
	("gpu maps", ["gpu_maps.rs"], r"@gpu"),
	("shared arrays", ["shared_arrays.rs"], r"\bshared\b"),
	("data parallelism", ["parallel.rs"], r"\bgo\s+(map|for|each)\b|\.map\([^)]*\)\s*@parallel|\bparallel\b"),
	("go blocks", ["go_blocks.rs"], r"\bgo\s*\{"),
	("channels", ["channel_words.rs"], r"\bchannel\b"),
	("tuple returns", ["tuples.rs"], r"\breturn\s+\w+\s*,\s*\w+|^\s*\w+\s*,\s*\w+\s*=\s*\w+\("),
	("named patterns", ["named_patterns.rs"], r"^\s*pattern\s+\w+"),
	("tree walks", ["tree_walks.rs"], r"\bwalk\b|\bdeep\b"),
	("exhaustive matches", ["match_exhaustive.rs"], r"\bmatch\b"),
	("data references $a @1", ["references.rs"], r"\$[a-z]\w*|@\d\b"),
	("meta annotations", ["meta_entries.rs"], r"@\w+\("),
	("version literals", ["versions.rs"], r"\bv?\d+\.\d+\.\d+\b"),
	("kebab names", ["kebab_names.rs"], r"\b[a-z]+-[a-z]+\s*:"),
	("index and position ints", ["index_positions.rs"], r"\bposition\b"),
	("number keys on {}", ["number_keys.rs"], r"=\s*\{\}"),
	("music words", ["music_words.rs"], r"\b[A-G][#b]?[0-9]\b"),
	("nonlocal cells", ["nonlocal_cells.rs"], r"\bnonlocal\b"),
	("scoped handlers", ["scoped_handlers.rs"], r"\}\s*in\s*\{"),
	("mutating fn!", ["mutation.rs"], r"\w!\s*\("),
	("charged late binding", ["late_binding.rs"], r":="),
	("getters", ["getters.rs"], r"^\s*\w+\s*:=\s*[^{(\n]"),
	("pipes", ["pipes.rs"], r"\s\|\s*\w+"),
	("rest parameters, spread", ["variadic.rs"], r"\.\.\.\w+|\w+\.\.\."),
	("named arguments", ["named_arguments.rs"], r"\b\w+\(\s*\w+\s*=\s*[^=]"),
	("object-shaped parameters", ["parameter_shapes.rs"], r"\(\s*\w+\{\w+"),
	("database tables (ORM)", ["database_tables.rs", "database_tables"], r"\bdatabase\b"),
	("web server", ["serve.rs"], r"\bserve\b"),
	("page routes", ["routes.rs"], r"\broute\b"),
	("components with state", ["component_state.rs"], r"\bcomponent\b|^[A-Z]\w*\s*:=\s*\w*\s*\{"),
	("element events", ["element_events.rs"], r"\bon (click|input|submit)\b"),
	("markup tags", ["markup_tags.rs"], r"\b(div|span|button|ul|li|html)\s*[({]"),
	("css transitions", ["transitions.rs"], r"\btransition\b"),
	("css style rules", ["style_rules.rs"], r"\bstyle\s*\{"),
	("component worlds", ["component_worlds.rs"], r"\bworld\b"),
	("min max builtins", ["min_max.rs"], r"\b(min|max)\("),
	("comprehensions", ["comprehensions.rs"], r"\[[^\]]*\bfor\b[^\]]*\bin\b"),
	("type constructors P(1,2)", ["type_constructor.rs"], r"\b[A-Z]\w*\(\d"),
	("plain enums", ["plain_enums.rs", "sum_variants.rs"], r"\benum\b"),
	("printable types", ["printable.rs"], r"\bto_text\b|\bshow\s*\("),
	("blocks never run warnings", ["run_time_blocks.rs"], r"."),
	("soft keywords", ["soft_keywords.rs"], r"."),
	("list element checks", ["list_element_checks.rs"], r":\s*\[\w+\]\s*="),
	("ambiguity questions", ["ambiguous_forms.rs"], r"."),
	("reflection dot words", ["reflection.rs"], r"\.(effects|params|parameters|source|signature)\b"),
	("type tests", ["type_tests.rs"], r"\bis\s+(a|an|int|float|text|list|pair|number)\b"),
	("class inheritance", ["class_methods/inheritance.rs"], r"\bextends\b|\binclude\b"),
	("instance destructuring", ["class_methods/destructuring.rs"], r"\{\s*\w+\s*,\s*\w+\s*\}\s*=\s*\w+"),
	("operators on instances", ["class_methods/operators.rs"], r"\bplus\s*\(|\boperator\b"),
	("comprehension-free map/filter words", ["library_words.rs", "library_words"], r"\.(map|filter|sum|first|upper)\b"),
	("broadcasting", ["broadcasting.rs"], r"\b\w+\s+\[\d"),
	("enum objects", ["declarations.rs"], r"\benum\b"),
	("traits", ["traits.rs", "traits/dispatch.rs", "traits/instance_types.rs"], r"\btrait\b|\binterface\b|\bprotocol\b"),
	("tasks", ["declarations/tasks.rs"], r"\bawait\b|\bgo\b"),
	("switch / match", ["switch.rs"], r"\b(switch|match)\b"),
	("for loops", ["for_loop.rs", "for_loop"], r"\bfor\b"),
	("blocks (charged)", ["blocks.rs"], r"\{"),
]


def lines_of(lowering, files):
	total = 0
	for name in files:
		path = os.path.join(lowering, name)
		paths = glob.glob(os.path.join(path, "**", "*.rs"), recursive=True) if os.path.isdir(path) else [path]
		total += sum(sum(1 for _ in open(each)) for each in paths)
	return total


def texts(pattern):
	return {path: open(path, errors="replace").read() for path in glob.glob(pattern, recursive=True)}


def main(root, wiki):
	sources = {
		"samples": texts(os.path.join(root, "samples", "**", "*.warp")),
		"guide": texts(os.path.join(root, "web", "playground", "guide*.md")),
		"wiki": texts(os.path.join(wiki, "*.md")),
	}
	tour = open(os.path.join(root, "web", "playground", "examples.js"), errors="replace").read()
	lowering = os.path.join(root, "src", "lowering")
	print(f"{'lines':>6} {'samples':>7} {'tour':>5} {'guide':>5} {'wiki':>5}  feature")
	rows = []
	for feature, files, syntax in FEATURES:
		pattern = re.compile(syntax, re.MULTILINE)
		counts = [sum(1 for text in sources[name].values() if pattern.search(text)) for name in ("samples", "guide", "wiki")]
		rows.append((lines_of(lowering, files), counts[0], len(pattern.findall(tour)), counts[1], counts[2], feature))
	for lines, samples, tour_uses, guide, wiki_pages, feature in sorted(rows, reverse=True):
		print(f"{lines:6} {samples:7} {tour_uses:5} {guide:5} {wiki_pages:5}  {feature}")


if __name__ == "__main__":
	main(sys.argv[1], sys.argv[2])
