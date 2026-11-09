// warp syntax for CodeMirror's simple mode, after ~/wasp/docs/codemirror-wasp-mode.js, with warp's comment rules:
// `//` and `/* */`, `#` followed by a space (`#s` is count, src/warp_parser.rs), `##` doc comments
// no words match nothing: an empty alternation would match the empty string, and the mode would never advance
const NOTHING = /(?!)/;
const words = list => list.length ? new RegExp(`(?:${list.join("|")})\\b`) : NOTHING;
// a definition's name, shown bold (card film-color-syntax): the name before a head ending in `:=`, `film(color, x, y)
// := {`, `square x := x*x` (decision spaced-definition-head), `f(x): int := …`, `count := 0`; and the name after a
// definition word, `def f …`, `class Point {…}`
const NAME = "[\\p{L}_][\\p{L}\\p{N}_]*";
// (a spaced head's parameters taken with its name, so none of them reads as a name defined)
const DEFINED_HEAD = new RegExp(`(${NAME})((?:[ \\t]+${NAME})*)(?=(?:\\([^()]*\\))?[ \\t]*(?:(?::|->)[^=]*)?:=)`, "u");
const DEFINITION_WORDS = ["def", "fun", "fn", "func", "function", "define", "class", "enum"];

CodeMirror.defineSimpleMode("warp", {
	start: [
		{ regex: /\/\*/, token: "comment", next: "comment" },
		{ regex: /\/\/.*/, token: "comment" },
		{ regex: /#(?:[ \t#!].*|$)/, token: "comment" },
		{ regex: /"(?:[^"\\]|\\.)*"?/, token: "string" },
		{ regex: /'(?:[^'\\]|\\.)*'?/, token: "string" },
		{ regex: /“[^”]*”?|‘[^’]*’?|«[^»]*»?/, token: "string" },
		{ regex: /0x[0-9a-fA-F]+|\d+(?:\.\d*)?(?:[eE][+-]?\d+)?/, token: "number" },
		{ regex: words(["true", "false", "yes", "no", "nil", "null", "none", "it", "self", "this", "π", "τ", "ø"]), token: "atom" },
		{ regex: words(["int", "float", "real", "number", "text", "string", "bool", "byte", "char", "codepoint", "list", "rational"]), token: "type" },
		{ regex: new RegExp(`(${DEFINITION_WORDS.join("|")})([ \\t]+)(${NAME})`, "u"), token: ["keyword", null, "def"] },
		// P165: hard keywords are never redefined, soft ones may name a local (keywords.js, made by build.sh)
		{ regex: words(KEYWORDS.hard), token: "keyword" },
		{ regex: DEFINED_HEAD, token: ["def", "variable"] },
		// a call's name, right before its `(`, as editors colour function calls (TextMate's entity.name.function, card
		// call-highlight): `hash(x)`, also a soft keyword called so, `print(x)`
		{ regex: new RegExp(`${NAME}(?=\\()`, "u"), token: "call" },
		{ regex: words(KEYWORDS.soft), token: "soft-keyword" },
		{ regex: /\$[\w$]+/, token: "variable-2" },
		{ regex: /[-+\/*=<>!:….≥≤≠×÷^¬√∑²³#%&|~≈∈∉]+/, token: "operator" },
		{ regex: /[\{\[\(]/, indent: true },
		{ regex: /[\}\]\)]/, dedent: true },
		{ regex: /[\p{L}_][\p{L}\p{N}_]*/u, token: "variable" },
	],
	comment: [
		{ regex: /.*?\*\//, token: "comment", next: "start" },
		{ regex: /.*/, token: "comment" },
	],
	meta: { lineComment: "//", dontIndentStates: ["comment"] },
});
