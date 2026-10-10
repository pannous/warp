// WebGPU (a part of host.js, which says how parts work; card web-apis, notes/web_framework.md "web-apis: WebGPU"):
// gpu_compute(shader, numbers, workgroups) runs a WGSL compute shader whose entry point `main` reads and writes the
// numbers as array<f32> at @group(0) @binding(0), dispatched over `workgroups` workgroups, and gives back the numbers it
// left; given a map of named arrays, it binds each where the shader declares the storage array of that name and gives
// them back by name. gpu_render(shader, width, height, values) runs the fragment shader `main` over every pixel of the image and gives
// the pixels as paint takes them, 0xFFRRGGBB row by row; the shader reads the map `values` as a uniform,
// `values.<name>` (src/gpu.rs does both natively). WebGPU only answers
// asynchronously, so a task Worker (host-tasks.js) does the GPU work while the program's worker waits for its answer in
// shared memory, as a task's result. In the playground, paint of a shader renders straight into the page's canvas: the
// page hands the canvas's OffscreenCanvas to the task Worker, which draws through its GPUCanvasContext, and no pixels
// come back (card web-apis-rest).

const GPU_ENTRY_POINT = "main";
const GPU_BINDING = 0;
// `@group(0) @binding(1) var<storage, read_write> ys: array<f32>`: its binding, name and element type (src/gpu.rs STORAGE_ARRAY)
const STORAGE_ARRAY = /@binding\((\d+)\)(?:\s*@group\(\d+\))?\s*var(?:<[^>]*>)?\s+(\w+)\s*:\s*array<(\w+)/g;
const ELEMENT_ARRAYS = { i32: Int32Array, u32: Uint32Array };
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
const VECTOR_FLOATS = 4; // the floats of one vector in an array value, `values.<name>[i]` (src/gpu.rs)
let gpuDevice;
// a shader painted every frame compiles once, not each frame (card playground-tour: the tour's page crashed in a
// shader animation on a Mac): the task Worker's last MOST_COMPILED modules and pipelines, by their WGSL
const MOST_COMPILED = 16;
const compiledModules = new Map();
const compiledPipelines = new Map();
const WORDS_STATE = 2; // the shared buffer's state when it holds raw 32-bit words, floats or pixels (writeShared's JSON is state 1)
// kept buffers (src/lowering/gpu_maps.rs marked_kept): a map result read on the CPU stays on the GPU for the next map of
// it, by block, the last MOST_KEPT of them (src/gpu.rs KEPT); flags as gpu_map_linear's last argument
const KEEP_RESULT = 1n;
const SOURCE_KEPT = 2n;
// a map the compiler switched to the GPU (gpu_maps.rs AUTOMATIC, card gpu-auto): silent without an adapter, else said once
const AUTOMATIC = 4n;
const AUTOMATIC_NOTICE = "ran on the GPU by itself (f32, imprecise by design; write @cpu for exact f64)"; // gpu_maps.rs
const MOST_KEPT = 4;
const KEPT_MISSING = "no kept buffer";
const keptBuffers = new Map(); // the task Worker's: block → {storage, count}
const CANVAS_MISSING = "no canvas of this paint";
const paintCanvases = new Map(); // the task Worker's: canvas id → {context, format}
let paintCanvasCount = 0; // the program's side: the canvases asked of the page, which names them by this count
let gpuWorker; // the program's side: the task Worker that ran the last GPU job, and keeps its buffers

