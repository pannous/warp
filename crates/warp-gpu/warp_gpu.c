// libwarp_gpu (card gpu-functions, decision shaders-in-lib): paint(shader, width, height, values) of a WGSL fragment
// shader on the GPU through wgpu-native, as web/playground/host-gpu.js gpuRendered paints it in a page: the same full
// image triangle, `values` the same uniform struct appended, the pixels read back row by row as 0xAARRGGBB.
// Built by scripts/wasmtime-c-api.sh beside libwarp_wasmtime; core/native/driver.warp painted() loads it lazily
#include <webgpu/webgpu.h>
#include <webgpu/wgpu.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "warp_node.h"

#define FULL_IMAGE_VERTICES "\n@vertex fn warp_full_image(@builtin(vertex_index) corner: u32) -> @builtin(position) vec4f {\n" \
	"\treturn vec4f(f32(corner / 2u) * 4.0 - 1.0, f32(corner % 2u) * 4.0 - 1.0, 0.0, 1.0);\n}"
#define VERTEX_ENTRY_POINT "warp_full_image"
#define FRAGMENT_ENTRY_POINT "main"
// a shader with a vertex stage of its own (card gpu-instanced-blend) draws `values.instances` quads, each its 4 corners
// (vertex_index 0 to 3) as a triangle strip, blended over what the quads before drew (instance_index 0 first), as
// web/playground/host-gpu.js draws them
#define OWN_VERTEX_STAGE "@vertex"
#define INSTANCES_NAME "instances"
#define VALUES_NAME "values"
// MOST_FLOATS: 64 KB, the uniform every WebGPU device binds (maxUniformBufferBindingSize), as the playground's
enum { VALUES_BINDING = 0, TRIANGLE_CORNERS = 3, PIXEL_BYTES = 4, ROW_ALIGNMENT = 256, UNIFORM_ALIGNMENT = 16,
	VECTOR_FLOATS = 4, QUAD_CORNERS = 4, MOST_FLOATS = 16384, MOST_DECLARATIONS = 16384, NAME_BYTES = 64 };
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
static WGPUTextureFormat compiled_format;
static WGPURenderPipeline compiled_pipeline;
// the window's surface a shader shows in (warp_gpu_show), configured at the size of what it shows
static WGPUSurface surface;
static void* surface_layer;
static uint32_t surface_width, surface_height;

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
// paint's values (host-gpu.js valuesLayout): its numbers and short vectors a uniform struct's members, each list of
// more (a list of vectors, one vec4f an item, or of more than 4 numbers, 4 a vec4f) a read-only storage array of its
// own (card paint-values-storage), as long as it is: `values.mesh` in the shader is the array warp_values_mesh
#define STORAGE_PREFIX "warp_values_"
enum { STORAGE_BINDING = 2, MOST_STORAGE_ARRAYS = 6 };
typedef struct { float* items; int count, room; } Floats;
typedef struct { char name[NAME_BYTES]; Floats numbers; } StorageValues;
typedef struct { float floats[MOST_FLOATS]; int count; char declarations[MOST_DECLARATIONS]; int members; int instances;
	StorageValues arrays[MOST_STORAGE_ARRAYS]; int array_count; } Layout;

