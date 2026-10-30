// libwarp_gpu (card gpu-functions, decision shaders-in-lib): paint(shader, width, height, values) of a WGSL fragment
// shader on the GPU through wgpu-native, as web/playground/host-gpu.js gpuRendered paints it in a page: the same full
// image triangle, `values` the same uniform struct appended, the pixels read back row by row as 0xAARRGGBB.
// Built by scripts/wasmtime-c-api.sh beside libwarp_wasmtime; core/native/driver.warp painted() loads it lazily
#include <webgpu/webgpu.h>
#include <webgpu/wgpu.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FULL_IMAGE_VERTICES "\n@vertex fn warp_full_image(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4f {\n" \
	"\treturn vec4f(f32(corner / 2u) * 4.0 - 1.0, f32(corner % 2u) * 4.0 - 1.0, 0.0, 1.0);\n}"
#define VERTEX_ENTRY_POINT "warp_full_image"
#define FRAGMENT_ENTRY_POINT "main"
#define VALUES_NAME "values"
enum { VALUES_BINDING = 0, TRIANGLE_CORNERS = 3, PIXEL_BYTES = 4, ROW_ALIGNMENT = 256, UNIFORM_ALIGNMENT = 16,
	VECTOR_FLOATS = 4, MOST_FLOATS = 4096, MOST_DECLARATIONS = 16384, NAME_BYTES = 64 };
static const uint32_t OPAQUE = 0xFF000000u;
static const char* VALUE_TYPES[] = {0, "f32", "vec2f", "vec3f", "vec4f"};
static const int VALUE_ALIGNMENTS[] = {0, 4, 8, 16, 16};

static WGPUInstance instance;
static WGPUAdapter adapter;
static WGPUDevice device;
static WGPUQueue queue;
static char failure[1024];
// the last shader's pipeline, so a frame loop compiles once
static char* compiled_code;
static WGPURenderPipeline compiled_pipeline;

static WGPUStringView view_of(const char* text) { return (WGPUStringView){text, WGPU_STRLEN}; }
static void failed(const char* what, WGPUStringView message) {
	snprintf(failure, sizeof failure, "%s%.*s", what, (int)(message.data ? message.length : 0), message.data ? message.data : "");
}

static void adapter_got(WGPURequestAdapterStatus status, WGPUAdapter got, WGPUStringView message, void* done, void* unused) {
	if (status == WGPURequestAdapterStatus_Success) adapter = got; else failed("no GPU adapter: ", message);
	*(int*)done = 1;
}
static void device_got(WGPURequestDeviceStatus status, WGPUDevice got, WGPUStringView message, void* done, void* unused) {
	if (status == WGPURequestDeviceStatus_Success) device = got; else failed("no GPU device: ", message);
	*(int*)done = 1;
}
static void waited(int* done) { while (!*done) wgpuInstanceProcessEvents(instance); }

// the run's one device, asked for at its first paint
static int device_ready(void) {
	if (device) return 1;
	if (!instance) instance = wgpuCreateInstance(NULL);
	if (!instance) return snprintf(failure, sizeof failure, "no WebGPU instance"), 0;
	int done = 0;
	WGPURequestAdapterCallbackInfo adapter_info = WGPU_REQUEST_ADAPTER_CALLBACK_INFO_INIT;
	adapter_info.mode = WGPUCallbackMode_AllowProcessEvents;
	adapter_info.callback = adapter_got;
	adapter_info.userdata1 = &done;
	wgpuInstanceRequestAdapter(instance, NULL, adapter_info);
	waited(&done);
	if (!adapter) return 0;
	done = 0;
	WGPURequestDeviceCallbackInfo device_info = WGPU_REQUEST_DEVICE_CALLBACK_INFO_INIT;
	device_info.mode = WGPUCallbackMode_AllowProcessEvents;
	device_info.callback = device_got;
	device_info.userdata1 = &done;
	wgpuAdapterRequestDevice(adapter, NULL, device_info);
	waited(&done);
	if (!device) return 0;
	queue = wgpuDeviceGetQueue(device);
	return 1;
}

// --- the values: `{name: number, name: [numbers], name: [[numbers], …]}` as warp shows them, laid out as WGSL aligns
// a uniform struct's members (host-gpu.js uniformLayout)
typedef struct { float floats[MOST_FLOATS]; int count; char declarations[MOST_DECLARATIONS]; int members; } Layout;

