//! Call forms of other languages from probes/function_calls_round8.md: Elixir, PHP, C++, Ruby, Kotlin
use crate::is;
use warp::parse;

#[test]
fn elixir_one_line_def_and_capture() {
	is!("def sq(x), do: x * x; sq(3)", 9);
	is!("defp add(a, b), do: a + b; add(2, 3)", 5);
	is!("Enum.map([1, 2, 3], &(&1 * 2))", parse("[2 4 6]"));
	is!("f = &(&1 * &2); f(3, 4)", 12);
}

#[test]
fn php_array_functions() {
	is!("array_map(fn($x) => $x * 2, [1, 2, 3])", parse("[2 4 6]"));
	is!("array_filter([1, 2, 3], fn($x) => $x > 1)", parse("[2 3]"));
	is!("array_sum([1, 2, 3])", 6);
}

#[test]
fn cpp_auto_and_lambdas() {
	is!("auto sq = [](int x) { return x * x; }; sq(5)", 25);
	is!("f = [](int a, int b) { return a + b; }; f(2, 3)", 5);
	is!("sq = [&](int x) { return x * x; }; sq(4)", 16);
	is!("auto x = 3; x + 1", 4);
}

#[test]
fn ruby_symbol_blocks_methods_and_block_sums() {
	is!("xs = [1, 2, 3].map(&:to_s); xs#2 + \"!\"", "2!");
	is!("\"42\".to_i + 1", 43);
	is!("[1, 2, 3].sum { |x| x * 2 }", 12);
	is!("[1, 2, 3].count { |x| x > 1 }", 2);
	is!("xs = [1, 2, 3]; xs.sumOf { it * 2 }", 12);
}

/// card elixir-block: Elixir's block form `def sq(x) do … end`
#[test]
fn elixir_do_end_def() {
	is!("def sq(x) do x * x end; sq(3)", 9);
	is!("defp add(a, b) do\n  a + b\nend\nadd(2, 3)", 5);
}

/// card std-function: C++'s declared function type before a lambda, `std::function<int(int)> sq = [](int x) {…}`
#[test]
fn cpp_std_function_declaration() {
	is!("std::function<int(int)> sq = [](int x) { return x * x; }; sq(3)", 9);
	is!("function<int(int, int)> add = [](int a, int b) { return a + b; }; add(2, 3)", 5);
}
