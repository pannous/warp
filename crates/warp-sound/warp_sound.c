// libwarp_sound (decision sound-plays): lib/sound.warp's sounds played in the process by miniaudio, one engine for the
// run, each sound mixed over the ones still playing; a run waits at its end till its sounds are heard out. A sound comes
// as its 16-bit samples straight from memory (card sound-direct) or as an audio file
// Built by scripts/wasmtime-c-api.sh beside libwarp_wasmtime, called `from 'warp_sound'`
#define MINIAUDIO_IMPLEMENTATION
#define MA_NO_ENCODING
#define MA_NO_GENERATION
#include "miniaudio.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
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

// Audio files decoded (wav, mp3, flac built into miniaudio): a file's 16-bit samples at a rate and channel count (0 for
// the file's own), kept for the run, each file once, so lib/sound.warp reads them value by value as the run needs them
enum { MOST_DECODED = 64, DECODE_CHUNK_FRAMES = 4096 };
typedef struct {
	char* path;
	int asked_rate, asked_channels;
	int rate, channels;
	ma_int16* values;
	ma_uint64 frames;
} decoded_file;
static decoded_file decoded[MOST_DECODED];
static int decoded_count;

// An MP3's true length (decision mp3-true-length), as the page's decodeAudioData gives it: the Xing/Info frame's LAME
// tag counts the frames and names the encoder's delay and padding, silence the decoder hands out around the music;
// miniaudio decodes the Info frame and that silence too
enum { MP3_HEADER_BYTES = 1 << 16, XING_FRAMES = 1, XING_BYTES = 2, XING_TOC = 4, XING_QUALITY = 8, LAME_GAPS_AT = 21 };
typedef struct {
	ma_uint64 samples; // per channel, at the file's own rate
	ma_uint32 padding, rate;
} mp3_length;

static ma_uint32 big_endian(const unsigned char* at) { return (ma_uint32)at[0] << 24 | at[1] << 16 | at[2] << 8 | at[3]; }
static ma_uint32 id3_size(const unsigned char* at, size_t size) {
	if (size < 10 || memcmp(at, "ID3", 3) != 0) return 0;
	ma_uint32 tag = (ma_uint32)(at[6] & 0x7F) << 21 | (at[7] & 0x7F) << 14 | (at[8] & 0x7F) << 7 | (at[9] & 0x7F);
	return 10 + tag + (at[5] & 0x10 ? 10 : 0);
}
// the length the tag names; 0 samples for a file without one (a WAV, an MP3 of no LAME tag)
static mp3_length mp3_tagged_length(const char* path) {
	static const ma_uint32 RATES[4][3] = {{11025, 12000, 8000}, {0}, {22050, 24000, 16000}, {44100, 48000, 32000}};
	mp3_length length = {0};
	unsigned char* bytes = malloc(MP3_HEADER_BYTES);
	FILE* file = bytes ? fopen(path, "rb") : NULL;
	size_t size = file ? fread(bytes, 1, MP3_HEADER_BYTES, file) : 0;
	if (file) fclose(file);
	size_t frame = id3_size(bytes, size);
	if (frame + 4 <= size && bytes[frame] == 0xFF && (bytes[frame + 1] & 0xE6) == 0xE2) {
		int version = bytes[frame + 1] >> 3 & 3, rate_index = bytes[frame + 2] >> 2 & 3, mono = bytes[frame + 3] >> 6 == 3;
		int mpeg1 = version == 3;
		size_t xing = frame + 4 + (mpeg1 ? (mono ? 17 : 32) : (mono ? 9 : 17));
		if (rate_index < 3 && xing + 12 <= size && (memcmp(bytes + xing, "Xing", 4) == 0 || memcmp(bytes + xing, "Info", 4) == 0)) {
			ma_uint32 flags = big_endian(bytes + xing + 4);
			size_t lame = xing + 8 + (flags & XING_FRAMES ? 4 : 0) + (flags & XING_BYTES ? 4 : 0) + (flags & XING_TOC ? 100 : 0) + (flags & XING_QUALITY ? 4 : 0);
			if (flags & XING_FRAMES && lame + LAME_GAPS_AT + 3 <= size) {
				const unsigned char* gaps = bytes + lame + LAME_GAPS_AT;
				ma_uint32 delay = gaps[0] << 4 | gaps[1] >> 4, padding = (gaps[1] & 0x0F) << 8 | gaps[2];
				ma_uint64 samples = (ma_uint64)big_endian(bytes + xing + 8) * (mpeg1 ? 1152 : 576);
				if (samples > delay + padding) length = (mp3_length){samples - delay - padding, padding, RATES[version][rate_index]};
			}
		}
	}
	free(bytes);
	return length;
}
// the decoded frames cut to the tagged length: the Info frame and the delay off the front, the padding off the end
static ma_uint64 trimmed_to_length(const char* path, ma_int16* values, ma_uint64 frames, int frame_size, ma_uint32 rate) {
	mp3_length length = mp3_tagged_length(path);
	if (!length.samples || !length.rate) return frames;
	double scale = (double)rate / length.rate;
	ma_uint64 kept = (ma_uint64)(length.samples * scale + 0.5), tail = (ma_uint64)(length.padding * scale + 0.5);
	if (kept + tail > frames) return frames;
	ma_uint64 leading = frames - kept - tail;
	memmove(values, values + leading * frame_size, kept * frame_size * sizeof *values);
	return kept;
}

