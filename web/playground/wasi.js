// A small WASI preview1 for the test binary in the browser (test-worker.js): arguments, clocks, randomness, stdout and
// stderr to a callback, and a file system whose "." is the repository root, read with synchronous GETs (FILE_ROOT;
// directories through the static server's listing page (?listing: test_in_browser.py lists a directory even when it
// holds an index.html), cached per worker) and whose /include holds the C headers warp's FFI reads (WARP_INCLUDE), with an empty /tmp. Writes land in an in-memory overlay that lives as
// long as the instance (one test), so a test can write a scratch file and read it back. Unknown calls: ENOSYS.

const ERRNO = { SUCCESS: 0, BADF: 8, INVAL: 28, NOENT: 44, NOSYS: 52, NOTDIR: 54 };
const FILETYPE = { CHARACTER_DEVICE: 2, DIRECTORY: 3, REGULAR_FILE: 4 };
const OPEN = { CREATE: 1, DIRECTORY: 2, EXCLUSIVE: 4, TRUNCATE: 8 };
const FD_FLAG_APPEND = 1;
const ROOT_FD = 3; // ".", the served repository; wasi-libc also resolves relative paths ("/" + path) here
const INCLUDE_FD = 4; // "/include": the C headers warp reads for FFI signatures, each served by its name (test_in_browser.py)
const INCLUDE_DIRECTORY = "/include";
const TEMPORARY_FD = 5; // "/tmp": a directory of the overlay only, for scratch files (tests/common temp_dir)
const TEMPORARY_DIRECTORY = "/tmp";
const PREOPENS = new Map([[ROOT_FD, "."], [INCLUDE_FD, INCLUDE_DIRECTORY], [TEMPORARY_FD, TEMPORARY_DIRECTORY]]);
const INCLUDE_URL = new URL("/__include__/", self.location.href).href;
// WARP_INCLUDE replaces the compiler's include directories (src/ffi_parser.rs), so it looks in exactly one place;
// WARP_HTTP_STUB is test_in_browser.py's stub answering any status and body (tests/common serve); TMPDIR the scratch
const ENVIRONMENT = [`WARP_INCLUDE=${INCLUDE_DIRECTORY}`, `WARP_HTTP_STUB=${new URL("/__stub__", self.location.href).href}`,
	`TMPDIR=${TEMPORARY_DIRECTORY}`];
const ALL_RIGHTS = 0xffffffffffffffffn;
const RANDOM_CHUNK = 65536; // crypto.getRandomValues limit per call
const DIRENT_HEADER = 24;
const LISTING_LINK = /<a href="([^"?#]+)">/g; // python -m http.server's directory page

class WasiExit extends Error {
	constructor(code) {
		super(`exit ${code}`);
		this.code = code;
	}
}

// "./a/../b/" → "b"
function normalPath(path) {
	const parts = [];
	for (const part of path.split("/")) {
		if (part === "" || part === ".") continue;
		if (part === "..") parts.pop();
		else parts.push(part);
	}
	return parts.join("/");
}

const listings = new Map(); // directory → Map(name → is a directory) on the server, null: none; for every test of this worker

function requestSync(method, url) {
	try {
		const request = new XMLHttpRequest();
		request.open(method, url, false);
		request.overrideMimeType("text/plain; charset=x-user-defined");
		request.send();
		return request.status === 200 ? request : null;
	} catch {
		return null;
	}
}

const bytesOf = request => Uint8Array.from(request.responseText, character => character.charCodeAt(0) & 0xff);

// the entries of a served directory, from the listing page (a name ending in "/" is a directory); a missing parent is
// known without asking the server
function listing(directory) {
	if (listings.has(directory)) return listings.get(directory);
	const parent = directory.includes("/") ? directory.slice(0, directory.lastIndexOf("/")) : "";
	const exists = directory === "" || listing(parent)?.get(directory.slice(directory.lastIndexOf("/") + 1));
	const request = exists ? requestSync("GET", `${FILE_ROOT}${directory}${directory ? "/" : ""}?listing`) : null;
	const entries = request && new Map([...request.responseText.matchAll(LISTING_LINK)].map(match => {
		const name = decodeURIComponent(match[1]);
		return [name.replace(/\/$/, ""), name.endsWith("/")];
	}));
	listings.set(directory, entries);
	return entries;
}

const headers = new Map(); // header URL → its lazy file or null, for every test of this worker

