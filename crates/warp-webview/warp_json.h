// warp values as JSON and back, for the js runtime in libwarp_webview (warp_webview.m): the arguments of a `use js`
// call written as web/playground/values.js guestPlain reads them (a text a string, a list an array, a map an object,
// yes/no booleans, ø null), its answer read as newGuestValue builds one (a whole number an Int, an object a map in its
// order). Plain C over lib/native/warp_node.h, so a web view of another system can share it
#pragma once
#include <math.h>
#include "warp_node.h"

enum { JSON_FIRST_ROOM = 256, CODEPOINT_BYTES = 4, HEX_DIGITS = 4 };

typedef struct { char* bytes; size_t length, room; } json_text;

static void json_added(json_text* json, const char* bytes, size_t length) {
	if (json->length + length + 1 > json->room) {
		json->room = (json->length + length + 1) * 2;
		json->bytes = realloc(json->bytes, json->room);
	}
	memcpy(json->bytes + json->length, bytes, length);
	json->length += length;
	json->bytes[json->length] = 0;
}
static void json_said(json_text* json, const char* bytes) { json_added(json, bytes, strlen(bytes)); }

static size_t utf8_written(uint32_t codepoint, char* bytes) {
	if (codepoint < 0x80) { bytes[0] = (char)codepoint; return 1; }
	if (codepoint < 0x800) { bytes[0] = (char)(0xC0 | codepoint >> 6); bytes[1] = (char)(0x80 | (codepoint & 0x3F)); return 2; }
	if (codepoint < 0x10000) {
		bytes[0] = (char)(0xE0 | codepoint >> 12); bytes[1] = (char)(0x80 | (codepoint >> 6 & 0x3F)); bytes[2] = (char)(0x80 | (codepoint & 0x3F));
		return 3;
	}
	bytes[0] = (char)(0xF0 | codepoint >> 18); bytes[1] = (char)(0x80 | (codepoint >> 12 & 0x3F));
	bytes[2] = (char)(0x80 | (codepoint >> 6 & 0x3F)); bytes[3] = (char)(0x80 | (codepoint & 0x3F));
	return 4;
}
static void json_string(json_text* json, const char* bytes, size_t length) {
	json_said(json, "\"");
	for (size_t i = 0; i < length; i++) {
		unsigned char byte = (unsigned char)bytes[i];
		char escaped[8];
		if (byte == '"' || byte == '\\') { snprintf(escaped, sizeof escaped, "\\%c", byte); json_said(json, escaped); }
		else if (byte < 0x20) { snprintf(escaped, sizeof escaped, "\\u%04x", byte); json_said(json, escaped); }
		else json_added(json, (const char*)&byte, 1);
	}
	json_said(json, "\"");
}
static void json_number(json_text* json, wasmtime_val_t number) {
	char shown[32];
	if (number.kind == WASMTIME_I64) snprintf(shown, sizeof shown, "%lld", (long long)number.of.i64);
	else if (isfinite(number.of.f64)) snprintf(shown, sizeof shown, "%.17g", number.of.f64);
	else snprintf(shown, sizeof shown, "null");
	json_said(json, shown);
}
static void json_text_node(warp_values* values, json_text* json, const wasmtime_anyref_t* node) {
	size_t length = 0;
	char* text = warp_text_bytes(values, node, &length);
	json_string(json, text, length);
	free(text);
}
static void json_of_node(warp_values* values, json_text* json, const wasmtime_anyref_t* node);
// a key `name: value` as the member of an object
static void json_member(warp_values* values, json_text* json, const wasmtime_anyref_t* key) {
	wasmtime_anyref_t left = warp_key_left(values, key), right = warp_key_right(values, key);
	json_text_node(values, json, &left);
	json_said(json, ":");
	json_of_node(values, json, &right);
	wasmtime_anyref_unroot(&left);
	if (!wasmtime_anyref_is_null(&right)) wasmtime_anyref_unroot(&right);
}
static bool is_keyed(warp_values* values, const wasmtime_anyref_t* list, size_t count) {
	if (count == 0) return false;
	for (size_t i = 0; i < count; i++) {
		wasmtime_anyref_t item = warp_list_item(values, list, i);
		int kind = warp_node_kind(values, &item);
		if (!wasmtime_anyref_is_null(&item)) wasmtime_anyref_unroot(&item);
		if (kind != WARP_KIND_KEY) return false;
	}
	return true;
}
static void json_of_list(warp_values* values, json_text* json, const wasmtime_anyref_t* list) {
	size_t count = warp_list_length(values, list);
	bool curly = warp_kind_field(values, list) / WARP_INFO_UNIT % 8 == WARP_BRACKET_CURLY;
	bool keyed = curly && is_keyed(values, list, count);
	json_said(json, keyed ? "{" : "[");
	for (size_t i = 0; i < count && !values->failure[0]; i++) {
		if (i) json_said(json, ",");
		wasmtime_anyref_t item = warp_list_item(values, list, i);
		if (keyed) json_member(values, json, &item); else json_of_node(values, json, &item);
		if (!wasmtime_anyref_is_null(&item)) wasmtime_anyref_unroot(&item);
	}
	json_said(json, keyed ? "}" : "]");
}
static void json_of_codepoint(warp_values* values, json_text* json, const wasmtime_anyref_t* node) {
	wasmtime_anyref_t data = warp_field_ref(values, node, WARP_NODE_DATA);
	uint32_t codepoint = 0;
	wasmtime_anyref_i31_get_u(values->context, &data, &codepoint);
	wasmtime_anyref_unroot(&data);
	char bytes[CODEPOINT_BYTES];
	json_string(json, bytes, utf8_written(codepoint, bytes));
}
static void json_of_node(warp_values* values, json_text* json, const wasmtime_anyref_t* node) {
	int64_t whole_kind = wasmtime_anyref_is_null(node) ? WARP_KIND_EMPTY : warp_kind_field(values, node);
	switch (whole_kind == WARP_MARKED_INSTANCE ? -1 : (int)(whole_kind & WARP_KIND_MASK)) {
	case WARP_KIND_EMPTY: json_said(json, "null"); return;
	case WARP_KIND_INT:
		if (whole_kind == WARP_MARKED_BOOL) { json_said(json, warp_node_int(values, node) ? "true" : "false"); return; }
		json_number(json, warp_number(values, node));
		return;
	case WARP_KIND_FLOAT: json_number(json, warp_number(values, node)); return;
	case WARP_KIND_TEXT: case WARP_KIND_SYMBOL: json_text_node(values, json, node); return;
	case WARP_KIND_CODEPOINT: json_of_codepoint(values, json, node); return;
	case WARP_KIND_LIST: case WARP_KIND_BLOCK: json_of_list(values, json, node); return;
	case WARP_KIND_KEY: json_said(json, "{"); json_member(values, json, node); json_said(json, "}"); return;
	case WARP_KIND_FUNCTION: warp_failed(values, "a function cannot be given to js natively yet"); return;
	default: warp_failed(values, "a value without a plain form (an instance, a type) cannot be given to js"); return;
	}
}

