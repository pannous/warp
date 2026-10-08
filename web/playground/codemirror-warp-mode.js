// warp syntax for CodeMirror's simple mode, after ~/wasp/docs/codemirror-wasp-mode.js, with warp's comment rules:
// `//` and `/* */`, `#` followed by a space (`#s` is count, src/warp_parser.rs), `##` doc comments
const words = list => new RegExp(`(?:${list.join("|")})\\b`);

CodeMirror.defineSimpleMode("warp", {
	start: [
		{ regex: /\/\*/, token: "comment", next: "comment" },
		{ regex: /\/\/(?:\s.*|$)/, token: "comment" },
		{ regex: /#(?:[ \t#!].*|$)/, token: "comment" },
		{ regex: /"(?:[^"\\]|\\.)*"?/, token: "string" },
		{ regex: /'(?:[^'\\]|\\.)*'?/, token: "string" },
		{ regex: /“[^”]*”?|‘[^’]*’?|«[^»]*»?/, token: "string" },
		{ regex: /0x[0-9a-fA-F]+|\d+(?:\.\d*)?(?:[eE][+-]?\d+)?/, token: "number" },
		{ regex: words(["true", "false", "yes", "no", "nil", "null", "none", "it", "self", "this", "π", "τ", "ø"]), token: "atom" },
		{ regex: words(["int", "float", "real", "number", "text", "string", "bool", "byte", "char", "codepoint", "list", "rational"]), token: "type" },
		// P165: hard keywords are never redefined, soft ones may name a local (keywords.js, made by build.sh)
		{ regex: words(KEYWORDS.hard), token: "keyword" },
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
