(async () => {
	const describe = async () => {
		const out = { steps: self.probeSteps = [] };
		self.addEventListener("error", event => out.steps.push(`error event: ${event.message} at ${event.filename}:${event.lineno}:${event.colno} ${event.error}`));
		self.addEventListener("unhandledrejection", event => out.steps.push(`unhandled rejection: ${event.reason}`));
		try { out.gpu = !!self.navigator.gpu; } catch (failure) { out.gpu = `THREW ${failure} ${failure?.stack}`; return out; }
		const thrown = value => value === undefined ? "undefined" : value === null ? "null" : `${value?.constructor?.name}: ${value?.message ?? value}`;
		const step = async (name, run) => {
			try {
				const value = await run();
				out.steps.push(`${name}: ok ${typeof value === "object" ? (value === null ? "null" : value.constructor?.name) : String(value)}`);
				return value;
			} catch (failure) {
				out.steps.push(`${name}: THREW ${thrown(failure)} ${failure?.stack ?? ""}`);
			}
		};
		const adapter = await step("requestAdapter", () => self.navigator.gpu?.requestAdapter());
		if (!adapter) return out;
		await step("info", () => `${adapter.info?.vendor} ${adapter.info?.architecture} ${adapter.info?.description}`);
		const device = await step("requestDevice", () => adapter.requestDevice());
		if (!device) return out;
		await step("format", () => navigator.gpu.getPreferredCanvasFormat());
		const good = "@vertex fn v(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f { return vec4f(0.0); }\n@fragment fn main() -> @location(0) vec4f { return vec4f(1.0); }";
		const module = await step("createShaderModule", () => device.createShaderModule({ code: good }));
		await step("getCompilationInfo good", async () => (await module.getCompilationInfo()).messages.length);
		const bad = device.createShaderModule({ code: "fn broken( {" });
		await step("getCompilationInfo bad", async () => (await bad.getCompilationInfo()).messages.map(m => `${m.type} ${m.lineNum}:${m.linePos} ${m.message}`).join(" | "));
		await step("pushErrorScope", () => device.pushErrorScope("validation"));
		const pipeline = await step("createRenderPipeline", () => device.createRenderPipeline({ layout: "auto", vertex: { module, entryPoint: "v" }, fragment: { module, entryPoint: "main", targets: [{ format: "rgba8unorm" }] } }));
		await step("popErrorScope", async () => thrown(await device.popErrorScope()));
		const texture = await step("createTexture", () => device.createTexture({ size: [4, 4], format: "rgba8unorm", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC }));
		const readback = await step("createBuffer", () => device.createBuffer({ size: 256 * 4, usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST }));
		await step("render+readback", async () => {
			const encoder = device.createCommandEncoder();
			const pass = encoder.beginRenderPass({ colorAttachments: [{ view: texture.createView(), loadOp: "clear", storeOp: "store" }] });
			pass.setPipeline(pipeline);
			pass.draw(3);
			pass.end();
			encoder.copyTextureToBuffer({ texture }, { buffer: readback, bytesPerRow: 256, rowsPerImage: 4 }, [4, 4]);
			device.queue.submit([encoder.finish()]);
			await readback.mapAsync(GPUMapMode.READ);
			return new Uint8Array(readback.getMappedRange().slice(0)).length;
		});
		if (self.OffscreenCanvas) {
			const context = await step("offscreen getContext", () => new OffscreenCanvas(8, 8).getContext("webgpu"));
			if (context) await step("configure", () => context.configure({ device, format: navigator.gpu.getPreferredCanvasFormat(), alphaMode: "opaque", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_DST }));
		}
		return out;
	};
	const where = [];
	let page;
	try { where.push("page"); page = await describe(); } catch (failure) { return `page THREW ${failure} ${failure?.stack} after ${where}`; }
	const source = `const describe = ${describe}; describe().then(r => postMessage(r), e => postMessage({ workerError: String(e) }));`;
	const worker = new Worker(URL.createObjectURL(new Blob([source], { type: "text/javascript" })));
	const inWorker = await new Promise(done => { worker.onmessage = m => done(m.data); worker.onerror = e => done({ onerror: e.message }); setTimeout(() => done({ timeout: true }), 20000); });
	// the playground's hand-off (host-gpu.js gpuPainted): a page canvas's OffscreenCanvas sent over a MessagePort that
	// reached the drawing Worker through another port, drawn there through WebGPU
	const drawer = `
		self.addEventListener("error", event => self.report?.(\`error event: \${event.message} at \${event.lineno}\`));
		onmessage = ({ data }) => {
			const program = data.port;
			self.report = text => program.postMessage(text);
			program.onmessage = ({ data }) => {
				program.postMessage("got the canvas port");
				data.canvasPort.onmessage = async ({ data }) => {
					try {
						program.postMessage("got " + data.canvas);
						const adapter = await navigator.gpu.requestAdapter();
						const device = await adapter.requestDevice();
						const context = data.canvas.getContext("webgpu");
						program.postMessage("context " + context);
						context.configure({ device, format: navigator.gpu.getPreferredCanvasFormat(), alphaMode: "opaque", usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_DST });
						const encoder = device.createCommandEncoder();
						const pass = encoder.beginRenderPass({ colorAttachments: [{ view: context.getCurrentTexture().createView(), loadOp: "clear", clearValue: [1, 0, 0, 1], storeOp: "store" }] });
						pass.end();
						device.queue.submit([encoder.finish()]);
						program.postMessage("drawn");
					} catch (failure) { program.postMessage("THREW " + failure); }
				};
			};
		};`;
	const handoff = [];
	try {
		const drawing = new Worker(URL.createObjectURL(new Blob([drawer], { type: "text/javascript" })));
		drawing.onerror = event => handoff.push(`drawing worker onerror: ${event.message}`);
		const toDrawer = new MessageChannel();
		drawing.postMessage({ port: toDrawer.port2 }, [toDrawer.port2]);
		toDrawer.port1.onmessage = ({ data }) => handoff.push(data);
		const canvasChannel = new MessageChannel();
		toDrawer.port1.postMessage({ canvasPort: canvasChannel.port1 }, [canvasChannel.port1]);
		const canvas = document.body.appendChild(document.createElement("canvas"));
		canvas.width = canvas.height = 8;
		const offscreen = canvas.transferControlToOffscreen();
		handoff.push("transferred control");
		canvasChannel.port2.postMessage({ canvas: offscreen }, [offscreen]);
		handoff.push("posted the canvas");
	} catch (failure) { handoff.push(`page THREW ${failure}`); }
	await new Promise(done => setTimeout(done, 5000));
	return JSON.stringify({ userAgent: navigator.userAgent, isolated: self.crossOriginIsolated, page, worker: inWorker, handoff }, null, 1);
})()