// a served file, read only when its bytes are wanted: a directory walk stats every file of the repository
function lazyFile(url) {
	let bytes;
	return {
		get bytes() { return bytes ??= bytesOf(requestSync("GET", url) ?? { responseText: "" }); },
		get size() { return bytes?.length ?? Number(requestSync("HEAD", url)?.getResponseHeader("Content-Length") ?? 0); },
	};
}

// what the server has at a path: {bytes, size} for a file, {entries} for a directory, null for nothing.
// "/include/…" are C headers (INCLUDE_URL), the other paths the repository's, asked only when their directory lists them
function served(path) {
	if (path.startsWith(`${INCLUDE_DIRECTORY}/`)) {
		const url = INCLUDE_URL + path.slice(INCLUDE_DIRECTORY.length + 1);
		if (!headers.has(url)) headers.set(url, requestSync("HEAD", url) ? lazyFile(url) : null);
		return headers.get(url);
	}
	if (path === TEMPORARY_DIRECTORY) return { entries: new Set() };
	if (path.startsWith("/")) return null; // the machine has nothing else; the overlay may
	if (path === "") return { entries: new Set(listing("").keys()) };
	const directory = path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
	const isDirectory = listing(directory)?.get(path.slice(path.lastIndexOf("/") + 1));
	if (isDirectory === undefined) return null;
	return isDirectory ? { entries: new Set(listing(path).keys()) } : lazyFile(FILE_ROOT + path);
}