static void pushed(Floats* floats, float number) {
	if (floats->count == floats->room) {
		floats->room = floats->room ? 2 * floats->room : 1024;
		floats->items = realloc(floats->items, floats->room * sizeof(float));
	}
	floats->items[floats->count++] = number;
}
static int name_char(char c) { return c == '_' || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9'); }
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
// one member: a list of more than 4 numbers or of vectors a storage array, else a uniform member after the others
static int member_laid_out(Layout* layout, const char* name, Floats* numbers, int vectors) {
	int count = numbers->count;
	if (vectors || count > VECTOR_FLOATS) {
		if (layout->array_count == MOST_STORAGE_ARRAYS) return layout_failed(name, "is one list too many: paint's values hold at most 6");
		while (numbers->count % VECTOR_FLOATS || numbers->count == 0) pushed(numbers, 0);
		StorageValues* array = &layout->arrays[layout->array_count++];
		snprintf(array->name, NAME_BYTES, "%s", name);
		array->numbers = *numbers;
		*numbers = (Floats){0};
		return 1;
	}
	if (count < 1) return layout_failed(name, "is one number or a list of two to four");
	if (layout->count + count + VECTOR_FLOATS > MOST_FLOATS) return layout_failed(name, "has more numbers than a uniform holds here");
	padded(layout, VALUE_ALIGNMENTS[count]);
	memcpy(layout->floats + layout->count, numbers->items, count * sizeof(float));
	layout->count += count;
	if (count == 1 && !strcmp(name, INSTANCES_NAME)) layout->instances = (int)numbers->items[0];
	size_t used = strlen(layout->declarations);
	snprintf(layout->declarations + used, MOST_DECLARATIONS - used, "%s%s: %s", layout->members++ ? ", " : "", name, VALUE_TYPES[count]);
	numbers->count = 0;
	return 1;
}
// the numbers of a list, `[1, 2]` or `[[1, 2], [3]]` (each inner one padded to a vec4f), after `[`
static const char* list_read(const char* at, Floats* numbers, int* vectors) {
	for (at = spaces_skipped(at); *at != ']'; at = spaces_skipped(at)) {
		if (*at == '[') {
			*vectors = 1;
			int inner = 0;
			float items[VECTOR_FLOATS];
			for (at = spaces_skipped(at + 1); *at != ']'; at = spaces_skipped(at)) {
				if (inner == VECTOR_FLOATS || !(at = number_read(at, &items[inner++]))) return NULL;
				at = spaces_skipped(at);
				if (*at == ',') at++;
			}
			at++;
			for (int i = 0; i < VECTOR_FLOATS; i++) pushed(numbers, i < inner ? items[i] : 0);
		} else {
			float number;
			if (!(at = number_read(at, &number))) return NULL;
			pushed(numbers, number);
		}
		at = spaces_skipped(at);
		if (*at == ',') at++;
	}
	return at + 1;
}
static void layout_cleared(Layout* layout) {
	for (int i = 0; i < layout->array_count; i++) free(layout->arrays[i].numbers.items);
	layout->array_count = 0;
	layout->count = 0;
	layout->members = 0;
	layout->declarations[0] = 0;
	layout->instances = 1;
}
static int values_laid_out(const char* values, Layout* layout) {
	static Floats numbers;
	const char* at = spaces_skipped(values);
	if (*at != '{') return 1;
	for (at = spaces_skipped(at + 1); *at && *at != '}'; at = spaces_skipped(at)) {
		char name[NAME_BYTES];
		int length = 0;
		while (*at && *at != ':' && length < NAME_BYTES - 1) name[length++] = *at++;
		name[length] = 0;
		if (*at != ':') return snprintf(failure, sizeof failure, "paint's values are a map of numbers"), 0;
		at = spaces_skipped(at + 1);
		int vectors = 0;
		numbers.count = 0;
		if (*at == '[') at = list_read(at + 1, &numbers, &vectors);
		else {
			float number;
			if ((at = number_read(at, &number))) pushed(&numbers, number);
		}
		if (!at) return layout_failed(name, "is a number or a list of numbers");
		if (!member_laid_out(layout, name, &numbers, vectors)) return 0;
		at = spaces_skipped(at);
		if (*at == ',') at++;
	}
	padded(layout, UNIFORM_ALIGNMENT);
	return 1;
}

// --- the pipeline
typedef struct { int done; int failed; } Scope;
// a shader that does not compile, as every host says it (web/playground/host-gpu.js checkedModule): why at its place,
// `the shader: expected `;`, found "}" at 4:1`, out of naga's report "Shader '' parsing error: why … ┌─ wgsl:4:1 …"
// (a shader block's place in the program, lib/prelude.warp shader_words); naga's whole report where it says neither
#define NAGA_CAUSE "error: "
#define NAGA_PLACE "wgsl:"
static void shader_failed(WGPUStringView message) {
	char report[sizeof failure];
	snprintf(report, sizeof report, "%.*s", (int)(message.data ? message.length : 0), message.data ? message.data : "");
	const char* why = strstr(report, NAGA_CAUSE);
	const char* place = strstr(report, NAGA_PLACE);
	if (!why || !place) { failed("the shader: ", message); return; }
	why += strlen(NAGA_CAUSE);
	place += strlen(NAGA_PLACE);
	snprintf(failure, sizeof failure, "the shader: %.*s at %.*s", (int)strcspn(why, "\n"), why, (int)strspn(place, "0123456789:"), place);
}
static void scope_popped(WGPUPopErrorScopeStatus status, WGPUErrorType type, WGPUStringView message, void* scope, void* unused) {
	if (type != WGPUErrorType_NoError) { shader_failed(message); ((Scope*)scope)->failed = 1; }
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
// the name of the shader's own vertex stage, `@vertex fn <name>(…`, into `name` (NAME_BYTES); 0 where it has none
static int own_vertex_stage(const char* shader, char* name) {
	const char* at = strstr(shader, OWN_VERTEX_STAGE);
	if (!at) return 0;
	at = strstr(at, "fn");
	if (!at) return 0;
	at = spaces_skipped(at + 2);
	int length = 0;
	while (name_char(at[length]) && length < NAME_BYTES - 1) { name[length] = at[length]; length++; }
	name[length] = 0;
	return length > 0;
}
// the code's pipeline drawing into the format, compiled anew; NULL and the failure told where it does not compile
static WGPURenderPipeline pipeline_compiled(const char* code, WGPUTextureFormat format) {
	wgpuDevicePushErrorScope(device, WGPUErrorFilter_Validation);
	WGPUShaderSourceWGSL source = WGPU_SHADER_SOURCE_WGSL_INIT;
	source.code = view_of(code);
	WGPUShaderModuleDescriptor module_descriptor = WGPU_SHADER_MODULE_DESCRIPTOR_INIT;
	module_descriptor.nextInChain = &source.chain;
	WGPUShaderModule module = wgpuDeviceCreateShaderModule(device, &module_descriptor);
	WGPUColorTargetState target = WGPU_COLOR_TARGET_STATE_INIT;
	target.format = format;
	char vertex[NAME_BYTES];
	int quads = own_vertex_stage(code, vertex);
	// a quad's color over the ones drawn before it: source alpha, as `vec4f(color, alpha)` says
	WGPUBlendState over = {
		.color = {.operation = WGPUBlendOperation_Add, .srcFactor = WGPUBlendFactor_SrcAlpha, .dstFactor = WGPUBlendFactor_OneMinusSrcAlpha},
		.alpha = {.operation = WGPUBlendOperation_Add, .srcFactor = WGPUBlendFactor_One, .dstFactor = WGPUBlendFactor_OneMinusSrcAlpha}};
	if (quads) target.blend = &over;
	WGPUFragmentState fragment = WGPU_FRAGMENT_STATE_INIT;
	fragment.module = module;
	fragment.entryPoint = view_of(FRAGMENT_ENTRY_POINT);
	fragment.targetCount = 1;
	fragment.targets = &target;
	WGPURenderPipelineDescriptor descriptor = WGPU_RENDER_PIPELINE_DESCRIPTOR_INIT;
	descriptor.vertex.module = module;
	descriptor.vertex.entryPoint = view_of(quads ? vertex : VERTEX_ENTRY_POINT);
	if (quads) descriptor.primitive.topology = WGPUPrimitiveTopology_TriangleStrip;
	descriptor.fragment = &fragment;
	WGPURenderPipeline pipeline = wgpuDeviceCreateRenderPipeline(device, &descriptor);
	wgpuShaderModuleRelease(module);
	if (scope_clean()) return pipeline;
	if (pipeline) wgpuRenderPipelineRelease(pipeline);
	return NULL;
}
static WGPURenderPipeline pipeline_of(const char* code, WGPUTextureFormat format) {
	if (compiled_code && compiled_format == format && !strcmp(compiled_code, code)) return compiled_pipeline;
	WGPURenderPipeline pipeline = pipeline_compiled(code, format);
	if (!pipeline) return NULL;
	if (compiled_pipeline) wgpuRenderPipelineRelease(compiled_pipeline);
	free(compiled_code);
	compiled_code = strdup(code);
	compiled_format = format;
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

static WGPUBindGroup bindings_of(WGPURenderPipeline pipeline, const WGPUBindGroupEntry* entries, size_t count) {
	WGPUBindGroupDescriptor group = WGPU_BIND_GROUP_DESCRIPTOR_INIT;
	group.layout = wgpuRenderPipelineGetBindGroupLayout(pipeline, 0);
	group.entryCount = count;
	group.entries = entries;
	WGPUBindGroup bindings = wgpuDeviceCreateBindGroup(device, &group);
	wgpuBindGroupLayoutRelease(group.layout);
	return bindings;
}
static WGPUTexture texture_of(int32_t width, int32_t height, WGPUTextureUsage usage) {
	WGPUTextureDescriptor descriptor = WGPU_TEXTURE_DESCRIPTOR_INIT;
	descriptor.size = (WGPUExtent3D){(uint32_t)width, (uint32_t)height, 1};
	descriptor.format = WGPUTextureFormat_RGBA8Unorm;
	descriptor.usage = usage;
	return wgpuDeviceCreateTexture(device, &descriptor);
}

// --- `$previous(at)` (card shader-read): the color the shader left at pixel `at` in its previous frame of this size,
// black before the first, kept on the GPU, as web/playground/host-gpu.js PREVIOUS_FRAME keeps it
#define PREVIOUS_HOLE VALUES_NAME ".previous"
#define PREVIOUS_FUNCTION "warp_previous"
#define PREVIOUS_FRAME "\n@group(0) @binding(1) var warp_previous_frame: texture_2d<f32>;\n" \
	"fn " PREVIOUS_FUNCTION "(at: vec2f) -> vec4f {\n" \
	"\tlet last = vec2i(textureDimensions(warp_previous_frame)) - vec2i(1);\n" \
	"\treturn textureLoad(warp_previous_frame, clamp(vec2i(at), vec2i(0), last), 0);\n}"
enum { PREVIOUS_FRAME_BINDING = 1 };
static char* previous_code;
static WGPUTexture previous_image;
static int32_t previous_width, previous_height;
// whether the shader names `name` whole (not as the start of a longer name)
static int names(const char* shader, const char* name) {
	for (const char* at = strstr(shader, name); at; at = strstr(at + 1, name))
		if ((at == shader || !name_char(at[-1])) && !name_char(at[strlen(name)])) return 1;
	return 0;
}
static int reads_previous(const char* shader) { return names(shader, PREVIOUS_HOLE); }
// the shader with each whole `from` written `to`; the caller frees it
static char* renamed(const char* shader, const char* from, const char* to) {
	size_t from_length = strlen(from), to_length = strlen(to), uses = 0;
	for (const char* at = strstr(shader, from); at; at = strstr(at + 1, from)) uses++;
	char* code = malloc(strlen(shader) + uses * (to_length > from_length ? to_length - from_length : 0) + 1);
	char* into = code;
	for (const char* at = shader; *at;) {
		if (!strncmp(at, from, from_length) && !name_char(at[from_length]) && (at == shader || !name_char(at[-1]))) {
			memcpy(into, to, to_length);
			into += to_length;
			at += from_length;
		} else *into++ = *at++;
	}
	*into = 0;
	return code;
}
// the frame this shader drew last at this size, else a black one, kept as previous_image
static WGPUTexture previous_frame(const char* code, int32_t width, int32_t height) {
	if (previous_image && previous_width == width && previous_height == height && !strcmp(previous_code, code)) return previous_image;
	if (previous_image) wgpuTextureRelease(previous_image);
	free(previous_code);
	previous_code = strdup(code);
	previous_width = width;
	previous_height = height;
	return previous_image = texture_of(width, height, WGPUTextureUsage_TextureBinding);
}
// the image just drawn becomes the shader's previous frame
static void previous_kept(WGPUTexture image) {
	if (previous_image) wgpuTextureRelease(previous_image);
	previous_image = image;
}

// --- a drawing: the shader's pipeline for a target format, its values' uniform and its previous frame bound
// corners and instances: a full image's triangle once, or the quads of a shader with its own vertex stage
typedef struct { WGPURenderPipeline pipeline; WGPUBuffer uniform; WGPUBindGroup bindings; WGPUTextureView previous; int reads_previous; int corners; int instances;
	WGPUBuffer storages[MOST_STORAGE_ARRAYS]; int storage_count; } Drawing;
static int drawing_made(const char* shader, const char* values, WGPUTextureFormat format, int32_t width, int32_t height, Drawing* drawing) {
	*drawing = (Drawing){0};
	failure[0] = 0;
	if (!device_ready()) return 0;
	static Layout layout;
	layout_cleared(&layout);
	if (!values_laid_out(values, &layout)) return 0;
	char vertex[NAME_BYTES];
	int quads = own_vertex_stage(shader, vertex);
	drawing->corners = quads ? QUAD_CORNERS : TRIANGLE_CORNERS;
	drawing->instances = quads ? layout.instances : 1;
	drawing->reads_previous = reads_previous(shader);
	char* body = drawing->reads_previous ? renamed(shader, PREVIOUS_HOLE, PREVIOUS_FUNCTION) : strdup(shader);
	// each storage array the shader reads: `values.mesh` its own array warp_values_mesh
	char storage_names[MOST_STORAGE_ARRAYS][NAME_BYTES + sizeof STORAGE_PREFIX];
	int read[MOST_STORAGE_ARRAYS], read_count = 0;
	for (int i = 0; i < layout.array_count; i++) {
		char hole[NAME_BYTES + sizeof VALUES_NAME + 1];
		snprintf(hole, sizeof hole, VALUES_NAME ".%s", layout.arrays[i].name);
		if (!names(body, hole)) continue;
		snprintf(storage_names[read_count], sizeof storage_names[0], STORAGE_PREFIX "%s", layout.arrays[i].name);
		char* named = renamed(body, hole, storage_names[read_count]);
		free(body);
		body = named;
		read[read_count++] = i;
	}
	size_t code_size = strlen(body) + sizeof FULL_IMAGE_VERTICES + sizeof PREVIOUS_FRAME + MOST_DECLARATIONS + 128 * (1 + read_count);
	char* code = malloc(code_size);
	snprintf(code, code_size, "%s%s", body, quads ? "" : FULL_IMAGE_VERTICES);
	free(body);
	if (layout.members) {
		size_t used = strlen(code);
		snprintf(code + used, code_size - used, "\nstruct WarpValues { %s }\n@group(0) @binding(%d) var<uniform> " VALUES_NAME ": WarpValues;",
			layout.declarations, VALUES_BINDING);
	}
	for (int i = 0; i < read_count; i++) {
		size_t used = strlen(code);
		snprintf(code + used, code_size - used, "\n@group(0) @binding(%d) var<storage, read> %s: array<vec4f>;", STORAGE_BINDING + i, storage_names[i]);
	}
	if (drawing->reads_previous) strcat(code, PREVIOUS_FRAME);
	drawing->pipeline = pipeline_of(code, format);
	WGPUBindGroupEntry entries[2 + MOST_STORAGE_ARRAYS];
	size_t count = 0;
	if (drawing->pipeline && layout.members) {
		uint64_t size = layout.count * sizeof(float);
		drawing->uniform = buffer_of(size, WGPUBufferUsage_Uniform | WGPUBufferUsage_CopyDst);
		wgpuQueueWriteBuffer(queue, drawing->uniform, 0, layout.floats, size);
		entries[count] = (WGPUBindGroupEntry)WGPU_BIND_GROUP_ENTRY_INIT;
		entries[count].binding = VALUES_BINDING;
		entries[count].buffer = drawing->uniform;
		entries[count++].size = size;
	}
	if (drawing->pipeline && drawing->reads_previous) {
		drawing->previous = wgpuTextureCreateView(previous_frame(code, width, height), NULL);
		entries[count] = (WGPUBindGroupEntry)WGPU_BIND_GROUP_ENTRY_INIT;
		entries[count].binding = PREVIOUS_FRAME_BINDING;
		entries[count++].textureView = drawing->previous;
	}
	for (int i = 0; drawing->pipeline && i < read_count; i++) {
		Floats* numbers = &layout.arrays[read[i]].numbers;
		uint64_t size = numbers->count * sizeof(float);
		WGPUBuffer storage = drawing->storages[drawing->storage_count++] = buffer_of(size, WGPUBufferUsage_Storage | WGPUBufferUsage_CopyDst);
		wgpuQueueWriteBuffer(queue, storage, 0, numbers->items, size);
		entries[count] = (WGPUBindGroupEntry)WGPU_BIND_GROUP_ENTRY_INIT;
		entries[count].binding = STORAGE_BINDING + i;
		entries[count].buffer = storage;
		entries[count++].size = size;
	}
	free(code);
	if (!drawing->pipeline) return 0;
	if (count) drawing->bindings = bindings_of(drawing->pipeline, entries, count);
	return 1;
}
static void drawing_released(Drawing* drawing) {
	if (drawing->bindings) wgpuBindGroupRelease(drawing->bindings);
	if (drawing->uniform) wgpuBufferRelease(drawing->uniform);
	for (int i = 0; i < drawing->storage_count; i++) wgpuBufferRelease(drawing->storages[i]);
	if (drawing->previous) wgpuTextureViewRelease(drawing->previous);
}
// where in a target a drawing lands: x, y, width, height in the target's pixels
typedef struct { float x, y, width, height; } Viewport;
// the drawing's full image drawn into the view, by the encoder, onto the whole view or only the viewport given
static void drawn_into(WGPUCommandEncoder encoder, WGPUTextureView view, const Drawing* drawing, const Viewport* viewport) {
	WGPURenderPassColorAttachment attachment = WGPU_RENDER_PASS_COLOR_ATTACHMENT_INIT;
	attachment.view = view;
	attachment.loadOp = WGPULoadOp_Clear;
	attachment.storeOp = WGPUStoreOp_Store;
	WGPURenderPassDescriptor pass_descriptor = WGPU_RENDER_PASS_DESCRIPTOR_INIT;
	pass_descriptor.colorAttachmentCount = 1;
	pass_descriptor.colorAttachments = &attachment;
	WGPURenderPassEncoder pass = wgpuCommandEncoderBeginRenderPass(encoder, &pass_descriptor);
	wgpuRenderPassEncoderSetPipeline(pass, drawing->pipeline);
	if (viewport) wgpuRenderPassEncoderSetViewport(pass, viewport->x, viewport->y, viewport->width, viewport->height, 0, 1);
	if (drawing->bindings) wgpuRenderPassEncoderSetBindGroup(pass, 0, drawing->bindings, 0, NULL);
	if (drawing->instances > 0) wgpuRenderPassEncoderDraw(pass, drawing->corners, drawing->instances, 0, 0);
	wgpuRenderPassEncoderEnd(pass);
	wgpuRenderPassEncoderRelease(pass);
}
static void drawn(WGPUCommandEncoder encoder, WGPUTextureView view, const Drawing* drawing) { drawn_into(encoder, view, drawing, NULL); }
static void submitted(WGPUCommandEncoder encoder) {
	WGPUCommandBuffer commands = wgpuCommandEncoderFinish(encoder, NULL);
	wgpuQueueSubmit(queue, 1, &commands);
	wgpuCommandBufferRelease(commands);
	wgpuCommandEncoderRelease(encoder);
}
// the drawing drawn into an image of its own (RGBA8), by the encoder; one a later frame reads as previous can be bound
static WGPUTexture image_drawn(WGPUCommandEncoder encoder, const Drawing* drawing, int32_t width, int32_t height) {
	WGPUTexture image = texture_of(width, height, WGPUTextureUsage_RenderAttachment | WGPUTextureUsage_CopySrc | WGPUTextureUsage_TextureBinding);
	WGPUTextureView view = wgpuTextureCreateView(image, NULL);
	drawn(encoder, view, drawing);
	wgpuTextureViewRelease(view);
	return image;
}
// the image kept as the drawing's previous frame, else let go
static void image_done(const Drawing* drawing, WGPUTexture image) {
	if (drawing->reads_previous) previous_kept(image); else wgpuTextureRelease(image);
}

// the shader's pixels, row by row as 0xAARRGGBB, into `pixels` (width × height); 0, else -1 and warp_gpu_failure()
int warp_gpu_paint(const char* shader, const char* values, int32_t width, int32_t height, uint32_t* pixels) {
	Drawing drawing;
	if (!drawing_made(shader, values, WGPUTextureFormat_RGBA8Unorm, width, height, &drawing)) return -1;
	WGPUCommandEncoder encoder = wgpuDeviceCreateCommandEncoder(device, NULL);
	WGPUTexture image = image_drawn(encoder, &drawing, width, height);

	uint32_t row_bytes = (width * PIXEL_BYTES + ROW_ALIGNMENT - 1) / ROW_ALIGNMENT * ROW_ALIGNMENT;
	WGPUBuffer readback = buffer_of((uint64_t)row_bytes * height, WGPUBufferUsage_MapRead | WGPUBufferUsage_CopyDst);
	WGPUTexelCopyTextureInfo from = WGPU_TEXEL_COPY_TEXTURE_INFO_INIT;
	from.texture = image;
	WGPUTexelCopyBufferInfo into = WGPU_TEXEL_COPY_BUFFER_INFO_INIT;
	into.buffer = readback;
	into.layout.bytesPerRow = row_bytes;
	into.layout.rowsPerImage = (uint32_t)height;
	WGPUExtent3D extent = {(uint32_t)width, (uint32_t)height, 1};
	wgpuCommandEncoderCopyTextureToBuffer(encoder, &from, &into, &extent);
	submitted(encoder);

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
	drawing_released(&drawing);
	image_done(&drawing, image);
	return done > 0 ? 0 : -1;
}

// --- a window's surface (lib/native_window.warp's Metal layer): what is shown there stays on the GPU, no pixel read back.
// The surface is as big as the layer's drawable (the window's pixels, which SDL keeps it at as the window resizes or
// goes full screen); the painting, drawn at its own size, is scaled into it whole, its aspect kept, the rest black.
// A format without sRGB, so the window shows the colors a PNG of the same painting holds
static const WGPUTextureFormat SURFACE_FORMAT = WGPUTextureFormat_BGRA8Unorm;
#ifdef __APPLE__
#include <objc/message.h>
typedef struct { double width, height; } LayerSize; // CGSize
// the Metal layer's drawableSize, in pixels
static void drawable_size(void* layer, int32_t* width, int32_t* height) {
	LayerSize size = ((LayerSize (*)(void*, SEL))objc_msgSend)(layer, sel_registerName("drawableSize"));
	if (size.width >= 1 && size.height >= 1) *width = (int32_t)size.width, *height = (int32_t)size.height;
}
#else
static void drawable_size(void* layer, int32_t* width, int32_t* height) {}
#endif
// the window's surface at the layer's size, else the painting's
static int surface_ready(void* layer, int32_t width, int32_t height) {
	if (!device_ready()) return 0;
	drawable_size(layer, &width, &height);
	if (surface && surface_layer != layer) { wgpuSurfaceRelease(surface); surface = NULL; }
	if (!surface) {
		WGPUSurfaceSourceMetalLayer source = WGPU_SURFACE_SOURCE_METAL_LAYER_INIT;
		source.layer = layer;
		WGPUSurfaceDescriptor descriptor = WGPU_SURFACE_DESCRIPTOR_INIT;
		descriptor.nextInChain = &source.chain;
		surface = wgpuInstanceCreateSurface(instance, &descriptor);
		if (!surface) return snprintf(failure, sizeof failure, "the window offers no surface to show on"), 0;
		surface_layer = layer;
		surface_width = surface_height = 0;
	}
	if (surface_width != (uint32_t)width || surface_height != (uint32_t)height) {
		WGPUSurfaceConfiguration configuration = WGPU_SURFACE_CONFIGURATION_INIT;
		configuration.device = device;
		configuration.format = SURFACE_FORMAT;
		configuration.width = surface_width = (uint32_t)width;
		configuration.height = surface_height = (uint32_t)height;
		configuration.presentMode = WGPUPresentMode_Fifo;
		wgpuSurfaceConfigure(surface, &configuration);
	}
	return 1;
}
// the last painting shown, its image kept on the GPU, shown again as the window comes into sight or changes size
// (warp_gpu_shown_again): a window opens behind the app in front, and a still picture paints once
typedef struct { WGPUTexture picture; void* layer; int32_t width, height; } Shown;
static Shown last_shown;
static void shown_kept(WGPUTexture picture, void* layer, int32_t width, int32_t height) {
	if (picture == last_shown.picture) return;
	wgpuTextureAddRef(picture);
	if (last_shown.picture) wgpuTextureRelease(last_shown.picture);
	last_shown = (Shown){picture, layer, width, height};
}
// the drawing shown in the window's surface, by the encoder, waiting for the display's next refresh (vsync); a window
// out of sight (hidden, minimized, covered) shows nothing and is no failure
static int presented(WGPUCommandEncoder encoder, const Drawing* drawing, const Viewport* viewport) {
	WGPUSurfaceTexture current = WGPU_SURFACE_TEXTURE_INIT;
	wgpuSurfaceGetCurrentTexture(surface, &current);
	if (current.status == (WGPUSurfaceGetCurrentTextureStatus)WGPUSurfaceGetCurrentTextureStatus_Occluded) return submitted(encoder), 1;
	if (current.status != WGPUSurfaceGetCurrentTextureStatus_SuccessOptimal && current.status != WGPUSurfaceGetCurrentTextureStatus_SuccessSuboptimal) {
		snprintf(failure, sizeof failure, "the window's surface gave no image to draw (status %d)", (int)current.status);
		surface_width = 0;
		submitted(encoder);
		return 0;
	}
	WGPUTextureView view = wgpuTextureCreateView(current.texture, NULL);
	drawn_into(encoder, view, drawing, viewport);
	submitted(encoder);
	wgpuSurfacePresent(surface);
	wgpuTextureViewRelease(view);
	wgpuTextureRelease(current.texture);
	return 1;
}
// the picture spread over the viewport it is drawn in, each window pixel taking the picture's pixel under it (sharp
// pixels, as the PNG has them)
#define PICTURE_SHADER "@group(0) @binding(0) var picture: texture_2d<f32>;\n" \
	"struct Corner { @builtin(position) position: vec4f, @location(0) across: vec2f }\n" \
	"@vertex fn " VERTEX_ENTRY_POINT "(@builtin(vertex_index) corner: u32) -> Corner {\n" \
	"\tlet at = vec2f(f32(corner / 2u) * 4.0 - 1.0, f32(corner % 2u) * 4.0 - 1.0);\n" \
	"\treturn Corner(vec4f(at, 0.0, 1.0), vec2f(at.x + 1.0, 1.0 - at.y) * 0.5);\n}\n" \
	"@fragment fn " FRAGMENT_ENTRY_POINT "(corner: Corner) -> @location(0) vec4f {\n" \
	"\tlet size = vec2i(textureDimensions(picture));\n" \
	"\treturn textureLoad(picture, clamp(vec2i(corner.across * vec2f(size)), vec2i(0), size - vec2i(1)), 0);\n}\n"
// the painting's place in the surface: as large as fits whole, centered
static Viewport letterboxed(int32_t width, int32_t height) {
	float scale = fminf((float)surface_width / width, (float)surface_height / height);
	float shown_width = width * scale, shown_height = height * scale;
	return (Viewport){(surface_width - shown_width) / 2, (surface_height - shown_height) / 2, shown_width, shown_height};
}
// the picture (an RGBA8 texture) shown in the window's surface, scaled to it, after what the encoder holds, and kept
static int picture_presented(WGPUCommandEncoder encoder, WGPUTexture picture, void* layer, int32_t width, int32_t height) {
	shown_kept(picture, layer, width, height);
	static WGPURenderPipeline picture_pipeline;
	if (!picture_pipeline) picture_pipeline = pipeline_compiled(PICTURE_SHADER, SURFACE_FORMAT);
	if (!picture_pipeline) return submitted(encoder), 0;
	WGPUTextureView picture_view = wgpuTextureCreateView(picture, NULL);
	WGPUBindGroupEntry entry = WGPU_BIND_GROUP_ENTRY_INIT;
	entry.binding = 0;
	entry.textureView = picture_view;
	Drawing drawing = {.pipeline = picture_pipeline, .bindings = bindings_of(picture_pipeline, &entry, 1)};
	Viewport viewport = letterboxed(width, height);
	int shown = presented(encoder, &drawing, &viewport);
	drawing_released(&drawing);
	wgpuTextureViewRelease(picture_view);
	return shown;
}
// the shader shown in the window's layer at width × height; 0, else -1 and warp_gpu_failure(). It draws into an image
// of the painting's size first (kept for the next frame by a shader reading its previous one), then shown scaled
int warp_gpu_show(const char* shader, const char* values, void* layer, int32_t width, int32_t height) {
	Drawing drawing;
	if (!drawing_made(shader, values, WGPUTextureFormat_RGBA8Unorm, width, height, &drawing) || !surface_ready(layer, width, height)) return -1;
	WGPUCommandEncoder encoder = wgpuDeviceCreateCommandEncoder(device, NULL);
	WGPUTexture image = image_drawn(encoder, &drawing, width, height);
	int shown = picture_presented(encoder, image, layer, width, height);
	image_done(&drawing, image);
	drawing_released(&drawing);
	return shown ? 0 : -1;
}
// pixels (RGBA, a byte each, row by row) shown in the window's layer at width × height; 0, else -1
int warp_gpu_show_pixels(const uint8_t* pixels, void* layer, int32_t width, int32_t height) {
	if (!surface_ready(layer, width, height)) return -1;
	WGPUTexture picture = texture_of(width, height, WGPUTextureUsage_TextureBinding | WGPUTextureUsage_CopyDst);
	WGPUTexelCopyTextureInfo into = WGPU_TEXEL_COPY_TEXTURE_INFO_INIT;
	into.texture = picture;
	WGPUTexelCopyBufferLayout rows = WGPU_TEXEL_COPY_BUFFER_LAYOUT_INIT;
	rows.bytesPerRow = (uint32_t)width * PIXEL_BYTES;
	rows.rowsPerImage = (uint32_t)height;
	WGPUExtent3D extent = {(uint32_t)width, (uint32_t)height, 1};
	wgpuQueueWriteTexture(queue, &into, pixels, (size_t)width * height * PIXEL_BYTES, &rows, &extent);
	int shown = picture_presented(wgpuDeviceCreateCommandEncoder(device, NULL), picture, layer, width, height);
	wgpuTextureRelease(picture);
	return shown ? 0 : -1;
}
// the last painting shown again, at the window's size now (lib/native_window.warp asks when its window comes into sight or
// changes size); 0 also when none was shown, else -1
int warp_gpu_shown_again(void) {
	Shown again = last_shown;
	if (!again.picture) return 0;
	if (!surface_ready(again.layer, again.width, again.height)) return -1;
	return picture_presented(wgpuDeviceCreateCommandEncoder(device, NULL), again.picture, again.layer, again.width, again.height) ? 0 : -1;
}

// --- gpu_compute(shader, arrays, workgroups) (card host-word): a WGSL compute shader, its entry point `main`, run over
// storage arrays as host-gpu.js gpuComputedArrays runs it. `arrays` a map of named lists, each bound where the shader
// declares the storage array of that name, f32 unless array<i32> or array<u32>; the arrays it left come back as a map,
// in the order given. `arrays` a list: the numbers at @binding(0), they come back as a list
enum { MOST_ARRAYS = 8 };
// an array's numbers: a GPU list's cells where they are (cells of their own type, its node kept to give back), else a
// list's read; element and cell 'f', 'i' or 'u' (f32, i32, u32)
typedef struct { char name[NAME_BYTES]; int binding; char element; double* numbers; void* cells; char cell; wasmtime_anyref_t node; size_t count; } StorageArray;
// a four-byte cell of that type, read and written as a number
static double cell_read(const void* cells, char type, size_t i) {
	return type == 'f' ? ((const float*)cells)[i] : type == 'u' ? ((const uint32_t*)cells)[i] : ((const int32_t*)cells)[i];
}
static void cell_written(void* cells, char type, size_t i, double number) {
	if (type == 'f') ((float*)cells)[i] = (float)number;
	else if (type == 'u') ((uint32_t*)cells)[i] = (uint32_t)number;
	else ((int32_t*)cells)[i] = (int32_t)number;
}
// the storage arrays the shader declares: `@binding(1) var<storage, read_write> counts: array<i32>`
static int storage_arrays(const char* shader, StorageArray* arrays) {
	int found = 0;
	for (const char* at = strstr(shader, "var<storage"); at && found < MOST_ARRAYS; at = strstr(at + 1, "var<storage")) {
		const char* binding = NULL;
		for (const char* back = at; back > shader && back > at - 64; back--) if (!strncmp(back, "@binding(", 9)) { binding = back + 9; break; }
		const char* name = strchr(at, '>');
		if (!name || !binding) continue;
		name = spaces_skipped(name + 1);
		StorageArray* array = &arrays[found];
		*array = (StorageArray){0};
		int length = 0;
		while (name[length] && name[length] != ':' && name[length] != ' ' && length < NAME_BYTES - 1) { array->name[length] = name[length]; length++; }
		array->name[length] = 0;
		array->binding = atoi(binding);
		const char* element = strstr(name, "array<");
		array->element = element && (!strncmp(element + 6, "i32", 3) || !strncmp(element + 6, "u32", 3)) ? element[6] : 'f';
		found++;
	}
	return found;
}
// the numbers of a list given, as doubles
static double* numbers_of(warp_values* values, const wasmtime_anyref_t* list, size_t* count) {
	*count = warp_list_length(values, list);
	double* numbers = malloc((*count ? *count : 1) * sizeof *numbers);
	for (size_t i = 0; i < *count; i++) {
		wasmtime_anyref_t item = warp_list_item(values, list, i);
		numbers[i] = warp_node_float(values, &item);
		wasmtime_anyref_unroot(&item);
	}
	return numbers;
}
// a list given into its array: a GPU list's cells as they are, any other list's numbers read
static void array_given(warp_values* values, const wasmtime_anyref_t* list, StorageArray* array) {
	array->cells = warp_cells(values, list, &array->count, &array->cell);
	if (array->cells) wasmtime_anyref_clone(list, &array->node);
	else array->numbers = numbers_of(values, list, &array->count);
}
static StorageArray* array_named(StorageArray* arrays, int count, const char* name) {
	for (int i = 0; i < count; i++) if (!strcmp(arrays[i].name, name)) return &arrays[i];
	return NULL;
}
static int given_failed(warp_values* values, const char* message, const char* name) {
	char said[NAME_BYTES + 128];
	snprintf(said, sizeof said, message, name);
	warp_failed(values, said);
	return 0;
}
// the given arrays read into the declared ones, in the order given (order); 0 and a failure where they do not fit
static int arrays_given(warp_values* values, const wasmtime_anyref_t* given, StorageArray* declared, int declared_count, int* order) {
	int64_t kind = warp_kind_field(values, given);
	if (kind != WARP_MARKED_MAP) {
		StorageArray* first = NULL;
		for (int i = 0; i < declared_count; i++) if (declared[i].binding == 0) first = &declared[i];
		if ((kind & WARP_KIND_MASK) != WARP_KIND_LIST) return given_failed(values, "gpu_compute takes a list of numbers or a map of named lists%s", "");
		if (!first) return given_failed(values, "gpu_compute of a list: the shader declares no storage array at @binding(0)%s", "");
		array_given(values, given, first);
		order[0] = (int)(first - declared);
		return 1;
	}
	int given_count = (int)warp_list_length(values, given);
	if (given_count > MOST_ARRAYS) return given_failed(values, "gpu_compute binds at most 8 arrays%s", "");
	for (int i = 0; i < given_count; i++) {
		wasmtime_anyref_t entry = warp_list_item(values, given, (size_t)i);
		wasmtime_anyref_t key = warp_key_left(values, &entry);
		wasmtime_anyref_t list = warp_key_right(values, &entry);
		char* name = warp_text_bytes(values, &key, NULL);
		StorageArray* array = array_named(declared, declared_count, name);
		if (array) {
			array_given(values, &list, array);
			order[i] = (int)(array - declared);
		}
		wasmtime_anyref_unroot(&list);
		wasmtime_anyref_unroot(&key);
		wasmtime_anyref_unroot(&entry);
		if (!array) { given_failed(values, "the shader declares no storage array %s", name); free(name); return 0; }
		free(name);
		if (!array->count) return given_failed(values, "%s is empty: a storage array holds at least one number", array->name);
	}
	for (int i = 0; i < declared_count; i++)
		if (!declared[i].numbers && !declared[i].cells) return given_failed(values, "the shader's array %s has no numbers", declared[i].name);
	return values->failure[0] ? 0 : given_count;
}
static void element_written(const StorageArray* array, uint8_t* bytes) {
	if (array->cells && array->cell == array->element) { memcpy(bytes, array->cells, array->count * 4); return; }
	for (size_t i = 0; i < array->count; i++)
		cell_written(bytes, array->element, i, array->cells ? cell_read(array->cells, array->cell, i) : array->numbers[i]);
}
// the shader run over the arrays, the arrays it left in `left` (the caller frees them): 0 and failure where it fails
static int arrays_computed(const char* shader, const StorageArray* declared, const int* order, int count, int64_t workgroups, uint8_t** left) {
	int read = 0;
	{
		wgpuDevicePushErrorScope(device, WGPUErrorFilter_Validation);
		WGPUShaderSourceWGSL source = WGPU_SHADER_SOURCE_WGSL_INIT;
		source.code = view_of(shader);
		WGPUShaderModuleDescriptor module_descriptor = WGPU_SHADER_MODULE_DESCRIPTOR_INIT;
		module_descriptor.nextInChain = &source.chain;
		WGPUShaderModule module = wgpuDeviceCreateShaderModule(device, &module_descriptor);
		WGPUComputePipelineDescriptor descriptor = WGPU_COMPUTE_PIPELINE_DESCRIPTOR_INIT;
		descriptor.compute.module = module;
		descriptor.compute.entryPoint = view_of("main");
		WGPUComputePipeline pipeline = wgpuDeviceCreateComputePipeline(device, &descriptor);
		wgpuShaderModuleRelease(module);
		if (scope_clean()) {
			WGPUBuffer storages[MOST_ARRAYS], readbacks[MOST_ARRAYS];
			WGPUBindGroupEntry entries[MOST_ARRAYS];
			for (int a = 0; a < count; a++) {
				const StorageArray* array = &declared[order[a]];
				uint64_t size = array->count * 4;
				uint8_t* bytes = malloc(size);
				element_written(array, bytes);
				storages[a] = buffer_of(size, WGPUBufferUsage_Storage | WGPUBufferUsage_CopySrc | WGPUBufferUsage_CopyDst);
				wgpuQueueWriteBuffer(queue, storages[a], 0, bytes, size);
				free(bytes);
				readbacks[a] = buffer_of(size, WGPUBufferUsage_MapRead | WGPUBufferUsage_CopyDst);
				entries[a] = (WGPUBindGroupEntry)WGPU_BIND_GROUP_ENTRY_INIT;
				entries[a].binding = (uint32_t)array->binding;
				entries[a].buffer = storages[a];
				entries[a].size = size;
			}
			WGPUBindGroupDescriptor group = WGPU_BIND_GROUP_DESCRIPTOR_INIT;
			group.layout = wgpuComputePipelineGetBindGroupLayout(pipeline, 0);
			group.entryCount = (size_t)count;
			group.entries = entries;
			WGPUBindGroup bindings = wgpuDeviceCreateBindGroup(device, &group);
			wgpuBindGroupLayoutRelease(group.layout);
			WGPUCommandEncoder encoder = wgpuDeviceCreateCommandEncoder(device, NULL);
			WGPUComputePassEncoder pass = wgpuCommandEncoderBeginComputePass(encoder, NULL);
			wgpuComputePassEncoderSetPipeline(pass, pipeline);
			wgpuComputePassEncoderSetBindGroup(pass, 0, bindings, 0, NULL);
			wgpuComputePassEncoderDispatchWorkgroups(pass, (uint32_t)workgroups, 1, 1);
			wgpuComputePassEncoderEnd(pass);
			wgpuComputePassEncoderRelease(pass);
			for (int a = 0; a < count; a++) wgpuCommandEncoderCopyBufferToBuffer(encoder, storages[a], 0, readbacks[a], 0, declared[order[a]].count * 4);
			submitted(encoder);
			read = 1;
			for (int a = 0; a < count; a++) {
				size_t size = declared[order[a]].count * 4;
				int done = 0;
				WGPUBufferMapCallbackInfo map_info = WGPU_BUFFER_MAP_CALLBACK_INFO_INIT;
				map_info.mode = WGPUCallbackMode_AllowProcessEvents;
				map_info.callback = mapped;
				map_info.userdata1 = &done;
				wgpuBufferMapAsync(readbacks[a], WGPUMapMode_Read, 0, size, map_info);
				while (!done) { wgpuDevicePoll(device, 1, NULL); wgpuInstanceProcessEvents(instance); }
				left[a] = malloc(size);
				if (done > 0) { memcpy(left[a], wgpuBufferGetConstMappedRange(readbacks[a], 0, size), size); wgpuBufferUnmap(readbacks[a]); }
				else read = 0;
			}
			for (int a = 0; a < count; a++) { wgpuBufferRelease(storages[a]); wgpuBufferRelease(readbacks[a]); }
			wgpuBindGroupRelease(bindings);
		}
		if (pipeline) wgpuComputePipelineRelease(pipeline);
	}
	return read;
}
// the arrays the shader left as warp values: a map in the order given, else (a list given) the one list
static wasmtime_anyref_t arrays_written(warp_values* values, const StorageArray* declared, const int* order, int count, int as_map, uint8_t** left) {
	wasmtime_anyref_t lists[MOST_ARRAYS];
	for (int a = 0; a < count; a++) {
		const StorageArray* array = &declared[order[a]];
		if (array->cells) {
			// a GPU list takes what the shader left in its own cells, in place
			if (array->cell == array->element) memcpy(array->cells, left[a], array->count * 4);
			else for (size_t i = 0; i < array->count; i++) cell_written(array->cells, array->cell, i, cell_read(left[a], array->element, i));
			lists[a] = array->node;
			if (as_map) lists[a] = warp_entry(values, array->name, lists[a]);
			continue;
		}
		wasmtime_anyref_t* numbers = malloc(array->count * sizeof *numbers);
		for (size_t i = 0; i < array->count; i++) {
			double number = cell_read(left[a], array->element, i);
			numbers[i] = array->element == 'f' ? warp_float(values, number) : warp_int(values, (int64_t)number);
		}
		lists[a] = warp_list(values, WARP_MARKED_SQUARELIST, numbers, array->count);
		free(numbers);
		if (as_map) lists[a] = warp_entry(values, array->name, lists[a]);
	}
	return as_map ? warp_list(values, WARP_MARKED_MAP, lists, (size_t)count) : lists[0];
}
// gpu_compute(shader, arrays, workgroups), bound by the driver (lib/native/foreign.c library_word)
wasm_trap_t* warp_word_gpu_compute(void* engine, wasmtime_caller_t* caller, const wasmtime_val_t* given, size_t given_count, wasmtime_val_t* results, size_t result_count) {
	(void)given_count, (void)result_count;
	warp_values values = warp_values_of(engine, caller, &given[1].of.anyref);
	wasmtime_anyref_t answer;
	wasmtime_anyref_set_null(&answer);
	char* shader = warp_text_bytes(&values, &given[0].of.anyref, NULL);
	StorageArray declared[MOST_ARRAYS];
	int order[MOST_ARRAYS];
	int declared_count = storage_arrays(shader, declared);
	int count = values.failure[0] ? 0 : arrays_given(&values, &given[1].of.anyref, declared, declared_count, order);
	uint8_t* left[MOST_ARRAYS] = {0};
	failure[0] = 0;
	if (count && device_ready() && arrays_computed(shader, declared, order, count, given[2].of.i64, left))
		answer = arrays_written(&values, declared, order, count, warp_kind_field(&values, &given[1].of.anyref) == WARP_MARKED_MAP, left);
	else if (count) warp_failed(&values, failure);
	for (int a = 0; a < count; a++) free(left[a]);
	for (int i = 0; i < declared_count; i++) {
		free(declared[i].numbers);
		// (a GPU list the answer did not take back)
		if (declared[i].cells && wasmtime_anyref_is_null(&answer)) wasmtime_anyref_unroot(&declared[i].node);
	}
	free(shader);
	return warp_answered(&values, answer, results);
}

// why the last warp_gpu_paint or warp_gpu_show failed
const char* warp_gpu_failure(void) { return failure; }