// the task Worker's side: the floats the shader left from index `first` on (only those are read back)
// before `numbers`, the `count` items of the buffer kept for block `source`; the buffer kept for block `keep` after
async function gpuComputedFloats({ shader, numbers, workgroups, first = 0, source, count, keep }) {
	const device = await theGpuDevice();
	const Elements = gpuElements(shader);
	const input = numbers instanceof Elements ? numbers : new Elements(numbers);
	const kept = source === undefined ? undefined : keptBuffers.get(source);
	if (source !== undefined && kept?.count !== count) throw new Error(KEPT_MISSING);
	const keptBytes = (kept?.count ?? 0) * Float32Array.BYTES_PER_ELEMENT;
	const size = keptBytes + input.byteLength;
	const offset = first * Float32Array.BYTES_PER_ELEMENT;
	const usage = GPUBufferUsage;
	const storage = device.createBuffer({ size, usage: usage.STORAGE | usage.COPY_SRC | usage.COPY_DST });
	const readback = device.createBuffer({ size: size - offset, usage: usage.MAP_READ | usage.COPY_DST });
	if (input.length) device.queue.writeBuffer(storage, keptBytes, input);
	const module = await gpuModule(device, shader);
	device.pushErrorScope("validation");
	const pipeline = compiledOnce(compiledPipelines, shader, () => device.createComputePipeline({ layout: "auto", compute: { module, entryPoint: GPU_ENTRY_POINT } }));
	const bindings = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: [{ binding: GPU_BINDING, resource: { buffer: storage } }] });
	const encoder = device.createCommandEncoder();
	if (kept) encoder.copyBufferToBuffer(kept.storage, 0, storage, 0, keptBytes);
	const pass = encoder.beginComputePass();
	pass.setPipeline(pipeline);
	pass.setBindGroup(0, bindings);
	pass.dispatchWorkgroups(workgroups);
	pass.end();
	encoder.copyBufferToBuffer(storage, offset, readback, 0, size - offset);
	const floats = new Elements(await gpuReadBack(device, encoder, readback));
	if (keep === undefined) storage.destroy();
	else keepBuffer(keep.block, { storage, count: keep.count });
	return floats;
}

// an earlier run's buffer for the same block address is stale; the oldest goes beyond MOST_KEPT
function keepBuffer(block, buffer) {
	keptBuffers.get(block)?.storage.destroy();
	keptBuffers.delete(block);
	keptBuffers.set(block, buffer);
	if (keptBuffers.size > MOST_KEPT) {
		const [oldest, { storage }] = keptBuffers.entries().next().value;
		storage.destroy();
		keptBuffers.delete(oldest);
	}
}

// the holes every shader has (src/shader_holes.rs BUILTIN_HOLES), filled here when the shader reads them and the
// program gave none: the image's size, seconds since the first render, the renders before, the pointer and key over
// the canvas (canvas.js, self.pagePointer)
const BUILTIN_HOLES = ["width", "height", "size", "time", "frame", "mouse", "mouse_down", "key"];
let firstRender, renders = 0;
function builtinValues(shader, width, height) {
	const shared = name => {
		const at = self.pagePointer?.names.indexOf(name) ?? -1;
		return at >= 0 ? Atomics.load(self.pagePointer.values, at) : 0;
	};
	firstRender ??= performance.now();
	const value = { width, height, size: [width, height], time: (performance.now() - firstRender) / 1000, frame: renders++,
		mouse: [shared("mouse_x"), shared("mouse_y")], mouse_down: shared("mouse_down"), key: shared("key") };
	return Object.fromEntries(BUILTIN_HOLES.filter(name => new RegExp(`\\b${VALUES_NAME}\\.${name}\\b`).test(shader)).map(name => [name, value[name]]));
}

// the task Worker's side of a page canvas: the OffscreenCanvas the page sends through `port`, drawn on through WebGPU
async function gpuCanvas({ id, port, replaces }) {
	const device = await theGpuDevice(); // without an adapter that error, as gpu_render's
	const canvas = await new Promise(resolve => port.onmessage = ({ data }) => resolve(data.canvas));
	port.close();
	paintCanvases.delete(replaces);
	const context = canvas.getContext("webgpu");
	const format = navigator.gpu.getPreferredCanvasFormat();
	context.configure({ device, format, alphaMode: "opaque" });
	paintCanvases.set(id, { context, format });
	return id;
}

