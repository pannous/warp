// libwarp_window (card native-window): the native warp's one window, over SDL3. It answers the system values it owns
// (warp_system_value) and takes the ones a program sets (warp_set_system_value); lib/native/foreign.c forwards those
// the driver does not own here, and asks for the window's Metal layer, which libwarp_gpu shows paintings in.
// `window_open = 1` opens it, never under WARP_NO_WINDOW or CI; each reading of window_open is a frame: the window's
// events taken in, its title telling the frame times. Built by scripts/wasmtime-c-api.sh where SDL3 is installed
#include <SDL3/SDL.h>
#include <SDL3/SDL_metal.h>
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
static int open_now, sized_by_program;
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

static void window_opened(void) {
	if (window || windows_refused()) return;
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
		case SDL_EVENT_QUIT: case SDL_EVENT_WINDOW_CLOSE_REQUESTED: window_closed(); return;
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
	frame_timed();
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
// the open window's Metal layer for a painting of width × height, NULL without a window: a window the program gave no
// size takes its first painting's
void* warp_window_layer(int32_t painting_width, int32_t painting_height) {
	if (!open_now || !view) return NULL;
	if (!sized_by_program && frames <= 1 && (painting_width != width || painting_height != height)) {
		width = painting_width;
		height = painting_height;
		SDL_SetWindowSize(window, (int)width, (int)height);
	}
	return SDL_Metal_GetLayer(view);
}