static const char* spaces_skipped(const char* at) { while (*at == ' ' || *at == '\n' || *at == '\t') at++; return at; }
// a number as warp shows it: a decimal, or an exact fraction `a/b`
static const char* number_read(const char* at, float* number) {
	char* end;
	double value = strtod(at, &end);
	if (end == at) return NULL;
	if (*end == '/') {
		char* below_end;
		double below = strtod(end + 1, &below_end);
		if (below_end != end + 1) { value /= below; end = below_end; }
	}
	*number = (float)value;
	return end;
}
static void padded(Layout* layout, int alignment) { while (layout->count % (alignment / 4)) layout->floats[layout->count++] = 0; }
static int layout_failed(const char* name, const char* why) {
	return snprintf(failure, sizeof failure, VALUES_NAME ".%s %s", name, why), 0;
}
// one member: its numbers (vectors: each a vec4f) after the layout's, then its declaration
static int member_laid_out(Layout* layout, const char* name, float* numbers, int count, int vectors) {
	int array = vectors || count > VECTOR_FLOATS;
	if (!array && count < 1) return layout_failed(name, "is one number or a list of two to four");
	if (layout->count + count + 2 * VECTOR_FLOATS > MOST_FLOATS) return layout_failed(name, "has more numbers than a uniform holds here");
	char type[32];
	if (array) snprintf(type, sizeof type, "array<vec4f, %d>", count / VECTOR_FLOATS + (count % VECTOR_FLOATS != 0) + (count == 0));
	padded(layout, array ? UNIFORM_ALIGNMENT : VALUE_ALIGNMENTS[count]);
	memcpy(layout->floats + layout->count, numbers, count * sizeof(float));
	layout->count += count;
	if (array) padded(layout, UNIFORM_ALIGNMENT);
	size_t used = strlen(layout->declarations);
	snprintf(layout->declarations + used, MOST_DECLARATIONS - used, "%s%s: %s", layout->members++ ? ", " : "", name, array ? type : VALUE_TYPES[count]);
	return 1;
}
// the numbers of a list, `[1, 2]` or `[[1, 2], [3]]` (each inner one padded to a vec4f), after `[`
static const char* list_read(const char* at, float* numbers, int* count, int* vectors) {
	for (at = spaces_skipped(at); *at != ']'; at = spaces_skipped(at)) {
		if (*at == '[') {
			*vectors = 1;
			int inner = 0, nested = 0;
			float items[VECTOR_FLOATS + 1];
			at = list_read(at + 1, items, &inner, &nested);
			if (!at || nested || inner > VECTOR_FLOATS || *count + VECTOR_FLOATS > MOST_FLOATS) return NULL;
			for (int i = 0; i < VECTOR_FLOATS; i++) numbers[(*count)++] = i < inner ? items[i] : 0;
		} else {
			if (*count >= MOST_FLOATS) return NULL;
			at = number_read(at, &numbers[(*count)++]);
			if (!at) return NULL;
		}
		at = spaces_skipped(at);
		if (*at == ',') at++;
	}
	return at + 1;
}
static int values_laid_out(const char* values, Layout* layout) {
	static float numbers[MOST_FLOATS];
	const char* at = spaces_skipped(values);
	if (*at != '{') return 1;
	for (at = spaces_skipped(at + 1); *at && *at != '}'; at = spaces_skipped(at)) {
		char name[NAME_BYTES];
		int length = 0;
		while (*at && *at != ':' && length < NAME_BYTES - 1) name[length++] = *at++;
		name[length] = 0;
		if (*at != ':') return snprintf(failure, sizeof failure, "paint's values are a map of numbers"), 0;
		at = spaces_skipped(at + 1);
		int count = 0, vectors = 0;
		at = *at == '[' ? list_read(at + 1, numbers, &count, &vectors) : number_read(at, &numbers[count++]);
		if (!at) return layout_failed(name, "is a number or a list of numbers");
		if (!member_laid_out(layout, name, numbers, count, vectors)) return 0;
		at = spaces_skipped(at);
		if (*at == ',') at++;
	}
	padded(layout, UNIFORM_ALIGNMENT);
	return 1;
}

