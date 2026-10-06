// `use wasm "lib.wasm" as lib; lib.f(x)` in the page (src/components.rs natively): a WebAssembly component transpiled
// by jco (build.sh components → components/<name>.js, a classic script calling registerComponent), loaded on its first
// call with importScripts, synchronously like every host call. Values cross as natively: records objects with the WIT
// field names, variants `{case: payload}`, enums and chars texts, the `err` of a result an error, resources handles
// `{$handle, type, text, component}` with ids per component. Needs host.js.

const COMPONENTS_FOLDER = "components/";
const WASI_VERSION = /@[\d.]+$/;

const transpiled = new Map(); // by component name: {cores, instantiate}, as the scripts register them
const loadedComponents = new Map(); // by path: {exports, functions, classes, held}

function registerComponent(name, cores, instantiate) {
	transpiled.set(name, { cores, instantiate });
}

const kebabCase = name => name.replace(/[A-Z]/g, letter => "-" + letter.toLowerCase());
const camelCase = name => name.replace(/[-_]([a-z0-9])/g, (_, letter) => letter.toUpperCase());
const isClass = value => typeof value === "function" && /^class\b/.test(Function.prototype.toString.call(value));
const componentName = path => path.split("/").pop().replace(/\.wasm$/, "");
const partition = (items, test) => [items.filter(test), items.filter(item => !test(item))];
const bytesOfBase64 = text => Uint8Array.from(atob(text), character => character.charCodeAt(0));

// WASI p2 for a component's imports: its output is the program's, no input, no environment
function componentWasi(hooks) {
	class Pollable { ready() { return true; } block() {} }
	class InputStream {
		read() { throw { tag: "closed" }; }
		blockingRead() { throw { tag: "closed" }; }
		subscribe() { return new Pollable(); }
	}
	class OutputStream {
		constructor(stream) { this.stream = stream; }
		checkWrite() { return 1n << 20n; }
		write(bytes) { hooks.print(new TextDecoder().decode(bytes), this.stream); }
		blockingWriteAndFlush(bytes) { this.write(bytes); }
		flush() {}
		blockingFlush() {}
		subscribe() { return new Pollable(); }
	}
	class WasiError { toDebugString() { return "error"; } }
	class TerminalInput {}
	class TerminalOutput {}
	const interfaces = {
		"wasi:cli/environment": { getEnvironment: () => [], getArguments: () => [], initialCwd: () => undefined },
		"wasi:cli/exit": { exit: status => { throw new Error(`the component exited (${status.tag})`); } },
		"wasi:cli/stdin": { getStdin: () => new InputStream() },
		"wasi:cli/stdout": { getStdout: () => new OutputStream(1) },
		"wasi:cli/stderr": { getStderr: () => new OutputStream(STDERR) },
		"wasi:cli/terminal-input": { TerminalInput },
		"wasi:cli/terminal-output": { TerminalOutput },
		"wasi:cli/terminal-stdin": { getTerminalStdin: () => undefined },
		"wasi:cli/terminal-stdout": { getTerminalStdout: () => undefined },
		"wasi:cli/terminal-stderr": { getTerminalStderr: () => undefined },
		"wasi:io/error": { Error: WasiError },
		"wasi:io/poll": { Pollable, poll: pollables => new Uint32Array(pollables.map((_, index) => index)) },
		"wasi:io/streams": { InputStream, OutputStream },
	};
	// jco asks with and without the version (`wasi:cli/exit@0.2.6`)
	return new Proxy(interfaces, { get: (all, name) => all[String(name).replace(WASI_VERSION, "")] });
}

function loadComponent(path, hooks) {
	const name = componentName(path);
	if (!transpiled.has(name)) {
		try {
			importScripts(`${COMPONENTS_FOLDER}${name}.js`);
		} catch (failure) {
			throw new Error(`cannot load the component ${path}: the page has no ${COMPONENTS_FOLDER}${name}.js (web/playground/build.sh components transpiles it): ${failure.message}`);
		}
	}
	const { cores, instantiate } = transpiled.get(name);
	const exports = instantiate(core => new WebAssembly.Module(bytesOfBase64(cores[core])), componentWasi(hooks));
	// the root's functions and classes and those of every interface; `warp:demo/text-tools` repeats `textTools`
	const [interfaces, root] = partition(Object.entries(exports).filter(([key]) => !key.includes(":")), ([, value]) => typeof value === "object");
	const members = [...root, ...interfaces.flatMap(([, value]) => Object.entries(value))];
	return {
		functions: members.filter(([, value]) => typeof value === "function" && !isClass(value)),
		classes: members.filter(([, value]) => isClass(value)),
		held: [],
	};
}

