#!/usr/bin/env python3
"""Generate test/modules/test_encoding_more.pairs.warp and test_idna.pairs.warp, every expected value checked against
python3 (base64, the punycode and idna codecs) first. Rerun from the repository root with RFC 3492's text at hand:
    curl -sSfO https://www.rfc-editor.org/rfc/rfc3492.txt && python3 test/encoding/gen_encoding_tests.py rfc3492.txt"""
import sys, os, re, base64
root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
lit = lambda s: "'" + s.replace("\\", "\\\\").replace("'", "\\'") + "'"
W = lambda t: '"' + t.replace("\\", "\\\\").replace('"', '\\"') + '"'
files = {}
def case(f, code, exp): files.setdefault(f, []).append(lit(code) + "\t" + exp)
def note(f, t): files.setdefault(f, []).append("// " + t)
E = "test/modules/test_encoding_more.pairs.warp"
note(E, "lib/encoding.warp base32, base32hex, base58, ascii85 and z85 on spec vectors, each checked against python3 first (card lib-encodings-more)")
note(E, "RFC 4648 section 10, base32 and base32hex")
for t in ["", "f", "fo", "foo", "foob", "fooba", "foobar"]:
    b32 = base64.b32encode(t.encode()).decode(); h32 = base64.b32hexencode(t.encode()).decode()
    case(E, "use encoding; base32_encode(%s)" % W(t), W(b32))
    case(E, "use encoding; base32hex_encode(%s)" % W(t), W(h32))
    case(E, "use encoding; [base32_decode(%s), base32hex_decode(%s)]" % (W(b32), W(h32)), "[%s, %s]" % (W(t), W(t)))
note(E, "base32 decoding takes lowercase and a missing padding, and refuses other characters")
case(E, "use encoding; base32_decode(\"mzxw6ytboi\")", W("foobar"))
case(E, "use encoding; base32_decode_bytes(\"74======\")", str(list(base64.b32decode("74======"))))
case(E, "use encoding; try { base32_decode(\"MZ1W6===\") } catch e { e.message }", W("base32: 1 at 3 is no base32 digit"))
case(E, "use encoding; try { base32hex_decode(\"CPNW\") } catch e { e.message }", W("base32hex: W at 4 is no base32hex digit"))
case(E, "use encoding; base32_encode([0, 255, 128])", W(base64.b32encode(bytes([0, 255, 128])).decode()))
note(E, "base58, the Bitcoin alphabet: leading zero bytes are leading 1s")
B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
def b58(b):
    n = int.from_bytes(b, "big"); s = ""
    while n: n, r = divmod(n, 58); s = B58[r] + s
    return "1" * (len(b) - len(b.lstrip(b"\0"))) + s
assert b58(b"Hello World!") == "2NEpo7TZRRrLZSi2U"
for b in [b"Hello World!", b"The quick brown fox jumps over the lazy dog", bytes([0, 0, 1]), bytes([0]), bytes([0, 0, 0, 0]), b"", bytes([0, 0, 0x28, 0x7f, 0xb4, 0xcd]), bytes(range(250, 256)) * 6]:
    case(E, "use encoding; base58_encode(%s)" % (W(b.decode()) if b.isascii() and b and b[0] else str(list(b))), W(b58(b)))
    case(E, "use encoding; base58_decode_bytes(%s)" % W(b58(b)), str(list(b)))
case(E, "use encoding; base58_decode(\"2NEpo7TZRRrLZSi2U\")", W("Hello World!"))
case(E, "use encoding; try { base58_decode(\"abc0\") } catch e { e.message }", W("base58: 0 at 4 is no base58 digit"))
note(E, "ascii85: Man and Wikipedia's Leviathan sample, z for a zero group, short final groups, <~ ~> accepted on decoding")
lev = "Man is distinguished, not only by his reason, but by this singular passion from other animals, which is a lust of the mind, that by a perseverance of delight in the continued and indefatigable generation of knowledge, exceeds the short vehemence of any carnal pleasure."
for b in [b"Man ", b"Man", b"Ma", b"M", lev.encode(), bytes(4), bytes(8) + b"x", b"", bytes([255, 255, 255, 255])]:
    a = base64.a85encode(b).decode()
    case(E, "use encoding; ascii85_encode(%s)" % (W(b.decode()) if b.isascii() and b"\0" not in b else str(list(b))), W(a))
    case(E, "use encoding; ascii85_decode_bytes(%s)" % W("<~" + a + "~>"), str(list(b)))
assert base64.a85encode(b"Man ").decode() == "9jqo^"
case(E, "use encoding; ascii85_decode(\"9jqo^ BlbD-\")", W(base64.a85decode("9jqo^ BlbD-").decode()))
case(E, "use encoding; try { ascii85_decode(\"9jqo^v\") } catch e { e.message }", W("ascii85: v at 6 is no ascii85 digit"))
note(E, "z85, ZeroMQ RFC 32: its vector, and lengths that are no multiple of 4 (bytes) or 5 (text) refused")
case(E, "use encoding; z85_encode([134, 79, 210, 111, 181, 89, 247, 91])", W("HelloWorld"))
case(E, "use encoding; z85_decode_bytes(\"HelloWorld\")", "[134, 79, 210, 111, 181, 89, 247, 91]")
Z85 = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#"
def z85(b):
    out = ""
    for i in range(0, len(b), 4):
        n = int.from_bytes(b[i:i + 4], "big"); g = ""
        for _ in range(5): n, r = divmod(n, 85); g = Z85[r] + g
        out += g
    return out