// --- the pipeline
typedef struct { int done; int failed; } Scope;
static void scope_popped(WGPUPopErrorScopeStatus status, WGPUErrorType type, WGPUStringView message, void* scope, void* unused) {
	if (type != WGPUErrorType_NoError) { failed("the shader: ", message); ((Scope*)scope)->failed = 1; }
	((Scope*)scope)->done = 1;
}
// whether the device reported no error since the scope was pushed
static int scope_clean(void) {
	Scope scope = {0, 0};
	WGPUPopErrorScopeCallbackInfo info = WGPU_POP_ERROR_SCOPE_CALLBACK_INFO_INIT;
	info.mode = WGPUCallbackMode_AllowProcessEvents;
	info.callback = scope_popped;
	info.userdata1 = &scope;
	wgpuDevicePopErrorScope(device, info);
	waited(&scope.done);
	return !scope.failed;
}
static WGPURenderPipeline pipeline_of(const char* code) {
	if (compiled_code && !strcmp(compiled_code, code)) return compiled_pipeline;
	wgpuDevicePushErrorScope(device, WGPUErrorFilter_Validation);
	WGPUShaderSourceWGSL source = WGPU_SHADER_SOURCE_WGSL_INIT;
	source.code = view_of(code);
	WGPUShaderModuleDescriptor module_descriptor = WGPU_SHADER_MODULE_DESCRIPTOR_INIT;
	module_descriptor.nextInChain = &source.chain;
	WGPUShaderModule module = wgpuDeviceCreateShaderModule(device, &module_descriptor);
	WGPUColorTargetState target = WGPU_COLOR_TARGET_STATE_INIT;
	target.format = WGPUTextureFormat_RGBA8Unorm;
	WGPUFragmentState fragment = WGPU_FRAGMENT_STATE_INIT;
	fragment.module = module;
	fragment.entryPoint = view_of(FRAGMENT_ENTRY_POINT);
	fragment.targetCount = 1;
	fragment.targets = &target;
	WGPURenderPipelineDescriptor descriptor = WGPU_RENDER_PIPELINE_DESCRIPTOR_INIT;
	descriptor.vertex.module = module;
	descriptor.vertex.entryPoint = view_of(VERTEX_ENTRY_POINT);
	descriptor.fragment = &fragment;
	WGPURenderPipeline pipeline = wgpuDeviceCreateRenderPipeline(device, &descriptor);
	wgpuShaderModuleRelease(module);
	if (!scope_clean()) {
		if (pipeline) wgpuRenderPipelineRelease(pipeline);
		return NULL;
	}
	if (compiled_pipeline) wgpuRenderPipelineRelease(compiled_pipeline);
	free(compiled_code);
	compiled_code = strdup(code);
	compiled_pipeline = pipeline;
	return pipeline;
}

static WGPUBuffer buffer_of(uint64_t size, WGPUBufferUsage usage) {
	WGPUBufferDescriptor descriptor = WGPU_BUFFER_DESCRIPTOR_INIT;
	descriptor.size = size;
	descriptor.usage = usage;
	return wgpuDeviceCreateBuffer(device, &descriptor);
}
static void mapped(WGPUMapAsyncStatus status, WGPUStringView message, void* done, void* unused) {
	if (status != WGPUMapAsyncStatus_Success) failed("reading the pixels back: ", message);
	*(int*)done = status == WGPUMapAsyncStatus_Success ? 1 : -1;
}