static int decoded_found(const char* path, int rate, int channels) {
	for (int i = 0; i < decoded_count; i++)
		if (decoded[i].asked_rate == rate && decoded[i].asked_channels == channels && strcmp(decoded[i].path, path) == 0) return i;
	return -1;
}
// the file's number, else minus miniaudio's error (MA_DOES_NOT_EXIST, MA_INVALID_FILE for a format it can't read)
int warp_sound_decode(const char* path, int rate, int channels) {
	int found = decoded_found(path, rate, channels);
	if (found >= 0) return found;
	if (decoded_count == MOST_DECODED) return MA_OUT_OF_MEMORY;
	ma_decoder_config config = ma_decoder_config_init(ma_format_s16, channels, rate);
	ma_decoder decoder;
	ma_result opened = ma_decoder_init_file(path, &config, &decoder);
	if (opened != MA_SUCCESS) return opened;
	int frame_size = decoder.outputChannels;
	ma_int16* values = NULL;
	ma_uint64 frames = 0, read = 0;
	do {
		ma_int16* grown = realloc(values, (frames + DECODE_CHUNK_FRAMES) * frame_size * sizeof *values);
		if (!grown) {
			free(values);
			ma_decoder_uninit(&decoder);
			return MA_OUT_OF_MEMORY;
		}
		values = grown;
		ma_decoder_read_pcm_frames(&decoder, values + frames * frame_size, DECODE_CHUNK_FRAMES, &read);
		frames += read;
	} while (read == DECODE_CHUNK_FRAMES);
	frames = trimmed_to_length(path, values, frames, frame_size, decoder.outputSampleRate);
	decoded[decoded_count] = (decoded_file){strdup(path), rate, channels, (int)decoder.outputSampleRate, frame_size, values, frames};
	decoded_count++;
	ma_decoder_uninit(&decoder);
	return decoded_count - 1;
}
long warp_sound_decoded_frames(int file) { return file >= 0 && file < decoded_count ? (long)decoded[file].frames : 0; }
int warp_sound_decoded_rate(int file) { return file >= 0 && file < decoded_count ? decoded[file].rate : 0; }
int warp_sound_decoded_channels(int file) { return file >= 0 && file < decoded_count ? decoded[file].channels : 0; }
// the value at that place among the interleaved channels, 0 past the end
int warp_sound_decoded_value(int file, long at) {
	if (file < 0 || file >= decoded_count || at < 0) return 0;
	decoded_file* f = &decoded[file];
	return (ma_uint64)at < f->frames * (ma_uint64)f->channels ? f->values[at] : 0;
}