// args: the program's argument list (argv[0] first); output(text, fd) receives stdout (1) and stderr (2)
function wasiImports(memory, args, output) {
	const written = new Map(); // path → bytes of files written in this instance
	const made = new Set(); // directories made in this instance
	const removed = new Set(); // paths deleted in this instance
	const descriptors = new Map(); // fd → {path, bytes, position, append} of a file or {path, entries} of a directory
	let nextFd = ROOT_FD + PREOPENS.size;
	const view = () => new DataView(memory().buffer);
	const bytesAt = (pointer, length) => new Uint8Array(memory().buffer, pointer, length);
	const textAt = (pointer, length) => utf8Decoder.decode(bytesAt(pointer, length).slice());
	const parentOf = path => path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
	const nameOf = path => path.slice(path.lastIndexOf("/") + 1);

	// what is at a path, the overlay first
	function lookup(path) {
		if (removed.has(path)) return null;
		if (written.has(path)) return { bytes: written.get(path) };
		const children = path => [...written.keys(), ...made].filter(child => parentOf(child) === path && !removed.has(child)).map(nameOf);
		if (made.has(path)) return { entries: new Set(children(path)) };
		const found = served(path);
		if (found?.entries) children(path).forEach(name => found.entries.add(name));
		return found;
	}
	// a path as the overlay and served() key it: relative for the repository, "/…" for the machine
	const resolve = (dirFd, pointer, length) => {
		const base = dirFd === ROOT_FD ? "" : PREOPENS.get(dirFd) ?? descriptors.get(dirFd)?.path;
		if (base === undefined) return undefined;
		const path = normalPath(`${base}/${textAt(pointer, length)}`);
		return base.startsWith("/") ? `/${path}` : path;
	};
	const open = entry => {
		descriptors.set(nextFd, entry);
		return nextFd++;
	};

	// write a list of texts as C strings: pointers into `pointers`, the bytes into `buffer`
	const writeStrings = (texts, pointers, buffer) => {
		for (const text of texts) {
			view().setUint32(pointers, buffer, true);
			pointers += 4;
			const bytes = utf8.encode(text + "\0");
			bytesAt(buffer, bytes.length).set(bytes);
			buffer += bytes.length;
		}
		return ERRNO.SUCCESS;
	};
	const writeSizes = (texts, count, size) => {
		view().setUint32(count, texts.length, true);
		view().setUint32(size, texts.reduce((total, text) => total + utf8.encode(text).length + 1, 0), true);
		return ERRNO.SUCCESS;
	};
	const writeFilestat = (pointer, entry) => {
		bytesAt(pointer, 64).fill(0);
		view().setUint8(pointer + 16, entry.entries ? FILETYPE.DIRECTORY : FILETYPE.REGULAR_FILE);
		view().setBigUint64(pointer + 24, 1n, true);
		view().setBigUint64(pointer + 32, BigInt(entry.size ?? entry.bytes?.length ?? 0), true);
		return ERRNO.SUCCESS;
	};
	const vectors = (pointer, count) => Array.from({ length: count }, (_, index) =>
		[view().getUint32(pointer + 8 * index, true), view().getUint32(pointer + 8 * index + 4, true)]);

	const known = {
		args_get: (pointers, buffer) => writeStrings(args, pointers, buffer),
		args_sizes_get: (count, size) => writeSizes(args, count, size),
		environ_get: (pointers, buffer) => writeStrings(ENVIRONMENT, pointers, buffer),
		environ_sizes_get: (count, size) => writeSizes(ENVIRONMENT, count, size),
		clock_time_get: (_id, _precision, time) => {
			view().setBigUint64(time, BigInt(Math.round((performance.timeOrigin + performance.now()) * 1e6)), true);
			return ERRNO.SUCCESS;
		},
		random_get: (buffer, length) => {
			for (let offset = 0; offset < length; offset += RANDOM_CHUNK) crypto.getRandomValues(bytesAt(buffer + offset, Math.min(RANDOM_CHUNK, length - offset)));
			return ERRNO.SUCCESS;
		},
		proc_exit: code => { throw new WasiExit(code); },
		sched_yield: () => ERRNO.SUCCESS,
		fd_write: (fd, iovs, count, writtenCount) => {
			const file = descriptors.get(fd);
			if (fd !== 1 && fd !== 2 && !file?.bytes) return ERRNO.BADF;
			let total = 0;
			for (const [pointer, length] of vectors(iovs, count)) {
				const chunk = bytesAt(pointer, length).slice();
				if (file) {
					const start = file.append ? file.bytes.length : file.position;
					const grown = new Uint8Array(Math.max(file.bytes.length, start + length));
					grown.set(file.bytes);
					grown.set(chunk, start);
					file.bytes = grown;
					file.position = start + length;
					written.set(file.path, file.bytes);
				} else output(utf8Decoder.decode(chunk), fd);
				total += length;
			}
			view().setUint32(writtenCount, total, true);
			return ERRNO.SUCCESS;
		},
		fd_read: (fd, iovs, count, read) => {
			const file = descriptors.get(fd);
			let total = 0;
			if (file?.bytes) {
				for (const [pointer, length] of vectors(iovs, count)) {
					const chunk = file.bytes.subarray(file.position, file.position + length);
					bytesAt(pointer, chunk.length).set(chunk);
					file.position += chunk.length;
					total += chunk.length;
				}
			} else if (fd !== 0) return ERRNO.BADF; // stdin is empty
			view().setUint32(read, total, true);
			return ERRNO.SUCCESS;
		},
		fd_seek: (fd, offset, whence, position) => {
			const file = descriptors.get(fd);
			if (!file?.bytes) return ERRNO.BADF;
			file.position = Math.max(0, [0, file.position, file.bytes.length][whence] + Number(offset));
			view().setBigUint64(position, BigInt(file.position), true);
			return ERRNO.SUCCESS;
		},
		fd_close: fd => descriptors.delete(fd) ? ERRNO.SUCCESS : ERRNO.BADF,
		fd_sync: () => ERRNO.SUCCESS,
		fd_datasync: () => ERRNO.SUCCESS,
		fd_prestat_get: (fd, prestat) => {
			if (!PREOPENS.has(fd)) return ERRNO.BADF;
			view().setUint8(prestat, 0);
			view().setUint32(prestat + 4, PREOPENS.get(fd).length, true);
			return ERRNO.SUCCESS;
		},
		fd_prestat_dir_name: (fd, pointer, length) => {
			if (!PREOPENS.has(fd)) return ERRNO.BADF;
			bytesAt(pointer, length).set(utf8.encode(PREOPENS.get(fd)).subarray(0, length));
			return ERRNO.SUCCESS;
		},
		fd_fdstat_get: (fd, stat) => {
			const entry = descriptors.get(fd);
			const filetype = fd <= 2 ? FILETYPE.CHARACTER_DEVICE : PREOPENS.has(fd) || entry?.entries ? FILETYPE.DIRECTORY : entry ? FILETYPE.REGULAR_FILE : 0;
			if (!filetype) return ERRNO.BADF;
			bytesAt(stat, 24).fill(0);
			view().setUint8(stat, filetype);
			view().setBigUint64(stat + 8, ALL_RIGHTS, true);
			view().setBigUint64(stat + 16, ALL_RIGHTS, true);
			return ERRNO.SUCCESS;
		},
		fd_fdstat_set_flags: () => ERRNO.SUCCESS,
		fd_filestat_get: (fd, stat) => {
			const entry = PREOPENS.has(fd) ? { entries: new Set() } : descriptors.get(fd);
			return entry ? writeFilestat(stat, entry) : ERRNO.BADF;
		},
		fd_filestat_set_size: (fd, size) => {
			const file = descriptors.get(fd);
			if (!file?.bytes) return ERRNO.BADF;
			const resized = new Uint8Array(Number(size));
			resized.set(file.bytes.subarray(0, resized.length));
			written.set(file.path, file.bytes = resized);
			return ERRNO.SUCCESS;
		},
		fd_readdir: (fd, buffer, length, cookie, used) => {
			const directory = fd === ROOT_FD ? { entries: lookup("").entries } : descriptors.get(fd);
			if (!directory?.entries) return ERRNO.NOTDIR;
			const names = [...directory.entries].sort();
			let offset = 0;
			for (let index = Number(cookie); index < names.length && offset < length; index++) {
				const name = utf8.encode(names[index]);
				const child = lookup(normalPath(`${directory.path ?? ""}/${names[index]}`));
				const entry = new Uint8Array(DIRENT_HEADER + name.length);
				const header = new DataView(entry.buffer);
				header.setBigUint64(0, BigInt(index + 1), true);
				header.setBigUint64(8, BigInt(index + 1), true);
				header.setUint32(16, name.length, true);
				header.setUint8(20, child?.entries ? FILETYPE.DIRECTORY : FILETYPE.REGULAR_FILE);
				entry.set(name, DIRENT_HEADER);
				const fitting = Math.min(entry.length, length - offset); // a cut entry tells the reader to ask with a larger buffer
				bytesAt(buffer + offset, fitting).set(entry.subarray(0, fitting));
				offset += fitting;
			}
			view().setUint32(used, offset, true);
			return ERRNO.SUCCESS;
		},
		path_open: (dirFd, _lookupFlags, pointer, length, openFlags, _rights, _inheriting, fdFlags, opened) => {
			const path = resolve(dirFd, pointer, length);
			if (path === undefined) return ERRNO.BADF;
			const existing = lookup(path);
			if (openFlags & OPEN.DIRECTORY) {
				if (!existing?.entries) return existing ? ERRNO.NOTDIR : ERRNO.NOENT;
				view().setUint32(opened, open({ path, entries: existing.entries }), true);
				return ERRNO.SUCCESS;
			}
			if (!existing && !(openFlags & OPEN.CREATE)) return ERRNO.NOENT;
			if (existing?.entries) {
				view().setUint32(opened, open({ path, entries: existing.entries }), true);
				return ERRNO.SUCCESS;
			}
			const bytes = openFlags & OPEN.TRUNCATE || !existing ? new Uint8Array() : existing.bytes;
			if (openFlags & (OPEN.CREATE | OPEN.TRUNCATE)) {
				written.set(path, bytes);
				removed.delete(path);
			}
			view().setUint32(opened, open({ path, bytes, position: 0, append: !!(fdFlags & FD_FLAG_APPEND) }), true);
			return ERRNO.SUCCESS;
		},
		path_filestat_get: (dirFd, _flags, pointer, length, stat) => {
			const path = resolve(dirFd, pointer, length);
			if (path === undefined) return ERRNO.BADF;
			const entry = lookup(path);
			return entry ? writeFilestat(stat, entry) : ERRNO.NOENT;
		},
		path_create_directory: (dirFd, pointer, length) => {
			const path = resolve(dirFd, pointer, length);
			made.add(path);
			removed.delete(path);
			return ERRNO.SUCCESS;
		},
		path_unlink_file: (dirFd, pointer, length) => {
			const path = resolve(dirFd, pointer, length);
			if (!lookup(path)) return ERRNO.NOENT;
			written.delete(path);
			removed.add(path);
			return ERRNO.SUCCESS;
		},
		path_remove_directory: (dirFd, pointer, length) => {
			const path = resolve(dirFd, pointer, length);
			made.delete(path);
			removed.add(path);
			return ERRNO.SUCCESS;
		},
		path_rename: (fromFd, fromPointer, fromLength, toFd, toPointer, toLength) => {
			const from = resolve(fromFd, fromPointer, fromLength), to = resolve(toFd, toPointer, toLength);
			const entry = lookup(from);
			if (!entry) return ERRNO.NOENT;
			if (entry.bytes) written.set(to, entry.bytes);
			else made.add(to);
			written.delete(from);
			made.delete(from);
			removed.add(from);
			removed.delete(to);
			return ERRNO.SUCCESS;
		},
		path_readlink: () => ERRNO.INVAL,
	};
	return new Proxy(known, { get: (functions, name) => functions[name] ?? (() => ERRNO.NOSYS) });
}
