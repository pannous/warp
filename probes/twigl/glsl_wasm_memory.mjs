// card twigl-browsers: glsl.wasm's calls grow its memory, which detaches a buffer read before the call
// ("draw.glsl_wgsl: attempting to access detached ArrayBuffer" in every browser). Runs the playground's glslWgsl
// (web/playground/host-gpu.js) in node over a long twigl body, so the translation grows memory.
// usage: node probes/twigl/glsl_wasm_memory.mjs <glsl.wasm> [host-gpu.js]
import { readFileSync } from "node:fs";

const source = readFileSync(process.argv[3] ?? new URL("../../web/playground/host-gpu.js", import.meta.url), "utf8");
const glslWgslSource = source.match(/function glslWgsl\(glsl\) \{[\s\S]*?\n\}/)[0];
const utf8 = new TextEncoder();
const getSync = () => readFileSync(process.argv[2]);
const glslWgsl = new Function("utf8", "getSync", "GLSL_URL", `let glslExports; return ${glslWgslSource}`)(utf8, getSync, "");

const line = "o.rgb += vec3(sin(FC.x * 0.01 + t), cos(FC.y * 0.02 - t), 0.5) * 0.001;\n";
const glsl = `#version 450\nvec4 twigl(vec2 FC, vec2 r, float t) {\nvec4 o = vec4(0.0);\n${line.repeat(Number(process.env.LINES ?? 4000))}return o;\n}\nvoid main() {}\n`;
const wgsl = glslWgsl(glsl);
console.log(wgsl.includes("fn twigl(") ? `ok: ${wgsl.length} chars of WGSL` : `unexpected: ${wgsl.slice(0, 200)}`);
