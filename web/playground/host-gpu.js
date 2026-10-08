// WebGPU (a part of host.js, which says how parts work; card web-apis, notes/web_framework.md "web-apis: WebGPU"):
// gpu_compute(shader, numbers, workgroups) runs a WGSL compute shader whose entry point `main` reads and writes the
// numbers as array<f32> at @group(0) @binding(0), dispatched over `workgroups` workgroups, and gives back the numbers it
// left. gpu_render(shader, width, height, values) runs the fragment shader `main` over every pixel of the image and gives
// the pixels as paint takes them, 0xFFRRGGBB row by row; the shader reads the map `values` as a uniform,
// `values.<name>` (src/gpu.rs does both natively). WebGPU only answers
// asynchronously, so a task Worker (host-tasks.js) does the GPU work while the program's worker waits for its answer in
// shared memory, as a task's result.

const GPU_ENTRY_POINT = "main";
const GPU_BINDING = 0;
// appended to gpu_render's shader (after it, so its line numbers stay the user's): one triangle covering the image
const FULL_IMAGE_VERTICES = `
@vertex fn warp_full_image(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4f {
	return vec4f(f32(corner / 2u) * 4.0 - 1.0, f32(corner % 2u) * 4.0 - 1.0, 0.0, 1.0);
}`;
const VERTEX_ENTRY_POINT = "warp_full_image";
const TRIANGLE_CORNERS = 3;
const PIXEL_BYTES = 4;
const ROW_ALIGNMENT = 256; // a copied row is padded to 256 bytes
const OPAQUE = 0xFF000000;
const VALUES_NAME = "values"; // gpu_render's values as the shader names them
const VALUES_BINDING = 0;
const UNIFORM_ALIGNMENT = 16; // a uniform struct is a whole number of 16-byte rows
// the WGSL type of a value of 1…4 floats and its alignment in bytes
const VALUE_TYPES = [, ["f32", 4], ["vec2f", 8], ["vec3f", 16], ["vec4f", 16]];
let gpuDevice; // the task Worker's device, asked for once
let gpuMapWarned = false;

// the task Worker's side: {values} the shader left, or {error} (no WebGPU, a shader that does not compile, …)
async function gpuComputed({ shader, numbers, workgroups }) {
	const device = gpuDevice ??= await gpuDeviceOrFailure();
	const input = new Float32Array(numbers);
	const usage = GPUBufferUsage;
	const storage = device.createBuffer({ size: input.byteLength, usage: usage.STORAGE | usage.COPY_SRC | usage.COPY_DST });
	const readback = device.createBuffer({ size: input.byteLength, usage: usage.MAP_READ | usage.COPY_DST });
	device.queue.writeBuffer(storage, 0, input);
	const module = await gpuModule(device, shader);
	device.pushErrorScope("validation");
	const pipeline = device.createComputePipeline({ layout: "auto", compute: { module, entryPoint: GPU_ENTRY_POINT } });
	const bindings = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: [{ binding: GPU_BINDING, resource: { buffer: storage } }] });
	const encoder = device.createCommandEncoder();
	const pass = encoder.beginComputePass();
	pass.setPipeline(pipeline);
	pass.setBindGroup(0, bindings);
	pass.dispatchWorkgroups(workgroups);
	pass.end();
	encoder.copyBufferToBuffer(storage, 0, readback, 0, input.byteLength);
	const values = Array.from(new Float32Array(await gpuReadBack(device, encoder, readback)));
	storage.destroy();
	return { values };
}