// the pixels the fragment shader colored, row by row; given a `canvas` id, drawn into that page canvas instead
async function gpuRendered({ shader, width, height, values, canvas }) {
	const device = await theGpuDevice();
	const painted = paintCanvases.get(canvas);
	if (canvas !== undefined && !painted) throw new Error(CANVAS_MISSING);
	const { declarations, bytes: valueBytes } = uniformLayout(values);
	const code = shader + FULL_IMAGE_VERTICES + declarations;
	const module = await gpuModule(device, code);
	const format = painted?.format ?? "rgba8unorm";
	const image = painted?.context.getCurrentTexture() ?? device.createTexture({ size: [width, height], format, usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC });
	device.pushErrorScope("validation");
	const { pipeline, groupLayout } = compiledOnce(compiledPipelines, `${format}\n${code}`, () => {
		const groupLayout = declarations ? valuesGroupLayout(device) : undefined;
		const layout = groupLayout ? device.createPipelineLayout({ bindGroupLayouts: [groupLayout] }) : "auto";
		const pipeline = device.createRenderPipeline({
			layout,
			vertex: { module, entryPoint: VERTEX_ENTRY_POINT },
			fragment: { module, entryPoint: GPU_ENTRY_POINT, targets: [{ format }] },
		});
		return { pipeline, groupLayout };
	});
	const uniform = groupLayout ? uniformBinding(device, groupLayout, valueBytes) : undefined;
	const encoder = device.createCommandEncoder();
	const pass = encoder.beginRenderPass({ colorAttachments: [{ view: image.createView(), loadOp: "clear", storeOp: "store" }] });
	pass.setPipeline(pipeline);
	if (uniform) pass.setBindGroup(0, uniform.bindings);
	pass.draw(TRIANGLE_CORNERS);
	pass.end();
	if (painted) return gpuSubmitted(device, encoder).finally(() => uniform?.buffer.destroy());
	const rowBytes = Math.ceil(width * PIXEL_BYTES / ROW_ALIGNMENT) * ROW_ALIGNMENT;
	const readback = device.createBuffer({ size: rowBytes * height, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST });
	encoder.copyTextureToBuffer({ texture: image }, { buffer: readback, bytesPerRow: rowBytes, rowsPerImage: height }, [width, height]);
	const bytes = new Uint8Array(await gpuReadBack(device, encoder, readback));
	image.destroy();
	uniform?.buffer.destroy();
	const pixels = new Uint32Array(width * height);
	for (let row = 0, pixel = 0; row < height; row++)
		for (let at = row * rowBytes; at < row * rowBytes + width * PIXEL_BYTES; at += PIXEL_BYTES)
			pixels[pixel++] = (OPAQUE | bytes[at] << 16 | bytes[at + 1] << 8 | bytes[at + 2]) >>> 0;
	return pixels;
}

// the WGSL declaring `values` (appended, so the shader's line numbers stay the user's) and the uniform's bytes, laid out
// as WGSL aligns its members (src/gpu.rs uniform_layout)
function uniformLayout(values) {
	const entries = Object.entries(values ?? {});
	if (!entries.length) return { declarations: "", bytes: new ArrayBuffer(0) };
	const members = [], floats = [];
	const padded = alignment => { while (floats.length % (alignment / 4)) floats.push(0); };
	for (const [name, value] of entries) {
		// a list of lists: one vec4f per item, its missing coordinates 0
		const vectors = Array.isArray(value) && value.some(Array.isArray);
		if (vectors && value.some(item => [item].flat().length > VECTOR_FLOATS)) throw new Error(`${VALUES_NAME}.${name} is a list of vectors of up to four numbers, got ${JSON.stringify(value)}`);
		const numbers = vectors ? value.flatMap(item => [...[item].flat(), 0, 0, 0, 0].slice(0, VECTOR_FLOATS)) : [value].flat();
		if (!numbers.every(number => typeof number === "number")) throw new Error(`${VALUES_NAME}.${name} is a number or a list of numbers, got ${JSON.stringify(value)}`);
		const array = vectors || numbers.length > VECTOR_FLOATS;
		const [type, alignment] = array ? [`array<vec4f, ${Math.max(1, Math.ceil(numbers.length / VECTOR_FLOATS))}>`, UNIFORM_ALIGNMENT] : VALUE_TYPES[numbers.length] ?? [];
		if (!type) throw new Error(`${VALUES_NAME}.${name} is one number or a list of two to four, got ${numbers.length}`);
		padded(alignment);
		floats.push(...numbers);
		if (array) padded(UNIFORM_ALIGNMENT);
		members.push(`${name}: ${type}`);
	}
	padded(UNIFORM_ALIGNMENT);
	const declarations = `\nstruct WarpValues { ${members.join(", ")} }\n@group(0) @binding(${VALUES_BINDING}) var<uniform> ${VALUES_NAME}: WarpValues;`;
	return { declarations, bytes: new Float32Array(floats).buffer };
}