// reading the answer: JSON, each value a new warp value
typedef struct { const char* at; warp_values* values; } json_reader;

static void json_skipped(json_reader* reader) { while (*reader->at && strchr(" \t\r\n", *reader->at)) reader->at++; }
static bool json_took(json_reader* reader, const char* word) {
	size_t length = strlen(word);
	if (strncmp(reader->at, word, length)) return false;
	reader->at += length;
	return true;
}
static wasmtime_anyref_t json_failed(json_reader* reader) {
	warp_failed(reader->values, "js answered no JSON warp reads");
	wasmtime_anyref_t none;
	wasmtime_anyref_set_null(&none);
	return none;
}
static uint32_t json_hex(json_reader* reader) {
	char digits[HEX_DIGITS + 1] = {0};
	for (int i = 0; i < HEX_DIGITS && reader->at[1]; i++) digits[i] = *++reader->at;
	return (uint32_t)strtoul(digits, NULL, 16);
}
// a string's bytes (the caller frees them) after its opening quote, up to past its closing one
static char* json_string_read(json_reader* reader, size_t* length) {
	json_text text = {0};
	json_said(&text, "");
	for (reader->at++; *reader->at && *reader->at != '"'; reader->at++) {
		if (*reader->at != '\\') { json_added(&text, reader->at, 1); continue; }
		char escape = *++reader->at;
		const char* plain = escape == 'n' ? "\n" : escape == 't' ? "\t" : escape == 'r' ? "\r" : escape == 'b' ? "\b" : escape == 'f' ? "\f" : NULL;
		if (plain) { json_said(&text, plain); continue; }
		if (escape != 'u') { json_added(&text, &escape, 1); continue; }
		uint32_t codepoint = json_hex(reader);
		if (codepoint >= 0xD800 && codepoint < 0xDC00 && reader->at[1] == '\\' && reader->at[2] == 'u') {
			reader->at += 2;
			codepoint = 0x10000 + ((codepoint - 0xD800) << 10) + (json_hex(reader) - 0xDC00);
		}
		char bytes[CODEPOINT_BYTES];
		json_added(&text, bytes, utf8_written(codepoint, bytes));
	}
	if (*reader->at) reader->at++;
	*length = text.length;
	return text.bytes;
}
static wasmtime_anyref_t json_value(json_reader* reader);
static wasmtime_anyref_t json_items(json_reader* reader, char close, bool keyed) {
	size_t count = 0, room = 8;
	wasmtime_anyref_t* items = malloc(room * sizeof *items);
	for (reader->at++, json_skipped(reader); *reader->at && *reader->at != close; json_skipped(reader)) {
		if (count == room) items = realloc(items, (room *= 2) * sizeof *items);
		if (keyed) {
			size_t length = 0;
			char* name = json_string_read(reader, &length);
			json_skipped(reader);
			if (*reader->at == ':') reader->at++;
			items[count++] = warp_entry(reader->values, name, json_value(reader));
			free(name);
		} else items[count++] = json_value(reader);
		json_skipped(reader);
		if (*reader->at == ',') reader->at++;
	}
	if (*reader->at) reader->at++;
	wasmtime_anyref_t list = warp_list(reader->values, keyed ? WARP_MARKED_MAP : WARP_MARKED_SQUARELIST, items, count);
	free(items);
	return list;
}
static wasmtime_anyref_t json_bool(warp_values* values, bool yes) {
	wasmtime_val_t field = {.kind = WASMTIME_I64, .of.i64 = yes};
	return warp_node_new(values, WARP_MARKED_BOOL, warp_struct_new(values, values->int_box, &field, 1), NULL);
}
static wasmtime_anyref_t json_number_read(json_reader* reader) {
	char* end;
	const char* start = reader->at;
	double number = strtod(start, &end);
	if (end == start) return json_failed(reader);
	reader->at = end;
	bool whole = !memchr(start, '.', (size_t)(end - start)) && !memchr(start, 'e', (size_t)(end - start)) && !memchr(start, 'E', (size_t)(end - start));
	return whole ? warp_int(reader->values, strtoll(start, NULL, 10)) : warp_float(reader->values, number);
}
static wasmtime_anyref_t json_value(json_reader* reader) {
	json_skipped(reader);
	char next = *reader->at;
	if (next == '{') return json_items(reader, '}', true);
	if (next == '[') return json_items(reader, ']', false);
	if (next == '"') {
		size_t length = 0;
		char* text = json_string_read(reader, &length);
		wasmtime_anyref_t node = warp_text(reader->values, text, length);
		free(text);
		return node;
	}
	if (json_took(reader, "true")) return json_bool(reader->values, true);
	if (json_took(reader, "false")) return json_bool(reader->values, false);
	if (json_took(reader, "null")) { wasmtime_anyref_t none; wasmtime_anyref_set_null(&none); return warp_node_new(reader->values, WARP_KIND_EMPTY, none, NULL); }
	return json_number_read(reader);
}
static wasmtime_anyref_t node_of_json(warp_values* values, const char* json) {
	json_reader reader = {.at = json, .values = values};
	return json_value(&reader);
}
