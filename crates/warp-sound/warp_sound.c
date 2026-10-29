// libwarp_sound (decision sound-plays): lib/sound.warp's WAV files played in the process by miniaudio, one engine for
// the run, each sound mixed over the ones still playing; a run waits at its end till its sounds are heard out.
// Built by scripts/wasmtime-c-api.sh beside libwarp_wasmtime, called `from 'warp_sound'`
#define MINIAUDIO_IMPLEMENTATION
#define MA_NO_ENCODING
#define MA_NO_GENERATION
#include "miniaudio.h"
#include <stdlib.h>

enum { MOST_SOUNDS = 256, HEARD_OUT_POLL_MS = 10 };

static ma_engine engine;
static int engine_started;
static ma_sound* sounds[MOST_SOUNDS];
static int sound_count;

static int playing(void) {
	for (int i = 0; i < sound_count; i++)
		if (ma_sound_is_playing(sounds[i])) return 1;
	return 0;
}
// at the run's end: its sounds heard out, then the engine closed
static void heard_out(void) {
	while (playing()) ma_sleep(HEARD_OUT_POLL_MS);
	for (int i = 0; i < sound_count; i++) {
		ma_sound_uninit(sounds[i]);
		free(sounds[i]);
	}
	ma_engine_uninit(&engine);
}
// the finished sounds dropped, so a long run keeps room for new ones
static void finished_dropped(void) {
	int kept = 0;
	for (int i = 0; i < sound_count; i++) {
		if (ma_sound_is_playing(sounds[i])) {
			sounds[kept++] = sounds[i];
		} else {
			ma_sound_uninit(sounds[i]);
			free(sounds[i]);
		}
	}
	sound_count = kept;
}
// under WARP_NO_WINDOW or CI (a test, an agent's run) the sounds go to miniaudio's silent device, unwaited for
static int silent(void) { return getenv("WARP_NO_WINDOW") || getenv("CI"); }
static ma_result engine_init(void) {
	if (!silent()) return ma_engine_init(NULL, &engine);
	static ma_context context;
	ma_backend null_backend = ma_backend_null;
	ma_result made = ma_context_init(&null_backend, 1, NULL, &context);
	if (made != MA_SUCCESS) return made;
	ma_engine_config config = ma_engine_config_init();
	config.pContext = &context;
	return ma_engine_init(&config, &engine);
}
// the WAV file played from now on: 0 once it plays, else miniaudio's error (no audio device, an unreadable file)
int warp_sound_play(const char* path) {
	if (!engine_started) {
		ma_result started = engine_init();
		if (started != MA_SUCCESS) return started;
		engine_started = 1;
		if (!silent()) atexit(heard_out);
	}
	finished_dropped();
	if (sound_count == MOST_SOUNDS) return MA_OUT_OF_MEMORY;
	ma_sound* sound = malloc(sizeof *sound);
	ma_result loaded = ma_sound_init_from_file(&engine, path, MA_SOUND_FLAG_DECODE | MA_SOUND_FLAG_NO_SPATIALIZATION, NULL, NULL, sound);
	if (loaded != MA_SUCCESS) {
		free(sound);
		return loaded;
	}
	sounds[sound_count++] = sound;
	return ma_sound_start(sound);
}