// an explicit layout holding the uniform, so a shader that leaves `values` unread still binds them
const valuesGroupLayout = device => device.createBindGroupLayout({ entries: [{ binding: VALUES_BINDING, visibility: GPUShaderStage.FRAGMENT, buffer: { type: "uniform" } }] });

// this frame's values in a uniform buffer, bound as the layout says; the buffer is destroyed once the frame is read
function uniformBinding(device, groupLayout, bytes) {
	const buffer = device.createBuffer({ size: bytes.byteLength, usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST });
	device.queue.writeBuffer(buffer, 0, bytes);
	const bindings = device.createBindGroup({ layout: groupLayout, entries: [{ binding: VALUES_BINDING, resource: { buffer } }] });
	return { buffer, bindings };
}

// the value made for `key` before, or made now; the oldest of more than MOST_COMPILED goes
function compiledOnce(cache, key, make) {
	if (!cache.has(key)) cache.set(key, make());
	if (cache.size > MOST_COMPILED) cache.delete(cache.keys().next().value);
	return cache.get(key);
}

// the task Worker's device, asked for once
const theGpuDevice = async () => gpuDevice ??= await gpuDeviceOrFailure();

async function gpuDeviceOrFailure() {
	const adapter = await self.navigator.gpu?.requestAdapter();
	if (!adapter) throw new Error("this browser offers no WebGPU adapter");
	return adapter.requestDevice();
}

// the shader's module, checked once: asking a module for its compilation info a second time crashed Chrome's page (Mac)
const gpuModule = (device, shader) => compiledOnce(compiledModules, shader, () => checkedModule(device, shader));

// the shader's module; one that does not compile throws where and why
async function checkedModule(device, shader) {
	const module = device.createShaderModule({ code: shader });
	const problems = (await module.getCompilationInfo()).messages.filter(message => message.type === "error");
	if (problems.length) throw new Error(problems.map(problem => `${problem.lineNum}:${problem.linePos}: ${problem.message}`).join("; "));
	return module;
}

// submit the encoded work (its validation scope pushed before the pipeline) and give the bytes it left in readback
const gpuReadBack = async (device, encoder, readback) => (await gpuReadBacks(device, encoder, [readback]))[0];

// submit the encoded work; the validation error it raised throws
async function gpuSubmitted(device, encoder) {
	device.queue.submit([encoder.finish()]);
	const invalid = await device.popErrorScope();
	if (invalid) throw new Error(invalid.message);
}

// as gpuReadBack, the bytes of each readback buffer
async function gpuReadBacks(device, encoder, readbacks) {
	await gpuSubmitted(device, encoder);
	return Promise.all(readbacks.map(async readback => {
		await readback.mapAsync(GPUMapMode.READ);
		const bytes = readback.getMappedRange().slice(0);
		readback.destroy();
		return bytes;
	}));
}

// the storage arrays the shader declares, each with the typed array of its elements: f32 unless array<i32> or array<u32>
const storageArrays = shader => [...shader.matchAll(STORAGE_ARRAY)].map(([, binding, name, element]) => ({ binding: Number(binding), name, Elements: ELEMENT_ARRAYS[element] ?? Float32Array }));

// the typed array of the numbers a compute shader reads at @binding(0) (natively gpu.rs element_of)
const gpuElements = shader => storageArrays(shader).find(array => array.binding === GPU_BINDING)?.Elements ?? Float32Array;

