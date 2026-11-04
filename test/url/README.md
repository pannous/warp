# WHATWG URL tests for lib/url (not part of smoke)

`urltestdata.json` is web-platform-tests' url/resources/urltestdata.json at commit
1a5412c7a434195ef7b7fe98b5397f7dffcd3183 (2026-11-04), 229610 bytes; LICENSE.md is wpt's (3-clause BSD).

lib/url is an RFC 3986 parser that keeps a URL as written (url_text(parse_url(t)) is t), not a WHATWG URL parser.
Selection, by the case's input and base before running (skipped, counted by category): a whitespace or control
character in the input (WHATWG strips or encodes it); a backslash (WHATWG reads it as a slash on special schemes);
non-ASCII in the input or base (IDNA and percent-encoding); file: (its quirks); a scheme other than http, https, ws,
wss and ftp (WHATWG's non-special rules); IPv4 number forms other than the dotted quad. The rest run: without a base
url_text(parse_url(input)), with one resolve_url(base, input); href and the parts are compared (protocol = scheme +
":", username = user, hostname = host with [..] for IPv6, pathname = path, search and hash with their ? and #).
A failing case is counted "by design" when one of WHATWG's normalizations applied to lib's own parts gives the
expected ones (default port, empty special path as /, dot segments, encode sets, host lowercasing, IPv6 form, ...),
or when it is a failure case: parse_url does not validate (it gives the parts), so only those that raise pass.

    WARP_LIB=$PWD/lib test/url/url_test.py [--fails "no base"|"with base"|failure|all]
