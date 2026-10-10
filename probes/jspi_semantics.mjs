// What the page relies on from JSPI (card jspi-page), in node (V8): `node probes/jspi_semantics.mjs`
// A bare Suspending import: 1. suspends even for a plain value, so outside WebAssembly.promising it traps;
// 2. under promising a promise suspends the caller until it settles; 3. a JavaScript frame between traps.
// host-foreign.js awaitingCall (its trampoline module, taken from the file): 4. a plain value never suspends, also
// outside promising and below a JavaScript frame; 5. a promise under promising is awaited; 6. a promise below a
// JavaScript frame traps with a message naming the suspension (host.js SUSPENSION_FAILURE)
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const PROGRAM = `(module
	(import "host" "call" (func $call (param i32) (result i32)))
	(import "host" "callback" (func $callback (param i32) (result i32)))
	(import "host" "foreign_call" (func $foreign (param anyref anyref anyref anyref anyref) (result anyref)))
	(func (export "direct") (param i32) (result i32) (call $call (local.get 0)))
	(func (export "nested") (param i32) (result i32) (call $callback (local.get 0)))
	(func (export "foreign") (param anyref) (result anyref) (call $foreign (local.get 0) (ref.null any) (ref.null any) (ref.null any) (ref.null any))))`;
const bytes = execFileSync("wasm-tools", ["parse", "-"], { input: PROGRAM });

const source = readFileSync(new URL("../web/playground/host-foreign.js", import.meta.url), "utf8");
const trampoline = source.slice(source.indexOf("const AWAITING_CALL_WASM"), source.indexOf("addHostPart({\n\twords"));
const awaitingCall = new Function(`${trampoline}; return awaitingCall;`)();

let instance;
const exportsOf = () => instance.exports;
// a "foreign" value: a number of the program (an i31) stays, 0 gives a promise of 7
const foreignValue = value => value === 0 ? Promise.resolve(7) : value;
instance = new WebAssembly.Instance(new WebAssembly.Module(bytes), {
	host: {
		call: new WebAssembly.Suspending(value => value > 0 ? Promise.resolve(value * 10) : -value),
		callback: value => value > 100 ? exportsOf().foreign(value - 100) : exportsOf().direct(value), // a JavaScript frame between
		foreign_call: awaitingCall(foreignValue),
	},
});
const { direct, nested, foreign } = instance.exports;

const outcome = async run => { try { return await run(); } catch (failure) { return `${failure.constructor.name}: ${failure.message}`; } };
const cases = [
	["1 bare: plain value, no promising", () => direct(-4)],
	["2 bare: promise under promising", () => WebAssembly.promising(direct)(4)],
	["3 bare: JavaScript frame between", () => WebAssembly.promising(nested)(4)],
	["4 trampoline: plain value, no promising", () => foreign(5)],
	["4 trampoline: plain value below a JavaScript frame", () => WebAssembly.promising(nested)(105)],
	["5 trampoline: promise under promising", () => WebAssembly.promising(foreign)(0)],
	["6 trampoline: promise below a JavaScript frame", () => WebAssembly.promising(nested)(100)],
];
for (const [name, run] of cases) console.log(`${name}:`, await outcome(run));