// gpu_compute over named arrays: each bound where the shader declares the storage array of that name; the arrays it
// left, by name, in the order given (natively gpu.rs compute_named)
async function gpuComputedArrays({ shader, arrays, workgroups }) {
	const declared = storageArrays(shader);
	const names = declared.map(array => array.name).join(", ");
	const missing = declared.find(array => !(array.name in arrays));
	if (missing) throw new Error(`the shader's array ${missing.name} has no numbers: give each of ${names}`);
	const given = Object.entries(arrays).map(([name, numbers]) => {
		const array = declared.find(array => array.name === name);
		if (!array) throw new Error(`the shader declares no storage array ${name}, only ${names}`);
		if (!numbers.length) throw new Error(`${name} is empty: a storage array holds at least one number`);
		return { ...array, numbers: new array.Elements(numbers) };
	});
	const device = await theGpuDevice();
	const usage = GPUBufferUsage;
	const storages = given.map(({ numbers }) => {
		const storage = device.createBuffer({ size: numbers.byteLength, usage: usage.STORAGE | usage.COPY_SRC | usage.COPY_DST });
		device.queue.writeBuffer(storage, 0, numbers);
		return storage;
	});
	const readbacks = storages.map(storage => device.createBuffer({ size: storage.size, usage: usage.MAP_READ | usage.COPY_DST }));
	const module = await gpuModule(device, shader);
	device.pushErrorScope("validation");
	const pipeline = compiledOnce(compiledPipelines, shader, () => device.createComputePipeline({ layout: "auto", compute: { module, entryPoint: GPU_ENTRY_POINT } }));
	const bindings = device.createBindGroup({ layout: pipeline.getBindGroupLayout(0), entries: given.map(({ binding }, at) => ({ binding, resource: { buffer: storages[at] } })) });
	const encoder = device.createCommandEncoder();
	const pass = encoder.beginComputePass();
	pass.setPipeline(pipeline);
	pass.setBindGroup(0, bindings);
	pass.dispatchWorkgroups(workgroups);
	pass.end();
	storages.forEach((storage, at) => encoder.copyBufferToBuffer(storage, 0, readbacks[at], 0, storage.size));
	const left = await gpuReadBacks(device, encoder, readbacks).finally(() => storages.forEach(storage => storage.destroy()));
	return Object.fromEntries(given.map(({ name, Elements }, at) => [name, Array.from(new Elements(left[at]))]));
}

// gpu_compute's {values} as a list; a kernel's floats and an image's pixels as {words}, given back as raw bytes
// (writeSharedWords), not as JSON
const GPU_JOBS = {
	gpu_compute: async job => ({ values: job.arrays ? await gpuComputedArrays(job) : Array.from(await gpuComputedFloats(job)) }),
	gpu_kernel: async job => ({ words: await gpuComputedFloats(job) }),
	gpu_render: async job => ({ words: await gpuRendered(job) }),
	gpu_canvas: async job => ({ values: await gpuCanvas(job) }),
	gpu_paint: async job => ({ values: (await gpuRendered(job), job.canvas) }),
};

// a task Worker's job (task-worker.js): the shader's answer, or why not, into the shared buffer the program waits on
async function gpuJobInto({ gpu: { word, job }, shared }) {
	let answer;
	try {
		answer = await GPU_JOBS[word](job);
	} catch (failure) {
		answer = { error: String(failure.message ?? failure) };
	}
	if (answer.words) writeSharedWords(shared, answer.words);
	else writeShared(shared, answer);
}

// 32-bit words into the shared buffer as they are: a million floats as JSON took longer than the GPU
function writeSharedWords(shared, words) {
	if (TASK_HEADER + words.byteLength > shared.byteLength) shared.grow(TASK_HEADER + words.byteLength);
	new Uint32Array(shared, TASK_HEADER, words.length).set(new Uint32Array(words.buffer, words.byteOffset, words.length));
	const header = new Int32Array(shared, 0, 2);
	header[1] = words.length;
	Atomics.store(header, 0, WORDS_STATE);
	Atomics.notify(header, 0);
}