// {values}: the pixels the fragment shader colored, row by row
async function gpuRendered({ shader, width, height, values }) {
	const device = gpuDevice ??= await gpuDeviceOrFailure();
	const { declarations, bytes: valueBytes } = uniformLayout(values);
	const module = await gpuModule(device, shader + FULL_IMAGE_VERTICES + declarations);
	const uniform = valueBytes.byteLength ? uniformBinding(device, valueBytes) : undefined;
	const format = "rgba8unorm";
	const image = device.createTexture({ size: [width, height], format, usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
	const rowBytes = Math.ceil(width * PIXEL_BYTES / ROW_ALIGNMENT) * ROW_ALIGNMENT;
	const readback = device.createBuffer({ size: rowBytes * height, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });
	device.pushErrorScope("validation");
	const pipeline = device.createRenderPipeline({
		layout: uniform?.layout ?? "auto",
		vertex: { module, entryPoint: VERTEX_ENTRY_POINT },
		fragment: { module, entryPoint: GPU_ENTRY_POINT, targets: [{ format }] },
	});
	const encoder = device.createCommandEncoder();
	const pass = encoder.beginRenderPass({ colorAttachments: [{ view: image.createView(), loadOp: "clear", storeOp: "store" }] });
	pass.setPipeline(pipeline);
	if (uniform) pass.setBindGroup(0, uniform.bindings);
	pass.draw(TRIANGLE_CORNERS);
	pass.end();
	encoder.copyTextureToBuffer({ texture: image }, { buffer: readback, bytesPerRow: rowBytes, rowsPerImage: height }, [width, height]);
	const bytes = new Uint8Array(await gpuReadBack(device, encoder, readback));
	image.destroy();
	const pixels = [];
	for (let row = 0; row < height; row++)
		for (let at = row * rowBytes; at < row * rowBytes + width * PIXEL_BYTES; at += PIXEL_BYTES)
			pixels.push((OPAQUE | bytes[at] << 16 | bytes[at + 1] << 8 | bytes[at + 2]) >>> 0);
	return { values: pixels };
}

// the WGSL declaring `values` (appended, so the shader's line numbers stay the user's) and the uniform's bytes, laid out
// as WGSL aligns its members (src/gpu.rs uniform_layout)
function uniformLayout(values) {
	const entries = Object.entries(values ?? {});
	if (!entries.length) return { declarations: "", bytes: new ArrayBuffer(0) };
	const members = [], floats = [];
	for (const [name, value] of entries) {
		const numbers = [value].flat();
		if (!numbers.every(number => typeof number === "number")) throw new Error(`${VALUES_NAME}.${name} is a number or a list of numbers, got ${JSON.stringify(value)}`);
		const [type, alignment] = VALUE_TYPES[numbers.length] ?? [];
		if (!type) throw new Error(`${VALUES_NAME}.${name} is one number or a list of two to four, got ${numbers.length}`);
		while (floats.length % (alignment / 4)) floats.push(0);
		floats.push(...numbers);
		members.push(`${name}: ${type}`);
	}
	while (floats.length % (UNIFORM_ALIGNMENT / 4)) floats.push(0);
	const declarations = `\nstruct WarpValues { ${members.join(", ")} }\n@group(0) @binding(${VALUES_BINDING}) var<uniform> ${VALUES_NAME}: WarpValues;`;
	return { declarations, bytes: new Float32Array(floats).buffer };
}

// an explicit layout holding the uniform, so a shader that leaves `values` unread still binds them
function uniformBinding(device, bytes) {
	const buffer = device.createBuffer({ size: bytes.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
	device.queue.writeBuffer(buffer, 0, bytes);
	const groupLayout = device.createBindGroupLayout({ entries: [{ binding: VALUES_BINDING, visibility: GPUShaderStage.FRAGMENT, buffer: { type: "uniform" } }] });
	const layout = device.createPipelineLayout({ bindGroupLayouts: [groupLayout] });
	const bindings = device.createBindGroup({ layout: groupLayout, entries: [{ binding: VALUES_BINDING, resource: { buffer } }] });
	return { layout, bindings };
}

async function gpuDeviceOrFailure() {
	const adapter = await self.navigator.gpu?.requestAdapter();
	if (!adapter) throw new Error("this browser offers no WebGPU adapter");
	return adapter.requestDevice();
}

// the shader's module; one that does not compile throws where and why
async function gpuModule(device, shader) {
	const module = device.createShaderModule({ code: shader });
	const problems = (await module.getCompilationInfo()).messages.filter(message => message.type === "error");
	if (problems.length) throw new Error(problems.map(problem => `${problem.lineNum}:${problem.linePos}: ${problem.message}`).join("; "));
	return module;
}

// submit the encoded work (its validation scope pushed before the pipeline) and give the bytes it left in readback
async function gpuReadBack(device, encoder, readback) {
	device.queue.submit([encoder.finish()]);
	const invalid = await device.popErrorScope();
	if (invalid) throw new Error(invalid.message);
	await readback.mapAsync(GPUMapMode.READ);
	const bytes = readback.getMappedRange().slice(0);
	readback.destroy();
	return bytes;
}

const GPU_JOBS = { gpu_compute: gpuComputed, gpu_render: gpuRendered };

// a task Worker's job (task-worker.js): the shader's answer, or why not, into the shared buffer the program waits on
async function gpuJobInto({ gpu: { word, job }, shared }) {
	let answer;
	try {
		answer = await GPU_JOBS[word](job);
	} catch (failure) {
		answer = { error: String(failure.message ?? failure) };
	}
	writeShared(shared, answer);
}

// the program's side: hand the job to a task Worker and wait for the values it gives
function gpuJob(word, job) {
	if (!hasTaskWorkers()) throw new Error(`${word} needs task Workers, which only a cross-origin isolated page has (the playground)`);
	const worker = taskPool.pop();
	if (!worker) throw new Error(`${word}: every task Worker is busy`);
	const shared = new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
	worker.postMessage({ gpu: { word, job }, shared });
	const { values, error } = readShared(shared, true);
	taskPool.push(worker);
	if (error) throw new Error(`${word}: ${error}`);
	return values;
}

// the f64 cells of a linear array: its block is [count: i64][count cells] (src/wasm_emitter/linear_arrays.rs)
function linearCells(memory, block) {
	const count = Number(new DataView(memory.buffer).getBigInt64(Number(block), true));
	return new Float64Array(memory.buffer, Number(block) + Float64Array.BYTES_PER_ELEMENT, count);
}

addHostPart({
	words: (holder, hooks, { program }) => {
		const plain = node => plainOfTree(readNode(program(), node));
		const list = (values, item) => buildValue(program(), { kind: SQUARE_LIST, items: values.map(item) });
		const gpuKernelLinear = (shader, source, values, target, workgroups, reduces) => {
			// the items, then the values of the program's numbers the kernel reads, then a cell per workgroup to reduce into
			const numbers = [...linearCells(program().memory, source), ...[plain(values) ?? []].flat().map(Number)];
			const partials = numbers.length;
			if (reduces) numbers.push(...new Array(Number(workgroups)).fill(0));
			try {
				const left = gpuJob("gpu_compute", { shader: plain(shader), numbers, workgroups: Number(workgroups) });
				const cells = linearCells(program().memory, target);
				cells.set(left.slice(reduces ? partials : 0).slice(0, cells.length));
				return 1n;
			} catch (failure) {
				if (!/adapter|task Workers/.test(failure.message)) throw failure;
				if (!gpuMapWarned) console.warn(`@gpu: ${failure.message}, so the map runs on the CPU`);
				gpuMapWarned = true;
				return 0n;
			}
		};
		return {
			gpu_compute: (shader, numbers, workgroups) => {
				const values = gpuJob("gpu_compute", { shader: plain(shader), numbers: plain(numbers).map(Number), workgroups: Number(workgroups) });
				// floats, also the whole ones (treeOfPlain would make 3 an Int)
				return list(values, float => ({ kind: KIND_FLOAT, data: { float }, chain: [] }));
			},
			// over a `linear xs = float[n]`: its cells read and written in place, no list built (card gpu-vectors)
			gpu_compute_linear: (shader, block, workgroups) => {
				const cells = linearCells(program().memory, block);
				// a copy: a view would post all of memory
				cells.set(gpuJob("gpu_compute", { shader: plain(shader), numbers: cells.slice(), workgroups: Number(workgroups) }));
				return block;
			},
			// `ys = xs.map(x => …) @gpu` (src/lowering/gpu_maps.rs): the kernel over source's cells (and the values it reads)
			// into target's; 0 without an adapter (said once), when the program maps them on the CPU
			gpu_map_linear: (shader, source, values, target, workgroups) => gpuKernelLinear(shader, source, values, target, workgroups, false),
			// `s = sum(xs.map(x => …) @gpu)`, min, max: each workgroup's partial result, left after the values, into target
			gpu_reduce_linear: (shader, source, values, target, workgroups) => gpuKernelLinear(shader, source, values, target, workgroups, true),
			gpu_render: (shader, width, height, values) => {
				const given = values == null ? {} : plain(values);
				const pixels = gpuJob("gpu_render", { shader: plain(shader), width: Number(width), height: Number(height), values: given });
				return list(pixels, treeOfPlain);
			},
		};
	},
});