assert z85(bytes([0x86, 0x4F, 0xD2, 0x6F, 0xB5, 0x59, 0xF7, 0x5B])) == "HelloWorld"
for b in [bytes(range(32)), bytes([255] * 8), b"abcd"]:
    case(E, "use encoding; z85_encode(%s)" % str(list(b)), W(z85(b)))
    case(E, "use encoding; z85_decode_bytes(%s)" % W(z85(b)), str(list(b)))
case(E, "use encoding; try { z85_encode(\"abc\") } catch e { e.message }", W("z85: 3 bytes are no multiple of 4"))
case(E, "use encoding; try { z85_decode(\"Hello\" + \"W\") } catch e { e.message }", W("z85: 6 characters are no multiple of 5"))
note(E, "the older words of use encoding still work")
case(E, "use encoding; [base64_encode(\"foo\"), base64_decode(\"Zm9v\"), hex_encode(\"foo\"), hex_decode(\"666f6f\")]", '["Zm9v", "foo", "666f6f", "foo"]')
I = "test/modules/test_idna.pairs.warp"
note(I, "lib/idna.warp Punycode: every RFC 3492 section 7.1 sample (A-S), its code points to its ASCII and back (card lib-encodings-more)")
t = open(sys.argv[1] if len(sys.argv) > 1 else "rfc3492.txt").read()
sec = t[t.rindex("7.1 Sample strings"):t.rindex("7.2 Decoding traces")]
samples = []; cur = None; cont = False
for line in sec.split("\n"):
    m = re.match(r"\s+\(([A-S])\) (.*)", line)
    if m: cur = [m.group(1), m.group(2).strip(), [], None]; samples.append(cur); continue
    if cur is None: continue
    if cont: cur[3] += line.strip().rstrip("\\"); cont = line.rstrip().endswith("\\"); continue
    if "Punycode:" in line:
        cur[3] = line.split("Punycode:")[1].strip(); cont = cur[3].endswith("\\"); cur[3] = cur[3].rstrip("\\"); continue
    if cur[3] is None: cur[2] += [int(x, 16) for x in re.findall(r"[uU]\+([0-9A-Fa-f]{4,6})", line)]
assert [s[0] for s in samples] == list("ABCDEFGHIJKLMNOPQRS"), [s[0] for s in samples]
for key, title, cps, puny in samples:
    py = "".join(map(chr, cps)).encode("punycode").decode()
    assert py == puny or (py == puny.lower() and key == "I"), (key, py, puny)
    note(I, "(%s) %s" % (key, title))
    case(I, "use idna; punycode_encode(%s)" % str(cps), W(py))
    case(I, "use idna; punycode_decode_points(%s)" % W(puny), str(cps))
note(I, "(I) above is written with the RFC's mixed-case annotation D, which decoding reads as d; encoding writes lowercase")
case(I, "use idna; punycode_encode(\"bücher\")", W("bcher-kva"))
case(I, "use idna; punycode_decode(\"bcher-kva\")", W("bücher"))
case(I, "use idna; try { punycode_decode(\"abc-!\") } catch e { e.message }", W("punycode: ! is no punycode digit"))
note(I, "domain_to_ascii and domain_to_unicode: lowercase, NFC, xn-- labels, the IDNA dots; python's idna codec agrees, lowercased (it keeps the case of ASCII labels, UTS #46 lowercases them)")
for d in ["bücher.example", "München.DE", "www.Bücher.de", "例え.テスト", "παράδειγμα.δοκιμή", "ドメイン名例.jp", "abc.example.com", "example.com."]:
    a = d.encode("idna").decode().lower()
    case(I, "use idna; domain_to_ascii(%s)" % W(d), W(a))
    case(I, "use idna; domain_to_unicode(%s)" % W(a), W(a.encode().decode("idna").lower()))
assert "bücher.example".encode("idna") == b"xn--bcher-kva.example"
case(I, "use idna; domain_to_ascii(\"例え。テスト\")", W("例え.テスト".encode("idna").decode()))
case(I, "use idna; domain_to_ascii(\"bücher．example｡com\")", W("xn--bcher-kva.example.com"))
case(I, "use idna; domain_to_ascii(\"u\u0308ber.example\")", W("xn--ber-goa.example"))
note(I, "non-transitional: ß stays ß (python's IDNA 2003 codec would map it to ss)")
case(I, "use idna; domain_to_ascii(\"straße.de\")", W("xn--strae-oqa.de"))
case(I, "use idna; domain_to_unicode(\"XN--BCHER-KVA.Example\")", W("bücher.example"))
case(I, "use idna; try { domain_to_ascii(\"%s.com\") } catch e { e.message }" % ("a" * 64), W("domain_to_ascii: the label " + "a" * 64 + " is longer than 63 characters"))
for f, L in files.items():
    open(os.path.join(root, f), "w", encoding="utf-8").write("\n".join(L) + "\n"); print(f, sum(1 for l in L if l.startswith("'")))
