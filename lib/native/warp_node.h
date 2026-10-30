// warp values read and built in place, for a library's host words (decision library-direct-refs): the driver binds a
// host import no driver word answers to the library's `warp_word_<name>` (core/native/driver.warp library_bound), a
// wasmtime_func_callback_t given the module's own references. The layout comes from warp_layout.h, generated from
// the compiler's (scripts/node_layout.warp); notes/self_hosting.md "Value layout" explains it.
// A word starts with warp_values_of(env, caller, a node it was given), reads and builds through it, and ends with
// warp_answered, which gives the module an Error of the first failure instead of the value.
#pragma once
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wasmtime.h>
#include "warp_layout.h"

#define WARP_OP_COLON 1
#define WARP_FAILURE_ROOM 200

typedef struct {
	wasm_engine_t* engine;
	wasmtime_caller_t* caller;
	wasmtime_context_t* context;
	// the module's types: a $Node, a $list and its $items (recursive or holding anyrefs, so taken from given values),
	// the boxes and $text (made: a type of its own is the module's by structure)
	wasmtime_struct_ref_pre_t *node, *list, *int_box, *float_box;
	wasmtime_array_ref_pre_t *items, *text;
	char failure[WARP_FAILURE_ROOM];
} warp_values;

static inline void warp_failed(warp_values* values, const char* message) {
	if (!values->failure[0]) snprintf(values->failure, WARP_FAILURE_ROOM, "%s", message);
}
static inline void warp_unroot(wasmtime_val_t* value) { if (value->kind == WASMTIME_ANYREF) wasmtime_anyref_unroot(&value->of.anyref); }
static inline wasmtime_val_t warp_anyref(wasmtime_anyref_t ref) { wasmtime_val_t value = {.kind = WASMTIME_ANYREF}; value.of.anyref = ref; return value; }

// a struct's field, null when the reference is no struct
static inline wasmtime_val_t warp_field(warp_values* values, const wasmtime_anyref_t* ref, size_t index) {
	wasmtime_val_t field = {.kind = WASMTIME_ANYREF};
	wasmtime_anyref_set_null(&field.of.anyref);
	wasmtime_structref_t structure;
	if (!wasmtime_anyref_as_struct(values->context, ref, &structure)) return field;
	wasmtime_error_t* error = wasmtime_structref_field(values->context, &structure, index, &field);
	if (error) { wasmtime_error_delete(error); warp_failed(values, "a warp value of another layout"); }
	wasmtime_structref_unroot(&structure);
	return field;
}
static inline wasmtime_anyref_t warp_field_ref(warp_values* values, const wasmtime_anyref_t* ref, size_t index) {
	return warp_field(values, ref, index).of.anyref;
}