// MIDI out (card midi-output-native, decision midi-native): lib/sound.warp's midi_send, the bytes of one message to the
// output named (the first for ""), else to warp's own virtual source "warp" that a synth app listens to; 0 sent,
// MIDI_NO_OUTPUT for no output of that name, MIDI_FAILED, MIDI_UNSUPPORTED where no MIDI is built in yet (Linux: ALSA
// to come). A quiet run (WARP_NO_WINDOW, CI) sends nothing, as it hears nothing. warp_midi_outputs and
// warp_midi_inputs: the devices' names, one a line
// (NULL where no MIDI is built in)
enum { MIDI_FAILED = -1, MIDI_NO_OUTPUT = -2, MIDI_UNSUPPORTED = -3, MIDI_NAMES_BYTES = 8192, MIDI_PACKET_BYTES = 1024 };
#ifdef __APPLE__
static char midi_names[MIDI_NAMES_BYTES];
#include <CoreMIDI/CoreMIDI.h>
static MIDIClientRef midi_client;
static MIDIPortRef midi_output_port;
static MIDIEndpointRef midi_virtual_source;

static int midi_started(void) {
	if (midi_client) return 1;
	if (MIDIClientCreate(CFSTR("warp"), NULL, NULL, &midi_client) != noErr) return 0;
	return MIDIOutputPortCreate(midi_client, CFSTR("warp out"), &midi_output_port) == noErr;
}
static void endpoint_name(MIDIEndpointRef endpoint, char* into, size_t size) {
	CFStringRef name = NULL;
	into[0] = 0;
	if (MIDIObjectGetStringProperty(endpoint, kMIDIPropertyDisplayName, &name) != noErr || !name) return;
	CFStringGetCString(name, into, (CFIndex)size, kCFStringEncodingUTF8);
	CFRelease(name);
}
static MIDIEndpointRef output_named(const char* port) {
	char name[256];
	for (ItemCount i = 0; i < MIDIGetNumberOfDestinations(); i++) {
		MIDIEndpointRef output = MIDIGetDestination(i);
		if (!*port) return output;
		endpoint_name(output, name, sizeof name);
		if (strcmp(name, port) == 0) return output;
	}
	return 0;
}
int warp_midi_send(const char* port, const unsigned char* bytes, size_t size) {
	if (silent()) return 0;
	if (!midi_started()) return MIDI_FAILED;
	Byte buffer[MIDI_PACKET_BYTES];
	MIDIPacketList* packets = (MIDIPacketList*)buffer;
	MIDIPacketListAdd(packets, sizeof buffer, MIDIPacketListInit(packets), 0, size, bytes);
	MIDIEndpointRef output = output_named(port);
	if (output) return MIDISend(midi_output_port, output, packets) == noErr ? 0 : MIDI_FAILED;
	if (*port) return MIDI_NO_OUTPUT;
	if (!midi_virtual_source && MIDISourceCreate(midi_client, CFSTR("warp"), &midi_virtual_source) != noErr) return MIDI_FAILED;
	return MIDIReceived(midi_virtual_source, packets) == noErr ? 0 : MIDI_FAILED;
}
static const char* endpoint_names(ItemCount count, MIDIEndpointRef (*endpoint_at)(ItemCount)) {
	size_t at = 0;
	midi_names[0] = 0;
	for (ItemCount i = 0; i < count && at < sizeof midi_names - 1; i++) {
		endpoint_name(endpoint_at(i), midi_names + at, sizeof midi_names - at - 1);
		at += strlen(midi_names + at);
		midi_names[at++] = '\n';
		midi_names[at] = 0;
	}
	return midi_names;
}
const char* warp_midi_outputs(void) { return endpoint_names(MIDIGetNumberOfDestinations(), MIDIGetDestination); }
const char* warp_midi_inputs(void) { return endpoint_names(MIDIGetNumberOfSources(), MIDIGetSource); }
#else
int warp_midi_send(const char* port, const unsigned char* bytes, size_t size) { (void)port; (void)bytes; (void)size; return MIDI_UNSUPPORTED; }
const char* warp_midi_outputs(void) { return NULL; }
const char* warp_midi_inputs(void) { return NULL; }
#endif
