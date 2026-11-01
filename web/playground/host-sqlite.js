// `use sqlite3` (lib/native/sqlite3.warp, natively libsqlite3) answered by SQLite's own WebAssembly build, its C API
// called as natively (host-foreign.js callC). Its JavaScript glue is left out: the module is instantiated here with the
// few emscripten and WASI imports SQLite reaches, so it loads synchronously at the first call. Files: a database a
// program opens by path is read from the page's files (the served ones under its writes) into memory, and written back
// there (kept in IndexedDB, host-files.js) when it closes or the run ends, so it stays from run to run as natively.
// Needs host-foreign.js and wasi.js.

// a release of @sqlite.org/sqlite-wasm; its glue's imports are the ones answered below
const SQLITE_URL = "https://cdn.jsdelivr.net/npm/@sqlite.org/sqlite-wasm@3.54.1-build1/dist/sqlite3.wasm";
const SQLITE_FIRST_PAGES = 256;
const SQLITE_MOST_PAGES = 32768;
const NO_SUCH_CALL = -52; // ENOSYS: SQLite's own files are the page's, never its virtual file system's
const IN_MEMORY = ":memory:";
const MAIN_SCHEMA = "main";
const SQLITE_DESERIALIZE_FREEONCLOSE = 1;
const SQLITE_DESERIALIZE_RESIZEABLE = 2;
const SQLITE_OK = 0;

let sqliteModule; // SQLite's exports, its memory and malloc among them, once per worker

function sqliteExports() {
	if (sqliteModule) return sqliteModule;
	const bytes = new Uint8Array(getSync(SQLITE_URL, 0, true));
	const memory = new WebAssembly.Memory({ initial: SQLITE_FIRST_PAGES, maximum: SQLITE_MOST_PAGES });
	const view = () => new DataView(memory.buffer);
	const env = {
		memory,
		emscripten_date_now: () => Date.now(),
		emscripten_get_now: () => performance.now(),
		emscripten_get_heap_max: () => SQLITE_MOST_PAGES * WASM_PAGE_BYTES,
		emscripten_resize_heap: wanted => {
			try { memory.grow(Math.ceil((wanted - memory.buffer.byteLength) / WASM_PAGE_BYTES)); return 1; } catch { return 0; }
		},
	};
	const wasi = {
		environ_sizes_get: (count, size) => { view().setUint32(count, 0, true); view().setUint32(size, 0, true); return 0; },
		environ_get: () => 0,
		clock_time_get: (clock, precision, time) => { view().setBigUint64(time, BigInt(Date.now()) * 1000000n, true); return 0; },
	};
	const refusing = answered => new Proxy(answered, { get: (known, name) => known[name] ?? (() => NO_SUCH_CALL) });
	const module = new WebAssembly.Module(bytes);
	const exports = new WebAssembly.Instance(module, { env: refusing(env), wasi_snapshot_preview1: refusing(wasi) }).exports;
	exports.__wasm_call_ctors();
	exports.sqlite3_initialize();
	return sqliteModule = { ...typedExports(bytes, exports), memory };
}

// the run's SQLite: its open and close keep the databases opened by path as the program's files
function sqliteLibrary(holder) {
	return wasmLibraryImports(holder, "sqlite3", () => holder.sqlite ??= runSqlite(holder));
}

function runSqlite(holder) {
	const sqlite = sqliteExports();
	const files = holder.sqliteFiles = new Map(); // database → its file
	const memory = () => new DataView(sqlite.memory.buffer);
	const open = (name, slot) => {
		const file = normalPath(utf8Decoder.decode(cText(sqlite.memory, name)));
		if (file === IN_MEMORY || file === "") return sqlite.sqlite3_open(name, slot);
		const inMemory = cBlock(sqlite, IN_MEMORY);
		const status = sqlite.sqlite3_open(inMemory, slot);
		sqlite.free(inMemory);
		if (status !== SQLITE_OK) return status;
		const database = memory().getUint32(slot, true);
		files.set(database, file);
		const content = programFiles.written.get(file) ?? served(file)?.bytes;
		return content?.length ? loaded(sqlite, database, contentBytes(content)) : status;
	};
	const close = database => {
		keptDatabase(sqlite, database, files);
		files.delete(database);
		return sqlite.sqlite3_close_v2(database);
	};
	return { ...sqlite, sqlite3_open: open, sqlite3_close: close };
}

// the file's bytes as the database's content
function loaded(sqlite, database, bytes) {
	const block = sqlite.sqlite3_malloc(bytes.length);
	new Uint8Array(sqlite.memory.buffer).set(bytes, block);
	const schema = cBlock(sqlite, MAIN_SCHEMA);
	const status = sqlite.sqlite3_deserialize(database, schema, block, BigInt(bytes.length), BigInt(bytes.length), SQLITE_DESERIALIZE_FREEONCLOSE | SQLITE_DESERIALIZE_RESIZEABLE);
	sqlite.free(schema);
	return status;
}

// the database's content written to its file, a database of no file left as it is
function keptDatabase(sqlite, database, files) {
	const file = files.get(database);
	if (!file) return;
	const schema = cBlock(sqlite, MAIN_SCHEMA);
	const size = sqlite.malloc(8);
	const content = sqlite.sqlite3_serialize(database, schema, size, 0);
	const bytes = new Uint8Array(sqlite.memory.buffer, content, Number(new DataView(sqlite.memory.buffer).getBigInt64(size, true))).slice();
	[schema, size].forEach(sqlite.free);
	sqlite.sqlite3_free(content);
	programFiles.written.set(file, bytes);
	programFiles.kept(file, bytes);
}

// a NUL-terminated copy of the text in SQLite's memory, for its caller to free
function cBlock(sqlite, text) {
	const bytes = utf8.encode(`${text}\0`);
	const block = sqlite.malloc(bytes.length);
	new Uint8Array(sqlite.memory.buffer).set(bytes, block);
	return block;
}

// the databases still open when the run ends are written to their files, as natively each write is on disk already
const keptDatabases = holder => holder.sqliteFiles?.forEach((file, database) => keptDatabase(holder.sqlite, database, holder.sqliteFiles));

addHostPart({
	imports: holder => ({ sqlite3: sqliteLibrary(holder) }),
	finished: keptDatabases,
	ended: keptDatabases,
});
