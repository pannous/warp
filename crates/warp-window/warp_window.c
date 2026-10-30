// libwarp_window (card native-window): the native warp's one window, over SDL3. It answers the system values it owns
// (warp_system_value) and takes the ones a program sets (warp_set_system_value); lib/native/foreign.c forwards those
// the driver does not own here, and asks for the window's Metal layer, which libwarp_gpu shows paintings in.
// `window_open = 1` or the first painting opens it, never under WARP_NO_WINDOW or CI. Each painting is a frame, its
// title telling the frame times; each painting and reading of window_open takes the window's events in. Closed, it
// ends a program painting on; a still picture (a run's one painting) stays shown after the run until it is closed.
// Built by scripts/wasmtime-c-api.sh where SDL3 is installed
#include <SDL3/SDL.h>
#include <SDL3/SDL_metal.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define WINDOW_TITLE "warp"
enum { DEFAULT_WIDTH = 640, DEFAULT_HEIGHT = 480, MOST_HELD = 16, TITLE_BYTES = 128, OUT_OF_SIGHT_MS = 16 };
static const uint64_t TITLE_EVERY_NS = 500000000;
// the codes lib/keys.warp KEY_NAMES gives the keys SDL codes above the characters (web/playground/canvas.js KEY_CODES)
static const SDL_Keycode SDL_KEYS[] = {SDLK_UP, SDLK_DOWN, SDLK_LEFT, SDLK_RIGHT, SDLK_LSHIFT, SDLK_RSHIFT, SDLK_LCTRL,
	SDLK_RCTRL, SDLK_LALT, SDLK_RALT, SDLK_LGUI, SDLK_RGUI};
static const int64_t WARP_KEYS[] = {63232, 63233, 63234, 63235, 63312, 63312, 63313, 63313, 63314, 63314, 63315, 63315};

static SDL_Window* window;
static SDL_MetalView view;
static int open_now, sized_by_program, closed_by_user;
static int64_t width = DEFAULT_WIDTH, height = DEFAULT_HEIGHT;
static int64_t held[MOST_HELD];
static int held_count;
static float mouse_x, mouse_y, wheel;
static int mouse_down;
static uint64_t frames, frames_since_title, last_frame_ns, title_ns;

static int windows_refused(void) { return getenv("WARP_NO_WINDOW") || getenv("CI"); }
static int named(const char* name, const char* wanted) { return !strcmp(name, wanted); }

static int64_t key_code(SDL_Keycode key) {
	for (size_t i = 0; i < sizeof SDL_KEYS / sizeof *SDL_KEYS; i++) if (SDL_KEYS[i] == key) return WARP_KEYS[i];
	return key & SDLK_SCANCODE_MASK ? 0 : (int64_t)key;
}
static void key_pressed(int64_t code) {
	for (int i = 0; i < held_count; i++) if (held[i] == code) return;
	if (code && held_count < MOST_HELD) held[held_count++] = code;
}
static void key_released(int64_t code) {
	for (int i = 0; i < held_count; i++)
		if (held[i] == code) { memmove(held + i, held + i + 1, (--held_count - i) * sizeof *held); return; }
}