// reading: a node's whole kind field and its Kind
static inline int64_t warp_kind_field(warp_values* values, const wasmtime_anyref_t* node) {
	return warp_field(values, node, WARP_NODE_KIND).of.i64;
}
static inline int warp_node_kind(warp_values* values, const wasmtime_anyref_t* node) {
	return wasmtime_anyref_is_null(node) ? WARP_KIND_EMPTY : (int)(warp_kind_field(values, node) & WARP_KIND_MASK);
}
// a number's box: an $i64box or $f64box's value, an exact ratio's $Ratio of two $i64box as a float, a $BigInt a failure
static inline wasmtime_val_t warp_boxed_number(warp_values* values, const wasmtime_anyref_t* box) {
	wasmtime_val_t first = warp_field(values, box, WARP_I64BOX_VALUE);
	if (first.kind == WASMTIME_I64 || first.kind == WASMTIME_F64) return first;
	wasmtime_val_t number = {.kind = WASMTIME_F64, .of.f64 = 0};
	if (first.kind != WASMTIME_ANYREF) { warp_failed(values, "an integer beyond 64 bits"); return number; }
	wasmtime_anyref_t denominator = warp_field_ref(values, box, WARP_RATIO_DENOMINATOR);
	number.of.f64 = (double)warp_field(values, &first.of.anyref, WARP_I64BOX_VALUE).of.i64 / (double)warp_field(values, &denominator, WARP_I64BOX_VALUE).of.i64;
	wasmtime_anyref_unroot(&denominator);
	warp_unroot(&first);
	return number;
}
// an Int's (a yes/no's, a ratio's) or Float's number, as an int or a float
static inline wasmtime_val_t warp_number(warp_values* values, const wasmtime_anyref_t* node) {
	int kind = warp_node_kind(values, node);
	wasmtime_val_t number = {.kind = WASMTIME_I64, .of.i64 = 0};
	if (kind != WARP_KIND_INT && kind != WARP_KIND_FLOAT) { warp_failed(values, "a number expected"); return number; }
	wasmtime_anyref_t box = warp_field_ref(values, node, WARP_NODE_DATA);
	number = warp_boxed_number(values, &box);
	wasmtime_anyref_unroot(&box);
	return number;
}
static inline int64_t warp_node_int(warp_values* values, const wasmtime_anyref_t* node) {
	wasmtime_val_t number = warp_number(values, node);
	return number.kind == WASMTIME_F64 ? (int64_t)number.of.f64 : number.of.i64;
}
static inline double warp_node_float(warp_values* values, const wasmtime_anyref_t* node) {
	wasmtime_val_t number = warp_number(values, node);
	return number.kind == WASMTIME_F64 ? number.of.f64 : (double)number.of.i64;
}
static inline uint32_t warp_array_length(warp_values* values, const wasmtime_anyref_t* ref) {
	uint32_t length = 0;
	wasmtime_arrayref_t array;
	if (!wasmtime_anyref_as_array(values->context, ref, &array)) return 0;
	wasmtime_error_t* error = wasmtime_arrayref_len(values->context, &array, &length);
	if (error) wasmtime_error_delete(error);
	wasmtime_arrayref_unroot(&array);
	return length;
}
static inline wasmtime_val_t warp_array_item(warp_values* values, const wasmtime_anyref_t* ref, uint32_t index) {
	wasmtime_val_t item = {.kind = WASMTIME_ANYREF};
	wasmtime_anyref_set_null(&item.of.anyref);
	wasmtime_arrayref_t array;
	if (!wasmtime_anyref_as_array(values->context, ref, &array)) return item;
	wasmtime_error_t* error = wasmtime_arrayref_get(values->context, &array, index, &item);
	if (error) { wasmtime_error_delete(error); warp_failed(values, "an item beyond a warp list"); }
	wasmtime_arrayref_unroot(&array);
	return item;
}
// a Text's or Symbol's UTF-8 bytes, zero-ended, the caller's to free
static inline char* warp_text_bytes(warp_values* values, const wasmtime_anyref_t* node, size_t* length) {
	int kind = warp_node_kind(values, node);
	if (kind != WARP_KIND_TEXT && kind != WARP_KIND_SYMBOL) warp_failed(values, "a text expected");
	wasmtime_anyref_t bytes = warp_field_ref(values, node, WARP_NODE_DATA);
	uint32_t count = warp_array_length(values, &bytes);
	char* text = malloc(count + 1);
	for (uint32_t i = 0; i < count; i++) text[i] = (char)warp_array_item(values, &bytes, i).of.i32;
	text[count] = 0;
	wasmtime_anyref_unroot(&bytes);
	if (length) *length = count;
	return text;
}
static inline bool warp_text_is(warp_values* values, const wasmtime_anyref_t* node, const char* name) {
	char* text = warp_text_bytes(values, node, NULL);
	bool same = strcmp(text, name) == 0;
	free(text);
	return same;
}
// a list's (a map's, a block's) $list: its length and items
static inline wasmtime_anyref_t warp_list_of(warp_values* values, const wasmtime_anyref_t* node) {
	int kind = warp_node_kind(values, node);
	if (kind != WARP_KIND_LIST && kind != WARP_KIND_BLOCK) warp_failed(values, "a list expected");
	return warp_field_ref(values, node, WARP_NODE_DATA);
}
static inline size_t warp_list_length(warp_values* values, const wasmtime_anyref_t* node) {
	wasmtime_anyref_t list = warp_list_of(values, node);
	size_t length = (size_t)warp_field(values, &list, WARP_LIST_LENGTH).of.i32;
	wasmtime_anyref_unroot(&list);
	return length;
}
// the item at index (from 0), a new root
static inline wasmtime_anyref_t warp_list_item(warp_values* values, const wasmtime_anyref_t* node, size_t index) {
	wasmtime_anyref_t list = warp_list_of(values, node);
	wasmtime_anyref_t items = warp_field_ref(values, &list, WARP_LIST_ITEMS);
	wasmtime_anyref_t item = warp_array_item(values, &items, (uint32_t)index).of.anyref;
	wasmtime_anyref_unroot(&items);
	wasmtime_anyref_unroot(&list);
	return item;
}
// a key's (a map entry's) two sides
static inline wasmtime_anyref_t warp_key_left(warp_values* values, const wasmtime_anyref_t* key) { return warp_field_ref(values, key, WARP_NODE_DATA); }
static inline wasmtime_anyref_t warp_key_right(warp_values* values, const wasmtime_anyref_t* key) { return warp_field_ref(values, key, WARP_NODE_VALUE); }
// the value of a map's entry of this name, found reports whether it has one
static inline wasmtime_anyref_t warp_map_get(warp_values* values, const wasmtime_anyref_t* map, const char* name, bool* found) {
	size_t count = warp_list_length(values, map);
	for (size_t i = 0; i < count; i++) {
		wasmtime_anyref_t entry = warp_list_item(values, map, i);
		wasmtime_anyref_t left = warp_key_left(values, &entry);
		bool named = warp_text_is(values, &left, name);
		wasmtime_anyref_unroot(&left);
		if (named) {
			wasmtime_anyref_t right = warp_key_right(values, &entry);
			wasmtime_anyref_unroot(&entry);
			if (found) *found = true;
			return right;
		}
		wasmtime_anyref_unroot(&entry);
	}
	if (found) *found = false;
	wasmtime_anyref_t none;
	wasmtime_anyref_set_null(&none);
	return none;
}
// an instance (core/emitter/instances.warp class_struct): its class's name, a field by name through its list of field
// names (a declared one at WARP_INSTANCE_DECLARED and on, in order), the map of its undeclared fields
static inline char* warp_class_name(warp_values* values, const wasmtime_anyref_t* instance) {
	wasmtime_anyref_t name = warp_field_ref(values, instance, WARP_INSTANCE_DATA);
	char* text = warp_text_bytes(values, &name, NULL);
	wasmtime_anyref_unroot(&name);
	return text;
}
static inline wasmtime_anyref_t warp_instance_extra(warp_values* values, const wasmtime_anyref_t* instance) {
	return warp_field_ref(values, instance, WARP_INSTANCE_EXTRA);
}
static inline wasmtime_anyref_t warp_instance_field(warp_values* values, const wasmtime_anyref_t* instance, const char* name, bool* found) {
	wasmtime_anyref_t names = warp_field_ref(values, instance, WARP_INSTANCE_VALUE);
	size_t count = warp_list_length(values, &names);
	for (size_t i = 0; i < count; i++) {
		wasmtime_anyref_t field_name = warp_list_item(values, &names, i);
		bool named = warp_text_is(values, &field_name, name);
		wasmtime_anyref_unroot(&field_name);
		if (named) {
			wasmtime_anyref_unroot(&names);
			if (found) *found = true;
			return warp_field_ref(values, instance, WARP_INSTANCE_DECLARED + i);
		}
	}
	wasmtime_anyref_unroot(&names);
	wasmtime_anyref_t extra = warp_instance_extra(values, instance);
	wasmtime_anyref_t field = wasmtime_anyref_is_null(&extra) ? extra : warp_map_get(values, &extra, name, found);
	if (wasmtime_anyref_is_null(&extra) && found) *found = false;
	if (!wasmtime_anyref_is_null(&extra)) wasmtime_anyref_unroot(&extra);
	return field;
}