function componentOf(path, hooks) {
	if (!loadedComponents.has(path)) loadedComponents.set(path, loadComponent(path, hooks));
	return loadedComponents.get(path);
}

// what `member` names: a function, a resource's constructor (`counter(5)`, `Counter(5)`) or static function
function componentFunction(component, member) {
	const wanted = camelCase(member);
	const found = component.functions.find(([name]) => name === wanted);
	if (found) return found[1];
	const constructed = component.classes.find(([name]) => name.toLowerCase() === wanted.toLowerCase());
	if (constructed) return Object.assign((...values) => new constructed[1](...values), { arity: constructed[1].length });
	const owner = component.classes.find(([, value]) => typeof value[wanted] === "function");
	if (owner) return owner[1][wanted].bind(owner[1]);
	const known = [...component.functions.map(([name]) => kebabCase(name)), ...component.classes.map(([name]) => `[constructor]${kebabCase(name).slice(1)}`)];
	throw new Error(`the component exports no function ${member}; it exports ${known.join(", ")}`);
}

// a resource's method, for `handle.member(…)`
function componentMethod(resource, kind, member) {
	const method = resource[camelCase(member)];
	if (typeof method === "function") return method.bind(resource);
	const known = Object.getOwnPropertyNames(Object.getPrototypeOf(resource)).filter(name => name !== "constructor").map(kebabCase);
	throw new Error(`a ${kind} has no method ${member}; it has ${known.join(", ")}`);
}

// a value warp gives as the component wants it: handles their resources again, record fields camelCase
function componentValue(value, path, held) {
	if (Array.isArray(value)) return value.map(item => componentValue(item, path, held));
	if (value === null || typeof value !== "object") return value;
	if ("$handle" in value) {
		if (value.component !== path) throw new Error(`${JSON.stringify(value)} is a handle of another component`);
		const resource = held[value.$handle - 1];
		if (!resource) throw new Error(`no handle ${value.$handle}`);
		return resource;
	}
	return Object.fromEntries(Object.entries(value).map(([key, item]) => [camelCase(key), componentValue(item, path, held)]));
}

// a component's value as warp holds it: resources handles, record fields kebab-case as in the WIT, `{tag, val}`
// variants `{case: payload}`
function plainOfComponent(value, path, held) {
	if (value === undefined) return null;
	if (Array.isArray(value) || ArrayBuffer.isView(value)) return Array.from(value, item => plainOfComponent(item, path, held));
	if (value === null || typeof value !== "object") return value;
	if (!isPlainObject(value)) {
		const kind = kebabCase(value.constructor.name).slice(1);
		const id = held.push(value);
		return { $handle: id, type: kind, text: `${kind}#${id}`, component: path };
	}
	const keys = Object.keys(value);
	if (typeof value.tag === "string" && keys.every(key => key === "tag" || key === "val")) {
		return "val" in value ? { [value.tag]: plainOfComponent(value.val, path, held) } : value.tag;
	}
	return Object.fromEntries(keys.map(key => [kebabCase(key), plainOfComponent(value[key], path, held)]));
}

registerForeignRuntime("wasm", {
	call(module, member, argumentValues, hooks) {
		const receiver = typeof module === "object" ? module.text : module;
		try {
			if (argumentValues === null) throw new Error(`${member} is a function of the component, call it: ${member}(…)`);
			const path = typeof module === "object" ? module.component : module;
			const component = componentOf(path, hooks);
			const callee = typeof module === "object"
				? componentMethod(componentValue(module, path, component.held), module.type, member)
				: componentFunction(component, member);
			const arity = callee.arity ?? callee.length;
			if (arity !== argumentValues.length) throw new Error(`${member} takes ${arity} arguments, got ${argumentValues.length}`);
			try {
				return plainOfComponent(callee(...componentValue(argumentValues, path, component.held)), path, component.held);
			} catch (failure) {
				// the `err` of a result: jco throws its payload
				if (failure?.payload === undefined) throw failure;
				throw new Error(typeof failure.payload === "string" ? failure.payload : JSON.stringify(plainOfComponent(failure.payload, path, component.held)));
			}
		} catch (failure) {
			throw new Error(`wasm ${receiver}: ${failure.message}`);
		}
	},
});
