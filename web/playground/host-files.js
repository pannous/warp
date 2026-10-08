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

// a synchronous POST of a text (stdlib net's post, src/extensions/utils.rs post_within), its answer's text
function postSync(url, body) {
	const request = new XMLHttpRequest();
	request.open("POST", url, false);
	request.setRequestHeader("Content-Type", "text/plain; charset=utf-8");
	request.send(body);
	if (request.status >= 400 || request.status === 0) throw new Error(request.status ? `HTTP status ${request.status}` : "network error (blocked by CORS?)");
	return request.responseText;
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

// the tables of `stored users: [User]` in the page's memory, by name: {columns, rows: [[id, column values…]]}; each run
// starts without them
let pageTables = {};
// the run's output (hooks.print; the playground shows no stderr), where a table in memory says so
let printNote = text => console.warn(text);
function pageTable(name, schema = []) {
	if (!pageTables[name]) {
		printNote(`note: the table ${name} lives in this page's memory only (the browser has no database yet; warp serve keeps it in SQLite)\n`);
		pageTables[name] = { columns: schema.map(([column]) => column), rows: [] };
	}
	return pageTables[name];
}

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

addHostPart({
	started: () => { pageTables = {}; },
	words: (holder, hooks, { program, text }) => {
		printNote = note => hooks.print(note, 1);
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
		os: { env: () => null, args: () => [] }, // a page has no environment and no command line
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
		// `stored users: [User]` (lowering/database_tables.rs, src/database.rs natively): the browser has no database yet
		// (notes/orm.md step 6, IndexedDB), so a table lives in the page's memory, said once per table
		table: {
			open: (name, schema) => {
				const table = pageTable(name, schema);
				return table.rows.map(row => [...row]);
			},
			insert: (name, columns, values) => {
				const table = pageTable(name);
				const id = table.rows.length === 0 ? 1 : table.rows[table.rows.length - 1][0] + 1;
				table.rows.push([id, ...table.columns.map(column => values[columns.indexOf(column)] ?? null)]);
				return id;
			},
			update: (name, id, column, value) => {
				const table = pageTable(name);
				const row = table.rows.find(row => row[0] === Number(id));
				if (row) row[1 + table.columns.indexOf(column)] = value;
				return null;
			},
			select: name => { throw new Error(`a filter of the table ${name} is an SQL query: it runs natively (warp serve), the browser has no database yet`); },
		},
		net: { post: (url, body) => postSync(url, contentText(body)) },
		// `clipboard.write(text)` (lowering/system_values.rs): a page writes it (markup.js copyText), a Worker has no
		// clipboard and hands the text to its page (self.writeClipboard: worker.js)
		clipboard: { write: text => { (self.writeClipboard ?? copyText)(contentText(text)); return null; } },
	},
});
