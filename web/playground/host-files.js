// Files and synchronous requests in the page (a part of host.js, which says how parts work): the host words fetch and
// read, std's file and net modules, and the files of the served repository the other parts read (host-foreign.js)

/// where host.read and the test runner's file system find files: the repository root the static server serves
const FILE_ROOT = new URL("../../", self.location.href).href;
const PAGE_PREFIX = "page:"; // src/web.rs PAGE_PREFIX: a file of the page itself (lib/libc.h), not of the served repository
const HTTP_NOT_FOUND = "HTTP status 404";
const FILE_NOT_FOUND = "No such file or directory (os error 2)";

// the files std's file module wrote, kept while the page is open: read and the file words see them before the served ones
const writtenFiles = new Map();
const filePath = path => path.replace(/^\.\//, "");

const textOfFile = path => writtenFiles.get(filePath(path)) ?? (servedFileExists(path) ? readFile(filePath(path)) : "");
function servedFileExists(path) {
	try {
		readBytes(path);
		return true;
	} catch {
		return false;
	}
}

// a path of the served root that served-files.js (build.sh) does not list: missing without asking, since every 404 is
// an error in the console (card console-errors)
const isUnserved = url => typeof SERVED_FILES !== "undefined" && url.startsWith(FILE_ROOT)
	&& !SERVED_FILES.has(decodeURIComponent(new URL(url).pathname.slice(new URL(FILE_ROOT).pathname.length)));

// a synchronous GET (host calls are synchronous, so this runs in a worker); `timeout` in ms
function getSync(url, timeout, binary = false) {
	if (isUnserved(url)) throw new Error(HTTP_NOT_FOUND);
	const request = new XMLHttpRequest();
	request.open("GET", url, false);
	if (timeout) request.timeout = timeout;
	if (binary) request.overrideMimeType("text/plain; charset=x-user-defined"); // bytes as they are
	request.send();
	if (request.status >= 400 || request.status === 0) throw new Error(request.status ? `HTTP status ${request.status}` : "network error (blocked by CORS?)");
	return binary ? Uint8Array.from(request.responseText, character => character.charCodeAt(0) & 0xff) : request.responseText;
}

// a synchronous POST of a text with its headers (stdlib net's post, src/extensions/utils.rs post_within), its answer's
// text; an answer of status >= 400 is the error, with its text (an API's own reason)
function postSync(url, body, headers = {}) {
	const request = new XMLHttpRequest();
	request.open("POST", url, false);
	request.setRequestHeader("Content-Type", "text/plain; charset=utf-8");
	for (const [name, value] of Object.entries(headers)) request.setRequestHeader(name, withPageSecret(url, value));
	request.send(body);
	if (request.status === 0) throw new Error("network error (blocked by CORS?)");
	if (request.status >= 400) throw new Error(`HTTP status ${request.status}: ${request.responseText}`);
	return request.responseText;
}

// A program sees a secret of the page (the playground's API key, assistant.js) only as its stand-in `env` gives:
// the secret itself goes into a header of a request to its own service, nowhere else, so no program can send it away
function withPageSecret(url, value) {
	const secret = self.pageSecrets?.[value];
	if (!secret) return value;
	if (new URL(url).origin !== secret.origin) throw new Error(`the playground's key goes only to ${secret.origin}, not to ${url}`);
	return secret.value;
}

// a request of a served file, failing in the words of the native read (src/host.rs)
function asFileRead(request) {
	try {
		return request();
	} catch (failure) {
		throw failure.message === HTTP_NOT_FOUND ? new Error(FILE_NOT_FOUND) : failure;
	}
}

const readFile = path => asFileRead(() => getSync(FILE_ROOT + path));

// the URL of a file of the served repository, of the page itself (PAGE_PREFIX) or a URL
function fileUrl(path) {
	if (/^https?:/.test(path)) return path;
	if (path.startsWith(PAGE_PREFIX)) return new URL(path.slice(PAGE_PREFIX.length), self.location.href).href;
	return FILE_ROOT + filePath(path);
}

// the bytes of a file of the served repository, of the page or of a URL, failing in the words of the native read
function readBytes(path) {
	const written = writtenFiles.get(filePath(path));
	if (written !== undefined) return utf8.encode(written);
	return asFileRead(() => getSync(fileUrl(path), 0, true));
}

// the body of a host call as (pointer, length), or (pointer, -length) of the failure reason, like src/host.rs
function hostResult(program, action, what) {
	return hostBytes(program, () => {
		const text = action();
		return utf8.encode(text.endsWith("\n") ? text : text + "\n"); // warp convention (src/host.rs fetch)
	}, what);
}

function hostBytes(program, action, what) {
	try {
		return writeBytes(program, action());
	} catch (reason) {
		const [pointer, length] = writeBytes(program, utf8.encode(`${what} failed: ${reason.message ?? reason}`));
		return [pointer, -length];
	}
}

// `database[k]`: the store of any file ending so (src/lowering/stored_values.rs DATABASE_STORE), kept in IndexedDB, which
// Workers have too; its values are read before the program runs (loadDatabase: worker.js, site-worker.js, site.js) and
// each change written back where the host keeps values (self.keepStored; the browser test suite keeps them in memory)
const DATABASE_STORE = "database.json";
const DATABASE = { name: "warp", version: 1, objects: "values" }; // the IndexedDB database and its object store
const databaseValues = {};
const isDatabase = file => file.endsWith(DATABASE_STORE);
let databaseLoaded;

// the values of a store: the session's (markup.js SESSION_STORE), the database's, else the program's
const valuesOf = file => file === "warp-session" ? sessionValues : isDatabase(file) ? databaseValues : storedValues;

// the IndexedDB object store of `database[k]` in a transaction of `mode`
function databaseObjects(mode) {
	return new Promise((resolve, reject) => {
		const opening = indexedDB.open(DATABASE.name, DATABASE.version);
		opening.onupgradeneeded = () => opening.result.createObjectStore(DATABASE.objects);
		opening.onsuccess = () => resolve(opening.result.transaction(DATABASE.objects, mode).objectStore(DATABASE.objects));
		opening.onerror = () => reject(opening.error);
	});
}

const requested = request => new Promise((resolve, reject) => {
	request.onsuccess = () => resolve(request.result);
	request.onerror = () => reject(request.error);
});

// databaseValues from IndexedDB, once; none when it fails (loudly on the console)
function loadDatabase() {
	return databaseLoaded ??= databaseObjects("readonly")
		.then(objects => Promise.all([requested(objects.getAllKeys()), requested(objects.getAll())]))
		.then(([names, values]) => names.forEach((name, index) => takeDatabaseValue(name, values[index])))
		.catch(failure => console.error("database values could not be read:", failure));
}

// the other tabs' workers keep the same database: each change is told to them, so their tables stay current (a row
// another tab added is there, the next id is past it) and none writes back rows it did not change (card playground-two)
const DATABASE_CHANNEL = "warp database";
const databaseChannel = typeof BroadcastChannel === "undefined" ? null : new BroadcastChannel(DATABASE_CHANNEL);
if (databaseChannel) databaseChannel.onmessage = ({ data: { name, value } }) => takeDatabaseValue(name, value);

// a change of `database[k]` (value undefined: deleted), written to IndexedDB where the host keeps values
function keep(name, value, file) {
	if (!isDatabase(file)) return self.keepStored?.(name, value, file);
	if (!self.keepStored) return;
	databaseObjects("readwrite")
		.then(objects => requested(value === undefined ? objects.delete(name) : objects.put(value, name)))
		.catch(failure => console.error(`${name} could not be kept in the database:`, failure));
	databaseChannel?.postMessage({ name, value });
}

// a value of the database read or told: a table's row goes into its table's rows, a table keeps the rows it has (a
// table kept as one value before rows had keys of their own brings its rows, which its next migration keeps one by one)
function takeDatabaseValue(name, value) {
	const row = name.match(ROW_KEY);
	if (!row) {
		const keptWhole = Array.isArray(value?.rows);
		databaseValues[name] = name.startsWith(TABLE_PREFIX) && value ? { rows: databaseValues[name]?.rows ?? [], ...value, keptWhole } : value;
		if (value === undefined) delete databaseValues[name];
		return;
	}
	const [, table, id] = row;
	const stored = databaseValues[table] ??= { types: {}, quantities: {}, rows: [] };
	stored.rows = stored.rows.filter(kept => kept[ID_COLUMN] !== Number(id));
	if (value !== undefined) stored.rows.push(value);
	stored.rows.sort((first, second) => first[ID_COLUMN] - second[ID_COLUMN]);
}

// `people: [Person] = database.people` (src/lowering/database_tables.rs): a table of the browser, kept in the database
// store (IndexedDB, as `database[k]`), where natively SQLite keeps it (src/database.rs): its column types under the
// table's key, each row, an object with its id, under its own (`table people.database.json people #3`), so a change
// writes only its row. In memory a table holds its rows. Filters stay list comprehensions over the rows (no SQL here).
const TABLE_PREFIX = "table ";
const ID_COLUMN = "id";
const tableKey = (file, table) => `${TABLE_PREFIX}${file} ${table}`;
const ROW_MARK = " #";
const ROW_KEY = /^(table .*) #(\d+)$/;
const rowKey = (file, table, id) => `${tableKey(file, table)}${ROW_MARK}${id}`;
// field types by column (src/database.rs column_of), each holding every value of the ones before it
const LOSSLESS_ORDER = [["int", "i64", "i32", "bool"], ["float", "f64", "f32", "number"], ["text", "string", "str", "char"]];
const REAL = 1;
// `text?`, an optional field: a column of its type that may hold null
const plainType = type => type.replace(/\?$/, "");
const widening = type => LOSSLESS_ORDER.findIndex(types => types.includes(plainType(type)));
// texts as SQLite's CAST writes them: a real keeps its ".0"
const textOfColumn = (value, from) => from === REAL && Number.isInteger(value) ? value.toFixed(1) : String(value);

// a converter of rows retyping the column as src/database.rs converted does (conversion_factor), else the same loud
// error. A unit column (`distance: km`) holds SI amounts: another unit of its quantity keeps them, plain numbers given a
// unit are read as it, loudly. Each column is [type, quantity, SI amount per unit], the last two for units only
function convertColumn(table, name, [storedType, storedQuantity], [type, quantity, perUnit], warn) {
	const column = `the column ${table}.${name} holds ${storedType}, the class's field is ${type}`;
	const [from, to] = [widening(storedType), widening(type)];
	if (storedQuantity && quantity) {
		if (storedQuantity !== quantity) throw new Error(`${column}: ${storedQuantity} and ${quantity} are different quantities, so it is not converted`);
		return () => {};
	}
	if (quantity && from >= 0 && from <= REAL) {
		warn(`${column}: its plain numbers are read as ${plainType(type)} (each × ${perUnit} to the SI amount a unit field holds)`);
		return row => { if (row[name] != null) row[name] = Number(row[name]) * perUnit; };
	}
	if (storedQuantity) throw new Error(`${column}: the unit would be lost, so it is not converted`);
	if (from < 0 || to <= from) throw new Error(`${column}: its values would lose data, so it is not converted`);
	return row => { row[name] = to === REAL ? Number(row[name]) : textOfColumn(row[name], from); };
}
// a unit column's quantity is kept in quantities (`m` of km), the browser has no unit table to look it up later
const storedTable = (file, table) => databaseValues[tableKey(file, table)] ?? { types: {}, quantities: {}, rows: [] };
// the table's column types, with every row when `rows` (a migration changed them, or they were kept as one value)
function keepTable(file, table, stored, rows) {
	const key = tableKey(file, table);
	const { rows: kept, keptWhole, ...columns } = stored;
	databaseValues[key] = { ...columns, rows: kept };
	keep(key, columns, DATABASE_STORE);
	if (rows || keptWhole) kept.forEach(row => keepRow(file, table, row));
}

const keepRow = (file, table, row) => keep(rowKey(file, table, row[ID_COLUMN]), row, DATABASE_STORE);

// the table's rows [id, columns…], the table first created or migrated to the class's fields [name, type, default];
// a removed field keeps its column (data is never dropped silently), loudly
// the table migrated to the class's fields, then its rows; src/database.rs gives the two halves apart too: a table's
// list migrates at registration and loads its rows at the first read (lowering/database_tables.rs)
function openTable(table, schema, file) {
	migrateTable.call(this, table, schema, file);
	return tableRows(table, schema, file);
}

const tableRows = (table, schema, file) => storedTable(file, table).rows.map(row => [row[ID_COLUMN], ...schema.map(([name]) => row[name])]);

function migrateTable(table, schema, file) {
	const stored = storedTable(file, table);
	const before = JSON.stringify(stored);
	stored.quantities ??= {};
	for (const [name, , , oldName] of schema.filter(([name, , , oldName]) => oldName in stored.types && !(name in stored.types))) {
		renameColumn(stored, oldName, name);
	}
	for (const [name, type, fallback, , quantity, perUnit] of schema) {
		const storedType = stored.types[name];
		if (storedType === undefined) {
			stored.rows.forEach(row => { row[name] = fallback; });
		} else if (plainType(storedType) !== plainType(type)) {
			const warn = message => this.warn(message);
			stored.rows.forEach(convertColumn(table, name, [storedType, stored.quantities[name]], [type, quantity, perUnit], warn));
		}
		stored.types[name] = type;
		if (quantity) stored.quantities[name] = quantity; else delete stored.quantities[name];
	}
	const fields = schema.map(([name]) => name);
	for (const name of Object.keys(stored.types).filter(name => !fields.includes(name))) {
		this.warn(`the table ${table} keeps its column ${name}, which the class no longer has (its data is kept)`);
	}
	keepTable(file, table, stored, JSON.stringify(stored) !== before);
	return null;
}

// a field marked `@was(oldName)`: its column keeps type and values under the new name
function renameColumn(stored, oldName, name) {
	stored.types[name] = stored.types[oldName];
	delete stored.types[oldName];
	if (oldName in stored.quantities) { stored.quantities[name] = stored.quantities[oldName]; delete stored.quantities[oldName]; }
	stored.rows.forEach(row => { row[name] = row[oldName]; delete row[oldName]; });
}

function insertRow(table, columns, values, file) {
	const stored = storedTable(file, table);
	const id = Math.max(0, ...stored.rows.map(row => row[ID_COLUMN])) + 1;
	const row = Object.fromEntries([[ID_COLUMN, id], ...columns.map((column, index) => [column, values[index]])]);
	stored.rows.push(row);
	databaseValues[tableKey(file, table)] = stored;
	keepRow(file, table, row);
	return id;
}

function deleteRow(table, id, file) {
	const stored = storedTable(file, table);
	stored.rows = stored.rows.filter(row => row[ID_COLUMN] !== id);
	keep(rowKey(file, table, id), undefined, DATABASE_STORE);
	return null;
}

function updateRow(table, id, column, value, file) {
	const stored = storedTable(file, table);
	const row = stored.rows.find(row => row[ID_COLUMN] === id);
	if (row) row[column] = value;
	if (row) keepRow(file, table, row);
	return null;
}

// `transaction { … }` (src/lowering/database_tables.rs): its file's tables as they were at the start, put back with every
// row's kept value by a rollback; writes go on to IndexedDB meanwhile, as SQLite's would to its journal
const tablesBefore = {};
const tablesOf = file => Object.keys(databaseValues).filter(key => key.startsWith(tableKey(file, "")) && !ROW_KEY.test(key));

function beginTransaction(file) {
	tablesBefore[file] = Object.fromEntries(tablesOf(file).map(key => [key, structuredClone(databaseValues[key])]));
	return null;
}

function rollBack(file) {
	const before = tablesBefore[file] ?? {};
	delete tablesBefore[file];
	for (const key of tablesOf(file)) {
		const table = key.slice(tableKey(file, "").length);
		const keptIds = new Set((before[key]?.rows ?? []).map(row => row[ID_COLUMN]));
		databaseValues[key].rows.filter(row => !keptIds.has(row[ID_COLUMN])).forEach(row => keep(rowKey(file, table, row[ID_COLUMN]), undefined, DATABASE_STORE));
		if (key in before) keepTable(file, table, before[key], true); else { delete databaseValues[key]; keep(key, undefined, DATABASE_STORE); }
	}
	return null;
}

addHostPart({
	words: (holder, hooks, { program, text }) => {
		const fetchUrl = (pointer, length, timeout) => {
			const url = text(pointer, length);
			return hostResult(program(), () => getSync(url, timeout), `fetch ${url}`);
		};
		return {
			fetch: (pointer, length) => fetchUrl(pointer, length),
			fetch_within: (pointer, length, milliseconds) => fetchUrl(pointer, length, Number(milliseconds)),
			// the file's bytes as they are, like src/host.rs read; a URL (a package's own files) is fetched
			read: (pointer, length) => {
				const path = text(pointer, length);
				return hostBytes(program(), () => readBytes(path), `read ${path}`);
			},
		};
	},
	adapters: {
		// a page has no command line; its environment is what the page sets (the stand-in of the playground's key)
		os: { env: name => self.pageEnvironment?.[name] ?? null, args: () => [] },
		// `stored theme = "dark"`, `local[k]`, `session[k]` (src/lowering/stored_values.rs, std_io): the page's values
		// (markup.js keptValues), each save sent back to it with its store (the dev store of a `warp dev` page, the
		// session's, else the program's)
		store: {
			load: (name, fallback, file) => name in valuesOf(file) ? valuesOf(file)[name] : fallback,
			save: (name, value, file) => { valuesOf(file)[name] = value; keep(name, value, file); return null; },
			// `delete local[k]` (kept as undefined: markup.js keepValue drops it), `keys(local)`
			remove: (name, file) => { delete valuesOf(file)[name]; keep(name, undefined, file); return null; },
			names: file => Object.keys(valuesOf(file)),
		},
		file: {
			write: (path, content) => { writtenFiles.set(filePath(path), contentText(content)); return null; },
			append: (path, content) => { writtenFiles.set(filePath(path), textOfFile(path) + contentText(content)); return null; },
			exists: path => writtenFiles.has(filePath(path)) || servedFileExists(path),
			list: folder => {
				const prefix = filePath(folder).replace(/\/?$/, "/");
				return [...writtenFiles.keys()].filter(path => path.startsWith(prefix) && !path.slice(prefix.length).includes("/")).map(path => path.slice(prefix.length)).sort();
			},
		},
		// a filter compiled natively is an SQL query (a page compiled by the browser keeps filters as comprehensions)
		table: { open: openTable, migrate: migrateTable, rows: tableRows, page: (table, schema, file, start, size) => tableRows(table, schema, file).slice(start - 1, start - 1 + size), count: (table, schema, file) => storedTable(file, table).rows.length, insert: insertRow, delete: deleteRow, update: updateRow, begin: beginTransaction, commit: file => { delete tablesBefore[file]; return null; }, rollback: rollBack, select: table => { throw new Error(`a filter of the table ${table} is an SQL query: it runs natively (warp serve)`); } },
		net: { post: (url, body, headers) => postSync(url, contentText(body), headers ?? {}) },
		// `clipboard.write(text)` (lowering/system_values.rs): a page writes it (markup.js copyText), a Worker has no
		// clipboard and hands the text to its page (self.writeClipboard: worker.js)
		clipboard: { write: text => { (self.writeClipboard ?? copyText)(contentText(text)); return null; } },
		// `play "song.mp3"`, `stop_sound` (lib/sound.warp, card sound-library): the page plays it with an <audio>
		// (worker.js self.playSoundFile, playground.js); a run without a page (tests, node) stays silent, as natively
		sound: {
			play_file: path => { self.playSoundFile?.(contentText(path)); return null; },
			stop: () => { self.stopSoundFiles?.(); return null; },
			// the page's audio clock as the worker keeps it (worker.js); a worker cannot wait for the page's audio
			queued: () => self.soundsQueued?.() ?? 0,
			wait: () => null,
		},
	},
});