// the shader's pixels, row by row as 0xAARRGGBB, into `pixels` (width × height); 0, else -1 and warp_gpu_failure()
int warp_gpu_paint(const char* shader, const char* values, int32_t width, int32_t height, uint32_t* pixels) {
	failure[0] = 0;
	if (!device_ready()) return -1;
	static Layout layout;
	layout.count = 0;
	layout.members = 0;
	layout.declarations[0] = 0;
	if (!values_laid_out(values, &layout)) return -1;
	size_t code_size = strlen(shader) + sizeof FULL_IMAGE_VERTICES + MOST_DECLARATIONS + 128;
	char* code = malloc(code_size);
	snprintf(code, code_size, "%s%s", shader, FULL_IMAGE_VERTICES);
	if (layout.members) {
		size_t used = strlen(code);
		snprintf(code + used, code_size - used, "\nstruct WarpValues { %s }\n@group(0) @binding(%d) var<uniform> " VALUES_NAME ": WarpValues;",
			layout.declarations, VALUES_BINDING);
	}
	WGPURenderPipeline pipeline = pipeline_of(code);
	free(code);
	if (!pipeline) return -1;

	WGPUTextureDescriptor texture_descriptor = WGPU_TEXTURE_DESCRIPTOR_INIT;
	texture_descriptor.size = (WGPUExtent3D){(uint32_t)width, (uint32_t)height, 1};
	texture_descriptor.format = WGPUTextureFormat_RGBA8Unorm;
	texture_descriptor.usage = WGPUTextureUsage_RenderAttachment | WGPUTextureUsage_CopySrc;
	WGPUTexture image = wgpuDeviceCreateTexture(device, &texture_descriptor);
	WGPUTextureView image_view = wgpuTextureCreateView(image, NULL);
	WGPUBuffer uniform = NULL;
	WGPUBindGroup bindings = NULL;
	if (layout.members) {
		uint64_t size = layout.count * sizeof(float);
		uniform = buffer_of(size, WGPUBufferUsage_Uniform | WGPUBufferUsage_CopyDst);
		wgpuQueueWriteBuffer(queue, uniform, 0, layout.floats, size);
		WGPUBindGroupEntry entry = WGPU_BIND_GROUP_ENTRY_INIT;
		entry.binding = VALUES_BINDING;
		entry.buffer = uniform;
		entry.size = size;
		WGPUBindGroupDescriptor group = WGPU_BIND_GROUP_DESCRIPTOR_INIT;
		group.layout = wgpuRenderPipelineGetBindGroupLayout(pipeline, 0);
		group.entryCount = 1;
		group.entries = &entry;
		bindings = wgpuDeviceCreateBindGroup(device, &group);
		wgpuBindGroupLayoutRelease(group.layout);
	}
	WGPUCommandEncoder encoder = wgpuDeviceCreateCommandEncoder(device, NULL);
	WGPURenderPassColorAttachment attachment = WGPU_RENDER_PASS_COLOR_ATTACHMENT_INIT;
	attachment.view = image_view;
	attachment.loadOp = WGPULoadOp_Clear;
	attachment.storeOp = WGPUStoreOp_Store;
	WGPURenderPassDescriptor pass_descriptor = WGPU_RENDER_PASS_DESCRIPTOR_INIT;
	pass_descriptor.colorAttachmentCount = 1;
	pass_descriptor.colorAttachments = &attachment;
	WGPURenderPassEncoder pass = wgpuCommandEncoderBeginRenderPass(encoder, &pass_descriptor);
	wgpuRenderPassEncoderSetPipeline(pass, pipeline);
	if (bindings) wgpuRenderPassEncoderSetBindGroup(pass, 0, bindings, 0, NULL);
	wgpuRenderPassEncoderDraw(pass, TRIANGLE_CORNERS, 1, 0, 0);
	wgpuRenderPassEncoderEnd(pass);
	wgpuRenderPassEncoderRelease(pass);

	uint32_t row_bytes = (width * PIXEL_BYTES + ROW_ALIGNMENT - 1) / ROW_ALIGNMENT * ROW_ALIGNMENT;
	WGPUBuffer readback = buffer_of((uint64_t)row_bytes * height, WGPUBufferUsage_MapRead | WGPUBufferUsage_CopyDst);
	WGPUTexelCopyTextureInfo from = WGPU_TEXEL_COPY_TEXTURE_INFO_INIT;
	from.texture = image;
	WGPUTexelCopyBufferInfo into = WGPU_TEXEL_COPY_BUFFER_INFO_INIT;
	into.buffer = readback;
	into.layout.bytesPerRow = row_bytes;
	into.layout.rowsPerImage = (uint32_t)height;
	wgpuCommandEncoderCopyTextureToBuffer(encoder, &from, &into, &texture_descriptor.size);
	WGPUCommandBuffer commands = wgpuCommandEncoderFinish(encoder, NULL);
	wgpuQueueSubmit(queue, 1, &commands);
	wgpuCommandBufferRelease(commands);
	wgpuCommandEncoderRelease(encoder);

	int done = 0;
	WGPUBufferMapCallbackInfo map_info = WGPU_BUFFER_MAP_CALLBACK_INFO_INIT;
	map_info.mode = WGPUCallbackMode_AllowProcessEvents;
	map_info.callback = mapped;
	map_info.userdata1 = &done;
	wgpuBufferMapAsync(readback, WGPUMapMode_Read, 0, (size_t)row_bytes * height, map_info);
	while (!done) {
		wgpuDevicePoll(device, 1, NULL);
		wgpuInstanceProcessEvents(instance);
	}
	if (done > 0) {
		const uint8_t* bytes = wgpuBufferGetConstMappedRange(readback, 0, (size_t)row_bytes * height);
		for (int row = 0; row < height; row++)
			for (int column = 0; column < width; column++) {
				const uint8_t* at = bytes + (size_t)row * row_bytes + column * PIXEL_BYTES;
				pixels[row * width + column] = OPAQUE | (uint32_t)at[0] << 16 | (uint32_t)at[1] << 8 | at[2];
			}
		wgpuBufferUnmap(readback);
	}
	wgpuBufferRelease(readback);
	if (bindings) wgpuBindGroupRelease(bindings);
	if (uniform) wgpuBufferRelease(uniform);
	wgpuTextureViewRelease(image_view);
	wgpuTextureRelease(image);
	return done > 0 ? 0 : -1;
}

// why the last warp_gpu_paint failed
const char* warp_gpu_failure(void) { return failure; }