// the program's side: hand the job to a task Worker and wait for the values it gives, raw words as `Words`
function gpuJob(word, job, transfer = [], Words = Float32Array) {
	if (!hasTaskWorkers()) throw new Error(`${word} needs task Workers, which only a cross-origin isolated page has (the playground)`);
	const worker = gpuTaskWorker();
	if (!worker) throw new Error(`${word}: every task Worker is busy`);
	const shared = new SharedArrayBuffer(TASK_HEADER + TASK_RESULT_BYTES, { maxByteLength: TASK_RESULT_LIMIT });
	worker.postMessage({ gpu: { word, job }, shared }, transfer);
	const header = new Int32Array(shared, 0, 2);
	Atomics.wait(header, 0, 0);
	const { values, error } = header[0] === WORDS_STATE ? { values: new Words(shared, TASK_HEADER, header[1]) } : readShared(shared);
	taskPool.push(worker);
	if (error) throw new Error(`${word}: ${error}`);
	return values;
}

// the task Worker of the last GPU job, which keeps its buffers; else (busy with a task) another, which does from now on
function gpuTaskWorker() {
	const index = taskPool.indexOf(gpuWorker);
	const worker = index < 0 ? taskPool.pop() : taskPool.splice(index, 1)[0];
	if (worker) gpuWorker = worker;
	return worker;
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
		// the kernel over the f32 copy of a block's cells, from `first` on read back; `kept` as gpuComputedFloats takes it
		const kernel = (shader, numbers, workgroups, first, kept = {}) =>
			gpuJob("gpu_kernel", { shader: plain(shader), numbers, workgroups: Number(workgroups), first, ...kept }, [numbers.buffer]);
		// the items (none when they come from a kept buffer), then the values of the program's numbers the kernel reads,
		// then a cell per workgroup to reduce into
		const kernelNumbers = (items, outer, reductions) => {
			const numbers = new Float32Array(items.length + outer.length + reductions);
			numbers.set(items);
			numbers.set(outer, items.length);
			return numbers;
		};
		// from source's kept buffer if it has one, else as uploaded
		const keptOrUploaded = (shader, items, outer, workgroups, reduces, source, keep, keeping) => {
			const first = reduces ? items.length + outer.length : 0;
			const reductions = reduces ? Number(workgroups) : 0;
			if (keeping & SOURCE_KEPT) {
				try {
					return kernel(shader, kernelNumbers([], outer, reductions), workgroups, first, { source: Number(source), count: items.length, keep });
				} catch (failure) {
					if (!failure.message.endsWith(KEPT_MISSING)) throw failure;
				}
			}
			return kernel(shader, kernelNumbers(items, outer, reductions), workgroups, first, { keep });
		};
		// as src/diagnostic.rs report_runtime_warning_once: the notice once a run, its detail after a colon
		const warnOnce = (notice, detail) => {
			if (!holder.warnings.some(warning => warning.startsWith(notice))) holder.warnings.push(`${notice}: ${detail}`);
		};
		const gpuKernelLinear = (shader, source, values, target, workgroups, reduces, keeping = 0n) => {
			const items = linearCells(program().memory, source);
			const outer = [plain(values) ?? []].flat().map(Number);
			const keep = keeping & KEEP_RESULT ? { block: Number(target), count: items.length } : undefined;
			try {
				const left = keptOrUploaded(shader, items, outer, workgroups, reduces, source, keep, keeping);
				const cells = linearCells(program().memory, target);
				cells.set(left.subarray(0, cells.length));
				if (keeping & AUTOMATIC) warnOnce(AUTOMATIC_NOTICE, `${reduces ? "a reduction" : "a map"} of ${items.length} items`);
				return 1n;
			} catch (failure) {
				if (!/adapter|task Workers/.test(failure.message)) throw failure;
				if (keeping & AUTOMATIC) return 0n; // the program did not ask for the GPU
				// once a run, as the program's runtime warning (natively report_runtime_warning), not on the console
				const warning = `@gpu: ${failure.message}, so the map runs on the CPU`;
				if (!holder.warnings.includes(warning)) holder.warnings.push(warning);
				return 0n;
			}
		};
		const renderJob = (shader, width, height, values) => ({ shader, width: Number(width), height: Number(height),
			values: { ...builtinValues(shader, Number(width), Number(height)), ...(values == null ? {} : plain(values)) } });
		// the pixels as a Uint32Array, for gpu_render and for paint given a shader (host.js, P234)
		holder.gpuRendered = (shader, width, height, values) => gpuJob("gpu_render", renderJob(shader, width, height, values), [], Uint32Array);
		// paint of a shader in the playground: drawn into the page canvas the GPU's task Worker holds, a new one when it
		// holds none of this size (the first frame, or another task Worker took the GPU); the canvas's id
		let paintCanvas;
		holder.gpuPainted = hooks.gpuCanvas && ((shader, width, height, values) => {
			const job = renderJob(shader, width, height, values);
			for (const retry of [false, true]) {
				if (paintCanvas?.width !== job.width || paintCanvas?.height !== job.height) paintCanvas = newPaintCanvas(job.width, job.height, paintCanvas?.id);
				try {
					return gpuJob("gpu_paint", { ...job, canvas: paintCanvas.id });
				} catch (failure) {
					if (retry || !failure.message.endsWith(CANVAS_MISSING)) throw failure;
					paintCanvas = undefined;
				}
			}
		});
		// the page makes the canvas and sends it through a channel to the task Worker, which configures it
		const newPaintCanvas = (width, height, replaces) => {
			const id = ++paintCanvasCount, { port1, port2 } = new MessageChannel();
			hooks.gpuCanvas({ id, width, height, port: port2, replaces });
			gpuJob("gpu_canvas", { id, port: port1, replaces }, [port1]);
			return { id, width, height };
		};
		return {
			gpu_compute: (shader, numbers, workgroups) => {
				const code = plain(shader), given = plain(numbers);
				// floats, also the whole ones (treeOfPlain would make 3 an Int); ints of an array<i32> or array<u32>
				const item = Elements => Elements === Float32Array ? float => ({ kind: KIND_FLOAT, data: { float }, chain: [] }) : int => ({ kind: KIND_INT, data: { int: String(int) }, chain: [] });
				if (given !== null && typeof given === "object" && isPlainObject(given)) {
					const arrays = Object.fromEntries(Object.entries(given).map(([name, array]) => [name, [array].flat().map(Number)]));
					const left = gpuJob("gpu_compute", { shader: code, arrays, workgroups: Number(workgroups) });
					const elements = Object.fromEntries(storageArrays(code).map(array => [array.name, array.Elements]));
					const entries = Object.entries(left).map(([name, values]) => ({ kind: COLON_KEY, key: [{ kind: "5", data: { text: name }, chain: [] }, { kind: SQUARE_LIST, items: values.map(item(elements[name])) }] }));
					return buildValue(program(), { kind: CURLY_LIST, items: entries });
				}
				const values = gpuJob("gpu_compute", { shader: code, numbers: given.map(Number), workgroups: Number(workgroups) });
				return list(values, item(gpuElements(code)));
			},
			// over a `linear xs = float[n]`: its cells read and written in place, no list built (card gpu-vectors)
			gpu_compute_linear: (shader, block, workgroups) => {
				const cells = linearCells(program().memory, block);
				cells.set(kernel(shader, Float32Array.from(cells), workgroups, 0));
				return block;
			},
			// `ys = xs.map(x => …) @gpu` (src/lowering/gpu_maps.rs): the kernel over source's cells (and the values it reads)
			// into target's; 0 without an adapter (said once), when the program maps them on the CPU
			gpu_map_linear: (shader, source, values, target, workgroups, keeping) => gpuKernelLinear(shader, source, values, target, workgroups, false, keeping),
			// `s = sum(xs.map(x => …) @gpu)`, min, max: each workgroup's partial result, left after the values, into target
			gpu_reduce_linear: (shader, source, values, target, workgroups, keeping) => gpuKernelLinear(shader, source, values, target, workgroups, true, keeping),
			gpu_render: (shader, width, height, values) => {
				const pixels = holder.gpuRendered(plain(shader), width, height, values);
				return listOfInts(program(), pixels) ?? list(Array.from(pixels), treeOfPlain);
			},
		};
	},
});