static void held_open(void);
static void window_opened(void) {
	if (window || windows_refused()) return;
	static int holding;
	if (!holding) holding = 1, atexit(held_open);
	if (!SDL_Init(SDL_INIT_VIDEO)) { fprintf(stderr, "window: %s\n", SDL_GetError()); return; }
	window = SDL_CreateWindow(WINDOW_TITLE, (int)width, (int)height, SDL_WINDOW_METAL);
	if (!window) { fprintf(stderr, "window: %s\n", SDL_GetError()); return; }
	view = SDL_Metal_CreateView(window);
	open_now = 1;
	last_frame_ns = title_ns = SDL_GetTicksNS();
}
static void window_closed(void) {
	if (view) SDL_Metal_DestroyView(view);
	if (window) SDL_DestroyWindow(window);
	view = NULL;
	window = NULL;
	open_now = 0;
}
// the frame times in the title, every half second: the mean of the frames since
static void frame_timed(void) {
	uint64_t now = SDL_GetTicksNS();
	frames++;
	frames_since_title++;
	last_frame_ns = now;
	if (now - title_ns < TITLE_EVERY_NS) return;
	char title[TITLE_BYTES];
	double milliseconds = (now - title_ns) / 1e6 / frames_since_title;
	snprintf(title, sizeof title, WINDOW_TITLE " — %.1f ms a frame, %.0f fps", milliseconds, 1000 / milliseconds);
	SDL_SetWindowTitle(window, title);
	title_ns = now;
	frames_since_title = 0;
}
// the window's events since the last frame taken in
static void events_taken(void) {
	SDL_Event event;
	wheel = 0;
	while (SDL_PollEvent(&event)) {
		switch (event.type) {
		case SDL_EVENT_QUIT: case SDL_EVENT_WINDOW_CLOSE_REQUESTED: closed_by_user = 1; window_closed(); return;
		case SDL_EVENT_KEY_DOWN: key_pressed(key_code(event.key.key)); break;
		case SDL_EVENT_KEY_UP: key_released(key_code(event.key.key)); break;
		case SDL_EVENT_MOUSE_MOTION: mouse_x = event.motion.x; mouse_y = event.motion.y; break;
		case SDL_EVENT_MOUSE_BUTTON_DOWN: mouse_down = 1; break;
		case SDL_EVENT_MOUSE_BUTTON_UP: mouse_down = 0; break;
		case SDL_EVENT_MOUSE_WHEEL: wheel += event.wheel.y; break;
		}
	}
	// a window out of sight has no refresh to wait for: its frames idle instead of spinning
	if (SDL_GetWindowFlags(window) & (SDL_WINDOW_HIDDEN | SDL_WINDOW_MINIMIZED | SDL_WINDOW_OCCLUDED)) SDL_Delay(OUT_OF_SIGHT_MS);
}
// a run that painted one picture leaves it in the window until the window is closed (or Escape); an animation's
// window closes with its run
static void held_open(void) {
	SDL_Event event;
	while (open_now && frames == 1 && SDL_WaitEvent(&event)) {
		int escaped = event.type == SDL_EVENT_KEY_DOWN && event.key.key == SDLK_ESCAPE;
		if (event.type == SDL_EVENT_QUIT || event.type == SDL_EVENT_WINDOW_CLOSE_REQUESTED || escaped) window_closed();
	}
}

// the value of a system value this library owns into *value; 0 for a name it does not own
int32_t warp_system_value(const char* name, int64_t* value) {
	if (named(name, "window_open")) {
		if (open_now) events_taken();
		*value = open_now;
	} else if (named(name, "key_down")) *value = open_now && held_count > 0;
	else if (named(name, "key code held")) *value = open_now && held_count ? held[held_count - 1] : 0;
	else if (named(name, "mouse_x")) *value = open_now ? (int64_t)mouse_x : 0;
	else if (named(name, "mouse_y")) *value = open_now ? (int64_t)mouse_y : 0;
	else if (named(name, "mouse_down")) *value = open_now && mouse_down;
	else if (named(name, "mouse_wheel")) *value = open_now ? (int64_t)wheel : 0;
	else return 0;
	return 1;
}
// a system value set: window_open opens (1) or closes (0) the window, owned here; the window's size is the driver's
// and only followed here (0, not owned)
int32_t warp_set_system_value(const char* name, int64_t value) {
	if (named(name, "window_open")) {
		if (value) window_opened(); else window_closed();
		return 1;
	}
	if (named(name, "window_width")) width = value, sized_by_program = 1;
	if (named(name, "window_height")) height = value, sized_by_program = 1;
	if (window && sized_by_program && (named(name, "window_width") || named(name, "window_height"))) SDL_SetWindowSize(window, (int)width, (int)height);
	return 0;
}
// the window's Metal layer for a painting of width × height, the window opened by the first, NULL where none opens: a
// window the program gave no size takes its first painting's. Closed by its user, the run ends
void* warp_window_layer(int32_t painting_width, int32_t painting_height) {
	if (closed_by_user) exit(0);
	if (!open_now) {
		if (!sized_by_program) width = painting_width, height = painting_height;
		window_opened();
	}
	if (!open_now || !view) return NULL;
	events_taken();
	if (closed_by_user) exit(0);
	frame_timed();
	if (!sized_by_program && frames == 1 && (painting_width != width || painting_height != height)) {
		width = painting_width;
		height = painting_height;
		SDL_SetWindowSize(window, (int)width, (int)height);
	}
	return SDL_Metal_GetLayer(view);
}