// the module's types, the recursive ones from the given values: a node, and a list wherever one comes (warp_list_known)
static inline wasmtime_struct_ref_pre_t* warp_struct_of(warp_values* values, const wasmtime_anyref_t* ref) {
	wasmtime_structref_t structure;
	if (!wasmtime_anyref_as_struct(values->context, ref, &structure)) return NULL;
	wasmtime_struct_type_t* type = wasmtime_structref_type(values->context, &structure);
	wasmtime_struct_ref_pre_t* pre = wasmtime_struct_ref_pre_new(values->context, type);
	wasmtime_struct_type_delete(type);
	wasmtime_structref_unroot(&structure);
	return pre;
}
static inline wasmtime_array_ref_pre_t* warp_array_of(warp_values* values, const wasmtime_anyref_t* ref) {
	wasmtime_arrayref_t array;
	if (!wasmtime_anyref_as_array(values->context, ref, &array)) return NULL;
	wasmtime_array_type_t* type = wasmtime_arrayref_type(values->context, &array);
	wasmtime_array_ref_pre_t* pre = wasmtime_array_ref_pre_new(values->context, type);
	wasmtime_array_type_delete(type);
	wasmtime_arrayref_unroot(&array);
	return pre;
}
static inline wasmtime_struct_ref_pre_t* warp_box_type(warp_values* values, wasm_valkind_t kind) {
	wasmtime_field_type_t field = {.mutable_ = false, .storage = {.kind = WASMTIME_STORAGE_TYPE_KIND_VALTYPE, .valtype = wasm_valtype_new(kind)}};
	wasmtime_struct_type_t* type = wasmtime_struct_type_new(values->engine, &field, 1);
	wasmtime_struct_ref_pre_t* pre = wasmtime_struct_ref_pre_new(values->context, type);
	wasmtime_struct_type_delete(type);
	wasm_valtype_delete(field.storage.valtype);
	return pre;
}
// a list's types learnt from a given list node, the first one seen
static inline void warp_list_known(warp_values* values, const wasmtime_anyref_t* node) {
	if (values->list) return;
	wasmtime_anyref_t list = warp_field_ref(values, node, WARP_NODE_DATA);
	wasmtime_anyref_t items = warp_field_ref(values, &list, WARP_LIST_ITEMS);
	values->list = warp_struct_of(values, &list);
	values->items = warp_array_of(values, &items);
	wasmtime_anyref_unroot(&items);
	wasmtime_anyref_unroot(&list);
}
static inline warp_values warp_values_of(void* engine, wasmtime_caller_t* caller, const wasmtime_anyref_t* node) {
	warp_values values = {.engine = engine, .caller = caller, .context = wasmtime_caller_context(caller)};
	values.node = warp_struct_of(&values, node);
	values.int_box = warp_box_type(&values, WASM_I64);
	values.float_box = warp_box_type(&values, WASM_F64);
	wasmtime_field_type_t byte = {.mutable_ = true, .storage = {.kind = WASMTIME_STORAGE_TYPE_KIND_I8}};
	wasmtime_array_type_t* text = wasmtime_array_type_new(values.engine, &byte);
	values.text = wasmtime_array_ref_pre_new(values.context, text);
	wasmtime_array_type_delete(text);
	int kind = warp_node_kind(&values, node);
	if (kind == WARP_KIND_LIST || kind == WARP_KIND_BLOCK) warp_list_known(&values, node);
	return values;
}
static inline void warp_values_done(warp_values* values) {
	wasmtime_struct_ref_pre_t* structs[] = {values->node, values->list, values->int_box, values->float_box};
	for (size_t i = 0; i < sizeof structs / sizeof *structs; i++) if (structs[i]) wasmtime_struct_ref_pre_delete(structs[i]);
	if (values->items) wasmtime_array_ref_pre_delete(values->items);
	if (values->text) wasmtime_array_ref_pre_delete(values->text);
}

