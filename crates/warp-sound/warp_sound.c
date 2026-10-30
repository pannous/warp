// libwarp_sound (decision sound-plays): lib/sound.warp's sounds played in the process by miniaudio, one engine for the
// run, each sound mixed over the ones still playing; a run waits at its end till its sounds are heard out. A sound comes
// as its 16-bit samples straight from memory (card sound-direct) or as an audio file
// Built by scripts/wasmtime-c-api.sh beside libwarp_wasmtime, called `from 'warp_sound'`
#define MINIAUDIO_IMPLEMENTATION
#define MA_NO_ENCODING
#define MA_NO_GENERATION
#include "miniaudio.h"
#include <stdlib.h>
enum { MOST_SOUNDS = 256, HEARD_OUT_POLL_MS = 10 };
static const ma_uint32 SOUND_FLAGS = MA_SOUND_FLAG_DECODE | MA_SOUND_FLAG_NO_SPATIALIZATION;

// a sound playing and the samples it plays from memory (none for a file, which miniaudio holds itself)
typedef struct {
	ma_sound sound;
	ma_audio_buffer* samples;
} voice;

static ma_engine engine;
static int engine_started;
static voice* voices[MOST_SOUNDS];
static int voice_count;

static void voice_freed(voice* v) {
	ma_sound_uninit(&v->sound);
	if (v->samples) ma_audio_buffer_uninit_and_free(v->samples);
	free(v);
}
static int playing(void) {
	for (int i = 0; i < voice_count; i++)
		if (ma_sound_is_playing(&voices[i]->sound)) return 1;
	return 0;
}
// at the run's end: its sounds heard out, then the engine closed
static void heard_out(void) {
	while (playing()) ma_sleep(HEARD_OUT_POLL_MS);
	for (int i = 0; i < voice_count; i++) voice_freed(voices[i]);
	ma_engine_uninit(&engine);
}
// the finished sounds dropped, so a long run keeps room for new ones
static void finished_dropped(void) {
	int kept = 0;
	for (int i = 0; i < voice_count; i++) {
		if (ma_sound_is_playing(&voices[i]->sound)) voices[kept++] = voices[i];
		else voice_freed(voices[i]);
	}
	voice_count = kept;
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
// a new voice once the engine runs and has room for it, else NULL with miniaudio's error in *failed
static voice* voice_made(ma_result* failed) {
	if (!engine_started) {
		*failed = engine_init();
		if (*failed != MA_SUCCESS) return NULL;
		engine_started = 1;
		if (!silent()) atexit(heard_out);
	}
	finished_dropped();
	*failed = MA_OUT_OF_MEMORY;
	if (voice_count == MOST_SOUNDS) return NULL;
	return calloc(1, sizeof(voice));
}
// the voice playing from now on: 0, else miniaudio's error with the voice freed
static int voice_started(voice* v, ma_result loaded) {
	if (loaded != MA_SUCCESS) {
		if (v->samples) ma_audio_buffer_uninit_and_free(v->samples);
		free(v);
		return loaded;
	}
	voices[voice_count++] = v;
	return ma_sound_start(&v->sound);
}
// the audio file played from now on: 0 once it plays, else miniaudio's error (no audio device, an unreadable file)
int warp_sound_play(const char* path) {
	ma_result failed;
	voice* v = voice_made(&failed);
	if (!v) return failed;
	return voice_started(v, ma_sound_init_from_file(&engine, path, SOUND_FLAGS, NULL, NULL, &v->sound));
}
// little-endian 16-bit samples, the channels interleaved, played from now on: copied, so the caller's bytes may go
int warp_sound_play_samples(const void* bytes, size_t size, int rate, int channels) {
	ma_result failed;
	voice* v = voice_made(&failed);
	if (!v) return failed;
	ma_audio_buffer_config config = ma_audio_buffer_config_init(ma_format_s16, channels, size / (2 * channels), bytes, NULL);
	config.sampleRate = rate;
	ma_result copied = ma_audio_buffer_alloc_and_init(&config, &v->samples);
	if (copied != MA_SUCCESS) return voice_started(v, copied);
	return voice_started(v, ma_sound_init_from_data_source(&engine, v->samples, SOUND_FLAGS, NULL, &v->sound));
}
