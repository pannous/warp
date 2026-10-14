//! The files of a site (`warp build --site`, src/web/site.rs in warp) and how a request path finds one, shared by warp's
//! servers and web view and by an app `warp build` made: the stub carrying a site packed (card build-links)
use std::path::Path;

pub const PAGE_FILE: &str = "index.html";
/// The page of a program with routes for any other path (card single-page): a static host such as GitHub Pages serves
/// 404.html for a path it has no file of, `warp dev` and `serve` do the same; the page's router shows that path's route
pub const ROUTED_PAGE_FILE: &str = "404.html";
const INDEX_PATH: &str = "/";
/// The content types of a site's files, by their extension
const CONTENT_TYPES: [(&str, &str); 3] =
	[("html", "text/html; charset=utf-8"), ("js", "text/javascript; charset=utf-8"), ("wasm", "application/wasm")];
const OTHER_CONTENT: &str = "application/octet-stream";
/// What a packed site starts with, which machine code never does
pub const PACKED_SITE: &[u8; 8] = b"WRPCsite";

/// A file of a site: its name and content
pub type SiteFile = (String, Vec<u8>);

/// The file of a site a request path names: "/" is the page, "/app.wasm" the module, …, any other path the page of a
/// program with routes (404.html)
pub fn file_at<'a>(files: &'a [SiteFile], path: &str) -> Option<&'a SiteFile> {
	let name = if path == INDEX_PATH { PAGE_FILE } else { path.trim_start_matches('/') };
	let named = |wanted: &str| files.iter().find(|(file, _)| file == wanted);
	named(name).or_else(|| named(ROUTED_PAGE_FILE))
}

/// The content type of a site's file, by its extension
pub fn content_type(name: &str) -> &'static str {
	let extension = Path::new(name).extension().and_then(|extension| extension.to_str()).unwrap_or_default();
	CONTENT_TYPES.iter().find(|(known, _)| *known == extension).map_or(OTHER_CONTENT, |(_, content_type)| content_type)
}

/// A path's file and its content type, as the web view answers it
pub fn served(files: &[SiteFile], path: &str) -> Option<(Vec<u8>, &'static str)> {
	file_at(files, path).map(|(name, bytes)| (bytes.clone(), content_type(name)))
}

/// The site as one run of bytes: PACKED_SITE, then each file's name and content, each after its length (u64 LE)
pub fn packed(files: &[SiteFile]) -> Vec<u8> {
	let mut bytes = PACKED_SITE.to_vec();
	for part in files.iter().flat_map(|(name, content)| [name.as_bytes(), content.as_slice()]) {
		bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
		bytes.extend_from_slice(part);
	}
	bytes
}

/// The site `packed` made, None for any other bytes
pub fn unpacked(bytes: &[u8]) -> Option<Vec<SiteFile>> {
	let mut rest = bytes.strip_prefix(PACKED_SITE.as_slice())?;
	let mut next = || -> Option<Vec<u8>> {
		let (length, after) = rest.split_first_chunk::<8>()?;
		let (part, after) = after.split_at_checked(u64::from_le_bytes(*length) as usize)?;
		rest = after;
		Some(part.to_vec())
	};
	let mut files = vec![];
	while let Some(name) = next() {
		files.push((String::from_utf8(name).ok()?, next()?));
	}
	Some(files)
}