// building: each a new root, its parts consumed (unrooted)
static inline wasmtime_anyref_t warp_struct_new(warp_values* values, wasmtime_struct_ref_pre_t* pre, wasmtime_val_t* fields, size_t count) {
	wasmtime_structref_t made;
	wasmtime_anyref_t ref;
	wasmtime_anyref_set_null(&ref);
	wasmtime_error_t* error = pre ? wasmtime_structref_new(values->context, pre, fields, count, &made) : NULL;
	for (size_t i = 0; i < count; i++) warp_unroot(&fields[i]);
	if (!pre || error) {
		if (error) wasmtime_error_delete(error);
		warp_failed(values, "a warp value this word can't build");
		return ref;
	}
	wasmtime_structref_to_anyref(&made, &ref);
	wasmtime_structref_unroot(&made);
	return ref;
}
static inline wasmtime_anyref_t warp_node_new(warp_values* values, int64_t kind, wasmtime_anyref_t data, wasmtime_anyref_t* value) {
	wasmtime_val_t fields[3] = {{.kind = WASMTIME_I64, .of.i64 = kind}, warp_anyref(data), {.kind = WASMTIME_ANYREF}};
	if (value) fields[2].of.anyref = *value; else wasmtime_anyref_set_null(&fields[2].of.anyref);
	return warp_struct_new(values, values->node, fields, 3);
}
static inline wasmtime_anyref_t warp_int(warp_values* values, int64_t number) {
	wasmtime_val_t field = {.kind = WASMTIME_I64, .of.i64 = number};
	return warp_node_new(values, WARP_KIND_INT, warp_struct_new(values, values->int_box, &field, 1), NULL);
}
static inline wasmtime_anyref_t warp_float(warp_values* values, double number) {
	wasmtime_val_t field = {.kind = WASMTIME_F64, .of.f64 = number};
	return warp_node_new(values, WARP_KIND_FLOAT, warp_struct_new(values, values->float_box, &field, 1), NULL);
}
static inline wasmtime_anyref_t warp_bytes_node(warp_values* values, int64_t kind, const char* bytes, size_t length) {
	wasmtime_val_t zero = {.kind = WASMTIME_I32, .of.i32 = 0};
	wasmtime_arrayref_t array;
	wasmtime_anyref_t data;
	wasmtime_anyref_set_null(&data);
	wasmtime_error_t* error = wasmtime_arrayref_new(values->context, values->text, &zero, (uint32_t)length, &array);
	if (error) { wasmtime_error_delete(error); warp_failed(values, "a text too long"); return data; }
	for (size_t i = 0; i < length; i++) {
		wasmtime_val_t byte = {.kind = WASMTIME_I32, .of.i32 = (uint8_t)bytes[i]};
		wasmtime_arrayref_set(values->context, &array, (uint32_t)i, &byte);
	}
	wasmtime_arrayref_to_anyref(&array, &data);
	wasmtime_arrayref_unroot(&array);
	return warp_node_new(values, kind, data, NULL);
}
static inline wasmtime_anyref_t warp_text(warp_values* values, const char* bytes, size_t length) { return warp_bytes_node(values, WARP_KIND_TEXT, bytes, length); }
static inline wasmtime_anyref_t warp_symbol(warp_values* values, const char* name) { return warp_bytes_node(values, WARP_KIND_SYMBOL, name, strlen(name)); }
// a list of these items (consumed), its kind field one of WARP_MARKED_SQUARELIST, WARP_MARKED_MAP …
static inline wasmtime_anyref_t warp_list(warp_values* values, int64_t kind, wasmtime_anyref_t* items, size_t count) {
	wasmtime_anyref_t none;
	wasmtime_anyref_set_null(&none);
	if (!values->items) { warp_failed(values, "a list built by a word given none"); return none; }
	wasmtime_val_t null_item = warp_anyref(none);
	wasmtime_arrayref_t array;
	wasmtime_error_t* error = wasmtime_arrayref_new(values->context, values->items, &null_item, (uint32_t)count, &array);
	if (error) { wasmtime_error_delete(error); warp_failed(values, "a list too long"); return none; }
	for (size_t i = 0; i < count; i++) {
		wasmtime_val_t item = warp_anyref(items[i]);
		wasmtime_arrayref_set(values->context, &array, (uint32_t)i, &item);
		warp_unroot(&item);
	}
	wasmtime_val_t fields[2] = {{.kind = WASMTIME_I32, .of.i32 = (int32_t)count}, {.kind = WASMTIME_ANYREF}};
	wasmtime_arrayref_to_anyref(&array, &fields[1].of.anyref);
	wasmtime_arrayref_unroot(&array);
	return warp_node_new(values, kind, warp_struct_new(values, values->list, fields, 2), NULL);
}
// a map's entry `name: value` (value consumed)
static inline wasmtime_anyref_t warp_entry(warp_values* values, const char* name, wasmtime_anyref_t value) {
	return warp_node_new(values, WARP_MARKED_ENTRY, warp_symbol(values, name), &value);
}

// the word's answer: the value, or after a failure the module's Error of its message (its exported error_of)
static inline wasm_trap_t* warp_answered(warp_values* values, wasmtime_anyref_t value, wasmtime_val_t* result) {
	wasm_trap_t* trap = NULL;
	if (values->failure[0]) {
		if (!wasmtime_anyref_is_null(&value)) wasmtime_anyref_unroot(&value);
		wasmtime_val_t message = warp_anyref(warp_text(values, values->failure, strlen(values->failure)));
		wasmtime_extern_t error_of;
		if (!wasmtime_caller_export_get(values->caller, "error_of", 8, &error_of) || error_of.kind != WASMTIME_EXTERN_FUNC) {
			trap = wasmtime_trap_new(values->failure, strlen(values->failure));
		} else {
			wasmtime_error_t* error = wasmtime_func_call(values->context, &error_of.of.func, &message, 1, result, 1, &trap);
			if (error) trap = wasmtime_trap_new(values->failure, strlen(values->failure)), wasmtime_error_delete(error);
		}
		warp_unroot(&message);
	} else {
		*result = warp_anyref(value);
	}
	warp_values_done(values);
	return trap;
}
