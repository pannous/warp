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

// a file of the served repository, failing in the words of the native read (src/host.rs)
function readFile(path) {
	try {
		return getSync(FILE_ROOT + path);
	} catch (failure) {
		throw failure.message === HTTP_NOT_FOUND ? new Error(FILE_NOT_FOUND) : failure;
	}
}

// the URL of a file of the served repository, of the page itself (PAGE_PREFIX) or a URL
function fileUrl(path) {
	if (/^https?:/.test(path)) return path;
	if (path.startsWith(PAGE_PREFIX)) return new URL(path.slice(PAGE_PREFIX.length), self.location.href).href;
	return FILE_ROOT + path.replace(/^\.\//, "");
}

// the bytes of a file of the served repository, of the page or of a URL, failing in the words of the native read
function readBytes(path) {
	const written = writtenFiles.get(filePath(path));
	if (written !== undefined) return utf8.encode(written);
	try {
		return getSync(fileUrl(path), 0, true);
	} catch (failure) {
		throw failure.message === HTTP_NOT_FOUND ? new Error(FILE_NOT_FOUND) : failure;
	}
}

// the body of a host call as (pointer, length), or (pointer, -length) of the failure reason, like src/host.rs
function hostResult(program, action, what) {
	try {
		let text = action();
		if (!text.endsWith("\n")) text += "\n"; // warp convention (src/host.rs fetch)
		return writeBytes(program, utf8.encode(text));
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
		.then(([names, values]) => names.forEach((name, index) => { databaseValues[name] = values[index]; }))
		.catch(failure => console.error("database values could not be read:", failure));
}

// a change of `database[k]` (value undefined: deleted), written to IndexedDB where the host keeps values
function keep(name, value, file) {
	if (!isDatabase(file)) return self.keepStored?.(name, value, file);
	if (!self.keepStored) return;
	databaseObjects("readwrite")
		.then(objects => requested(value === undefined ? objects.delete(name) : objects.put(value, name)))
		.catch(failure => console.error(`${name} could not be kept in the database:`, failure));
}

// `people: [Person] = database.people` (src/lowering/database_tables.rs): a table of the browser, kept as one value of
// the database store (IndexedDB, as `database[k]`), where natively SQLite keeps it (src/database.rs): its column types
// and its rows, objects with their id. Filters stay list comprehensions over the rows (no SQL here).
const TABLE_PREFIX = "table ";
const ID_COLUMN = "id";
const tableKey = (file, table) => `${TABLE_PREFIX}${file} ${table}`;
// field types by column (src/database.rs column_of), each holding every value of the ones before it
const LOSSLESS_ORDER = [["int", "i64", "i32", "bool"], ["float", "f64", "f32", "number"], ["text", "string", "str", "char"]];
const REAL = 1;
// `text?`, an optional field: a column of its type that may hold null
const plainType = type => type.replace(/\?$/, "");
const widening = type => LOSSLESS_ORDER.findIndex(types => types.includes(plainType(type)));
// texts as SQLite's CAST writes them: a real keeps its ".0"
const textOfColumn = (value, from) => from === REAL && Number.isInteger(value) ? value.toFixed(1) : String(value);

// a converter of rows retyping the column as src/database.rs converted does, else the same loud error
function convertColumn(table, name, storedType, type) {
	const [from, to] = [widening(storedType), widening(type)];
	if (from < 0 || to <= from) {
		throw new Error(`the column ${table}.${name} holds ${storedType}, the class's field is ${type}: its values would lose data, so it is not converted`);
	}
	return row => { row[name] = to === REAL ? Number(row[name]) : textOfColumn(row[name], from); };
}
const storedTable = (file, table) => databaseValues[tableKey(file, table)] ?? { types: {}, rows: [] };
function keepTable(file, table, stored) {
	const key = tableKey(file, table);
	databaseValues[key] = stored;
	keep(key, stored, DATABASE_STORE);
}

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
	for (const [name, , , oldName] of schema.filter(([name, , , oldName]) => oldName in stored.types && !(name in stored.types))) {
		renameColumn(stored, oldName, name);
	}
	for (const [name, type, fallback] of schema) {
		const storedType = stored.types[name];
		if (storedType === undefined) {
			stored.types[name] = type;
			stored.rows.forEach(row => { row[name] = fallback; });
		} else if (plainType(storedType) !== plainType(type)) {
			stored.rows.forEach(convertColumn(table, name, storedType, type));
			stored.types[name] = type;
		} else {
			stored.types[name] = type;
		}
	}
	const fields = schema.map(([name]) => name);
	for (const name of Object.keys(stored.types).filter(name => !fields.includes(name))) {
		this.warn(`the table ${table} keeps its column ${name}, which the class no longer has (its data is kept)`);
	}
	keepTable(file, table, stored);
	return null;
}

// a field marked `@was(oldName)`: its column keeps type and values under the new name
function renameColumn(stored, oldName, name) {
	stored.types[name] = stored.types[oldName];
	delete stored.types[oldName];
	stored.rows.forEach(row => { row[name] = row[oldName]; delete row[oldName]; });
}

function insertRow(table, columns, values, file) {
	const stored = storedTable(file, table);
	const id = Math.max(0, ...stored.rows.map(row => row[ID_COLUMN])) + 1;
	stored.rows.push(Object.fromEntries([[ID_COLUMN, id], ...columns.map((column, index) => [column, values[index]])]));
	keepTable(file, table, stored);
	return id;
}

function updateRow(table, id, column, value, file) {
	const stored = storedTable(file, table);
	const row = stored.rows.find(row => row[ID_COLUMN] === id);
	if (row) row[column] = value;
	keepTable(file, table, stored);
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
				try {
					return writeBytes(program(), readBytes(path));
				} catch (reason) {
					const [failed, failedLength] = writeBytes(program(), utf8.encode(`read ${path} failed: ${reason.message ?? reason}`));
					return [failed, -failedLength];
				}
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
		table: { open: openTable, migrate: migrateTable, rows: tableRows, page: (table, schema, file, start, size) => tableRows(table, schema, file).slice(start - 1, start - 1 + size), count: (table, schema, file) => storedTable(file, table).rows.length, insert: insertRow, update: updateRow, select: table => { throw new Error(`a filter of the table ${table} is an SQL query: it runs natively (warp serve)`); } },
		net: { post: (url, body, headers) => postSync(url, contentText(body), headers ?? {}) },
		// `clipboard.write(text)` (lowering/system_values.rs): a page writes it (markup.js copyText), a Worker has no
		// clipboard and hands the text to its page (self.writeClipboard: worker.js)
		clipboard: { write: text => { (self.writeClipboard ?? copyText)(contentText(text)); return null; } },
	},
});
