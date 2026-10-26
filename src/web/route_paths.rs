//! A route's path pattern matched against a request path, for a server choosing its route before any warp code runs
//! (moved from lowering/web/routes.rs, card rust-bootstrap; lib/router.warp route_matches is the warp side)

const PATH_SEPARATOR: char = '/';
/// `:id`, `:id:int`: a path parameter, and its type
pub const PARAMETER_MARK: &str = ":";
/// The marks on a pattern's last part: optional, any parts, one part or more
const PART_MARKS: [char; 3] = ['?', '*', '+'];
const GROUP_START: char = '(';
const ANY_PARTS: &str = "*";

/// A parameter of a pattern part: ":id:int" → id, its type int; ":id(\\d+)?" → id, its regular expression (a URLPattern
/// mark or group leaves it untyped, as lib/router.warp route_part_type)
struct Parameter<'a> {
	kind: Option<&'a str>,
}

fn parameters(pattern: &str) -> impl Iterator<Item = Parameter<'_>> {
	pattern.split(PATH_SEPARATOR).filter_map(|segment| segment.strip_prefix(PARAMETER_MARK)).map(|declared| {
		let (declared, marked) = declared.strip_suffix(PART_MARKS).map_or((declared, false), |rest| (rest, true));
		let (declared, group) = match declared.split_once(GROUP_START) {
			Some((head, group)) => (head, group.strip_suffix(')')),
			None => (declared, None),
		};
		let (name, kind) =
			declared.split_once(PARAMETER_MARK).map_or((declared, None), |(name, kind)| (name, Some(kind)));
		{
			let _ = name;
			Parameter { kind: kind.filter(|_| !marked && group.is_none()) }
		}
	})
}

/// Whether `path` matches `pattern`, as lib/router.warp route_matches, for a server choosing its route before any warp
/// code runs: fixed parts equal, a typed parameter's part of its type, a mark on the last part, "*" any path
pub fn path_matches(pattern: &str, path: &str) -> bool {
	if pattern == ANY_PARTS {
		return true;
	}
	let parts = |text: &'_ str| {
		text.split(PATH_SEPARATOR).filter(|part| !part.is_empty()).map(str::to_string).collect::<Vec<String>>()
	};
	let (wanted, given) = (parts(pattern), parts(path));
	let mark = wanted.last().and_then(|last| {
		if last == ANY_PARTS {
			Some('*')
		} else {
			last.strip_prefix(PARAMETER_MARK)?.chars().last().filter(|end| PART_MARKS.contains(end))
		}
	});
	let fixed = wanted.len() - usize::from(mark.is_some());
	let counted = match mark {
		None => given.len() == fixed,
		Some('?') => (fixed..=fixed + 1).contains(&given.len()),
		Some('+') => given.len() > fixed,
		Some(_) => given.len() >= fixed,
	};
	counted
		&& wanted.iter().zip(&given).take(fixed).all(|(part, segment)| match part.strip_prefix(PARAMETER_MARK) {
			None => part == segment,
			Some(_) => {
				parameters(part).next().and_then(|parameter| parameter.kind).is_none_or(|kind| part_fits(kind, segment))
			}
		})
}

/// Whether a path part is a value of the type a parameter declares (lib/router.warp route_fits): digits, a float's
/// with at most one dot
fn part_fits(kind: &str, segment: &str) -> bool {
	let dots = match kind {
		"int" => 0,
		"float" => 1,
		_ => return true,
	};
	!segment.is_empty()
		&& segment.chars().all(|character| character.is_ascii_digit() || character == '.')
		&& segment.matches('.').count() <= dots
}

