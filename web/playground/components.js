// `use lib; lib.f(x)` (also `use lib.wasm`, long form `use wasm "lib.wasm" as lib`) in the page (src/components.rs natively): a WebAssembly component transpiled
// by jco (build.sh components → components/<name>.js, a classic script calling registerComponent with the core modules
// and the WIT signatures of its exports), loaded on its first call with importScripts, synchronously like every host
// call. Values cross as natively, by the WIT types: records objects with the WIT field names, variants
// `{case: payload}`, enums texts, chars codepoints `{$char: c}` (P94), flags lists of names, the `err` of a result an
// error, resources handles `{$handle, type, text, component}` with ids per component. Needs host.js.

const COMPONENTS_FOLDER = "components/";
const WASI_VERSION = /@[\d.]+$/;

const transpiled = new Map(); // by component name: {cores, signatures, instantiate}, as the scripts register them
const loadedComponents = new Map(); // by path: {path, functions, classes, signatures, held}

// signatures: {functions: {WIT name: {params: [names], result: type}}, types: [kind]} (wasm-tools component wit --json)
function registerComponent(name, cores, signatures, instantiate) {
	transpiled.set(name, { cores, signatures, instantiate });
}

const kebabCase = name => name.replace(/[A-Z]/g, (letter, at) => (at ? "-" : "") + letter.toLowerCase());
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
	const { cores, signatures, instantiate } = transpiled.get(name);
	const exports = instantiate(core => new WebAssembly.Module(bytesOfBase64(cores[core])), componentWasi(hooks));
	// the root's functions and classes and those of every interface; `warp:demo/text-tools` repeats `textTools`
	const [interfaces, root] = partition(Object.entries(exports).filter(([key]) => !key.includes(":")), ([, value]) => typeof value === "object");
	const members = [...root, ...interfaces.flatMap(([, value]) => Object.entries(value))];
	return {
		path,
		functions: members.filter(([, value]) => typeof value === "function" && !isClass(value)),
		classes: members.filter(([, value]) => isClass(value)),
		signatures,
		held: [],
	};
}

function componentOf(path, hooks) {
	if (!loadedComponents.has(path)) loadedComponents.set(path, loadComponent(path, hooks));
	return loadedComponents.get(path);
}

// what `member` names: a function, a resource's constructor (`counter(5)`, `Counter(5)`) or static function, with its
// WIT name (`stats-of`, `[constructor]counter`, `[static]counter.merged`)
function componentFunction(component, member) {
	const wanted = camelCase(member);
	const found = component.functions.find(([name]) => name === wanted);
	if (found) return { callee: found[1], wit: kebabCase(wanted) };
	const constructed = component.classes.find(([name]) => name.toLowerCase() === wanted.toLowerCase());
	if (constructed) return { callee: (...values) => new constructed[1](...values), wit: `[constructor]${kebabCase(constructed[0])}` };
	const owner = component.classes.find(([, value]) => typeof value[wanted] === "function");
	if (owner) return { callee: owner[1][wanted].bind(owner[1]), wit: `[static]${kebabCase(owner[0])}.${kebabCase(wanted)}` };
	const known = [...component.functions.map(([name]) => kebabCase(name)), ...component.classes.map(([name]) => `[constructor]${kebabCase(name)}`)];
	throw new Error(`the component exports no function ${member}; it exports ${known.join(", ")}`);
}

// a resource's method, for `handle.member(…)`
function componentMethod(resource, kind, member) {
	const method = resource[camelCase(member)];
	if (typeof method === "function") return { callee: method.bind(resource), wit: `[method]${kind}.${kebabCase(camelCase(member))}` };
	const known = Object.getOwnPropertyNames(Object.getPrototypeOf(resource)).filter(name => name !== "constructor").map(kebabCase);
	throw new Error(`a ${kind} has no method ${member}; it has ${known.join(", ")}`);
}

// a value warp gives as the component wants it: handles their resources again, record fields camelCase
function componentValue(value, component) {
	if (Array.isArray(value)) return value.map(item => componentValue(item, component));
	if (value === null || typeof value !== "object") return value;
	if ("$handle" in value) {
		if (value.component !== component.path) throw new Error(`${JSON.stringify(value)} is a handle of another component`);
		const resource = component.held[value.$handle - 1];
		if (!resource) throw new Error(`no handle ${value.$handle}`);
		return resource;
	}
	return Object.fromEntries(Object.entries(value).map(([key, item]) => [camelCase(key), componentValue(item, component)]));
}

function handleOfResource(resource, component) {
	const kind = kebabCase(resource.constructor.name);
	const id = component.held.push(resource);
	return { $handle: id, type: kind, text: `${kind}#${id}`, component: component.path };
}

// a component's value of the WIT type `type` (a primitive's name or an index into the signatures' types) as warp holds it
function plainOfComponent(value, type, component) {
	const kind = typeof type === "number" ? component.signatures.types[type] : type;
	const typed = (item, itemType) => plainOfComponent(item, itemType, component);
	if (kind === "char") return { $char: value };
	if (value === undefined) return null;
	if (kind?.type !== undefined) return typed(value, kind.type);
	if (kind?.list !== undefined) return Array.from(value, item => typed(item, kind.list));
	if (kind?.option !== undefined) return typed(value, kind.option);
	if (kind?.result !== undefined) return typed(value, kind.result.ok);
	if (kind?.tuple) return value.map((item, index) => typed(item, kind.tuple.types[index]));
	if (kind?.record) return Object.fromEntries(kind.record.fields.map(field => [field.name, typed(value[camelCase(field.name)], field.type)]));
	if (kind?.variant) {
		const payloadType = kind.variant.cases.find(variantCase => variantCase.name === value.tag)?.type;
		return payloadType === null || payloadType === undefined ? value.tag : { [value.tag]: typed(value.val, payloadType) };
	}
	if (kind?.flags) return kind.flags.flags.map(flag => flag.name).filter(name => value[camelCase(name)]);
	if (Array.isArray(value) || ArrayBuffer.isView(value)) return Array.from(value, item => typed(item, undefined));
	if (value !== null && typeof value === "object" && !isPlainObject(value)) return handleOfResource(value, component);
	return value;
}

registerForeignRuntime("wasm", {
	call(module, member, argumentValues, hooks) {
		const receiver = typeof module === "object" ? module.text : module;
		try {
			if (argumentValues === null) throw new Error(`${member} is a function of the component, call it: ${member}(…)`);
			const component = componentOf(typeof module === "object" ? module.component : module, hooks);
			const { callee, wit } = typeof module === "object"
				? componentMethod(componentValue(module, component), module.type, member)
				: componentFunction(component, member);
			const signature = component.signatures.functions[wit] ?? { params: [], result: undefined };
			const parameters = signature.params.filter(name => name !== "self");
			if (parameters.length !== argumentValues.length) throw new Error(`${member} takes ${parameters.length} arguments (${parameters.join(", ")}), got ${argumentValues.length}`);
			try {
				return plainOfComponent(callee(...componentValue(argumentValues, component)), signature.result, component);
			} catch (failure) {
				// the `err` of a result: jco throws its payload
				if (failure?.payload === undefined) throw failure;
				const errorType = component.signatures.types[signature.result]?.result?.err;
				const reason = plainOfComponent(failure.payload, errorType, component);
				throw new Error(typeof reason === "string" ? reason : JSON.stringify(reason));
			}
		} catch (failure) {
			throw new Error(`wasm ${receiver}: ${failure.message}`);
		}
	},
});