// --- text_coverage(words, size) (label of use draw, card samples-graphics): the words set `size` pixels per em on one
// line in a system sans-serif font through stb_truetype (notes/reuse_graphics.md), as [width, height, then how much
// each pixel is covered from 0 to 255, row by row], its height the font's ascent and descent, as the playground's
// canvas sets them (web/playground/host-media.js). WARP_FONT names another font file
#define STB_TRUETYPE_IMPLEMENTATION
#define STBTT_STATIC
#include "stb_truetype.h"
#include "warp_node.h"

#define FONT_VARIABLE "WARP_FONT"
// the sans-serif fonts looked for, the first found used: what browsers show for sans-serif on each system
static const char* SYSTEM_FONTS[] = {"/System/Library/Fonts/Helvetica.ttc", "/System/Library/Fonts/Supplemental/Arial.ttf",
	"/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/dejavu/DejaVuSans.ttf",
	"/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf", "/usr/share/fonts/liberation-sans/LiberationSans-Regular.ttf",
	"C:\\Windows\\Fonts\\arial.ttf"};
enum { FONT_COUNT = sizeof SYSTEM_FONTS / sizeof *SYSTEM_FONTS };
static char font_failure[1024];

static unsigned char* file_bytes(const char* path) {
	FILE* file = fopen(path, "rb");
	if (!file) return NULL;
	fseek(file, 0, SEEK_END);
	long size = ftell(file);
	fseek(file, 0, SEEK_SET);
	unsigned char* bytes = malloc((size_t)size);
	if (fread(bytes, 1, (size_t)size, file) != (size_t)size) { free(bytes); bytes = NULL; }
	fclose(file);
	return bytes;
}
// the font, read once; NULL and font_failure without one
static stbtt_fontinfo* font(void) {
	static stbtt_fontinfo info;
	static int asked, found;
	if (asked) return found ? &info : NULL;
	asked = 1;
	const char* named = getenv(FONT_VARIABLE);
	const char* path = named && *named ? named : NULL;
	for (int i = 0; !path && i < FONT_COUNT; i++) if (SDL_GetPathInfo(SYSTEM_FONTS[i], NULL)) path = SYSTEM_FONTS[i];
	if (!path) {
		snprintf(font_failure, sizeof font_failure, "no font found: install DejaVu Sans, or name a .ttf in " FONT_VARIABLE);
		return NULL;
	}
	unsigned char* bytes = file_bytes(path);
	if (!bytes || !stbtt_InitFont(&info, bytes, stbtt_GetFontOffsetForIndex(bytes, 0))) {
		snprintf(font_failure, sizeof font_failure, "cannot read the font %s", path);
		return NULL;
	}
	found = 1;
	return &info;
}
// the next code point of UTF-8 text, the text moved past it
static int code_point(const unsigned char** text) {
	const unsigned char* at = *text;
	int length = *at < 0x80 ? 1 : *at < 0xE0 ? 2 : *at < 0xF0 ? 3 : 4;
	int point = length == 1 ? *at : *at & (0x7F >> length);
	for (int i = 1; i < length && at[i]; i++) point = point << 6 | (at[i] & 0x3F);
	for (int i = 0; i < length && **text; i++) (*text)++;
	return point;
}
// the coverage of the words, `count` its numbers (the caller frees it); NULL without a font
static int64_t* coverage(const char* words, float size, size_t* count) {
	stbtt_fontinfo* info = font();
	if (!info) return NULL;
	float scale = stbtt_ScaleForMappingEmToPixels(info, size);
	int ascent, descent, gap;
	stbtt_GetFontVMetrics(info, &ascent, &descent, &gap);
	float caret = 0;
	int previous = 0;
	for (const unsigned char* at = (const unsigned char*)words; *at;) {
		int point = code_point(&at), advance, bearing;
		if (previous) caret += scale * stbtt_GetCodepointKernAdvance(info, previous, point);
		stbtt_GetCodepointHMetrics(info, point, &advance, &bearing);
		caret += scale * advance;
		previous = point;
	}
	int width = caret < 1 ? 1 : (int)ceilf(caret), height = (int)ceilf(scale * (ascent - descent));
	if (height < 1) height = 1;
	int64_t* covered = calloc((size_t)width * height + 2, sizeof *covered);
	covered[0] = width, covered[1] = height;
	float baseline = scale * ascent;
	caret = 0, previous = 0;
	for (const unsigned char* at = (const unsigned char*)words; *at;) {
		int point = code_point(&at), advance, bearing, left, top, right, bottom;
		if (previous) caret += scale * stbtt_GetCodepointKernAdvance(info, previous, point);
		float shift = caret - floorf(caret);
		stbtt_GetCodepointBitmapBoxSubpixel(info, point, scale, scale, shift, 0, &left, &top, &right, &bottom);
		int columns = right - left, rows = bottom - top;
		if (columns > 0 && rows > 0) {
			unsigned char* glyph = malloc((size_t)columns * rows);
			stbtt_MakeCodepointBitmapSubpixel(info, glyph, columns, rows, columns, scale, scale, shift, 0, point);
			for (int y = 0; y < rows; y++) for (int x = 0; x < columns; x++) {
				int column = (int)floorf(caret) + left + x, row = (int)roundf(baseline) + top + y;
				if (column < 0 || column >= width || row < 0 || row >= height) continue;
				int64_t* cell = &covered[2 + (size_t)row * width + column];
				*cell = *cell + glyph[y * columns + x] > 255 ? 255 : *cell + glyph[y * columns + x];
			}
			free(glyph);
		}
		stbtt_GetCodepointHMetrics(info, point, &advance, &bearing);
		caret += scale * advance;
		previous = point;
	}
	*count = (size_t)width * height + 2;
	return covered;
}
// text_coverage(words, size), bound by the driver (lib/native/foreign.c library_word)
wasm_trap_t* warp_word_text_coverage(void* engine, wasmtime_caller_t* caller, const wasmtime_val_t* given, size_t given_count, wasmtime_val_t* results, size_t result_count) {
	(void)given_count, (void)result_count;
	warp_values values = warp_values_of(engine, caller, &given[0].of.anyref);
	wasmtime_anyref_t answer;
	wasmtime_anyref_set_null(&answer);
	char* words = warp_text_bytes(&values, &given[0].of.anyref, NULL);
	size_t count = 0;
	int64_t* covered = values.failure[0] ? NULL : coverage(words, (float)given[1].of.i64, &count);
	if (covered) {
		wasmtime_anyref_t* numbers = malloc(count * sizeof *numbers);
		for (size_t i = 0; i < count; i++) numbers[i] = warp_int(&values, covered[i]);
		answer = warp_list(&values, WARP_MARKED_SQUARELIST, numbers, count);
		free(numbers);
		free(covered);
	} else if (!values.failure[0]) warp_failed(&values, font_failure);
	free(words);
	return warp_answered(&values, answer, results);
}
