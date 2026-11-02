// The compiler's words for a running program (a part of host.js, which says how parts work): code known only at run time,
// parsed or run by the page's warp.wasm, and the paths the runtime's file words open

addHostPart({
	words: (holder, hooks, { program }) => ({
		// the code of a text known only at run time: the page's warp.wasm parses it (driver.js codeParsed)
		parse_code: text => codeParsed(program(), text),
		// the value of a text run as a program of its own: the page's warp.wasm compiles it (driver.js codeEvaluated)
		eval_code: text => codeEvaluated(program(), text, hooks),
		// the path a file word opens, its links followed (core/runtime/extended/wasi.warp opened_entry): the browser's
		// files have no links, -1 says the path is itself
		real_path: () => -1,
	}),
});
