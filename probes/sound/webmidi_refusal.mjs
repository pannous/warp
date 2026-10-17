// node probes/sound/webmidi_refusal.mjs: playground.js sendMidi explains Firefox's refusal and asks again after it
// (card webmidi-requires); the MIDI block is cut out of the page script and run with a stubbed navigator
import { readFileSync } from "node:fs";
const page = readFileSync(new URL("../../web/playground/playground.js", import.meta.url), "utf8");
const block = page.slice(page.indexOf("let midiAccess;"), page.indexOf("// `play \"song.mp3\"`"));
let asked = 0;
const sent = [], logged = [];
const refusal = Object.assign(new Error("WebMIDI requires a site permission add-on to activate"), { name: "SecurityError" });
const navigator = {
	requestMIDIAccess: () => ++asked === 1 ? Promise.reject(refusal) : Promise.resolve({ outputs: new Map([["a", { name: "synth", send: m => sent.push(m) }]]) }),
};
const console = { error: text => logged.push(text) };
const sendMidi = new Function("navigator", "console", "beforeUserActivation", `${block}; return sendMidi;`)(navigator, console, () => false);
await sendMidi("", [0x90, 60, 100]);
await sendMidi("", [0x90, 60, 100]);
const explained = logged.length === 1 && logged[0].includes("restart Firefox") && logged[0].includes("site permission add-on");
const ok = explained && asked === 2 && sent.length === 1;
globalThis.console.log(ok ? "ok" : "FAILED", { asked, sent, logged });
process.exit(ok ? 0 : 1);
