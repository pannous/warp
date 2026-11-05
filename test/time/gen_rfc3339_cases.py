#!/usr/bin/env python3
# Writes test/time/rfc3339_cases.tsv: group, input, and the milliseconds since 1970 python's datetime gives it, or ERR
# for a text that is no RFC 3339 date-time (or YYYY-MM-DD date). Valid: the strict RFC 3339 grammar (T, t or a space;
# Z, z or ±hh:mm; any fraction, cut to milliseconds) and a real date and time (datetime refuses 24:00, 23:60 and the
# leap second 23:59:60; lib keeps no leap seconds either). Run from the repo root.
import datetime as dt, re, os
UTC = dt.timezone.utc; EPOCH = dt.datetime(1970, 1, 1, tzinfo=UTC)
RFC = re.compile(r"^(\d{4})-(\d{2})-(\d{2})(?:[Tt ](\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?([Zz]|[+-]\d{2}:\d{2}))?$")
# lib's own leniency (test_audit_bugfixes_2): a date's month and day may have one digit, 1970-1-1
LENIENT_DATE = re.compile(r"^\d{4}-\d{1,2}-\d{1,2}$")
def expect(s):
    if LENIENT_DATE.match(s) and not RFC.match(s):
        try: return str((dt.datetime(*map(int, s.split("-")), tzinfo=UTC) - EPOCH) // dt.timedelta(milliseconds=1))
        except ValueError: return "ERR"
    m = RFC.match(s)
    if not m: return "ERR"
    y, mo, d, h, mi, se, frac, zone = m.groups()
    try:
        if h is None: return str((dt.datetime(int(y), int(mo), int(d), tzinfo=UTC) - EPOCH) // dt.timedelta(milliseconds=1))
        if zone in "Zz": tz = UTC
        else:
            oh, om = int(zone[1:3]), int(zone[4:6])
            if oh > 23 or om > 59: return "ERR"
            tz = dt.timezone((-1 if zone[0] == "-" else 1) * dt.timedelta(hours=oh, minutes=om))
        t = dt.datetime(int(y), int(mo), int(d), int(h), int(mi), int(se), tzinfo=tz)
        ms = (t - EPOCH) // dt.timedelta(milliseconds=1) + (int((frac + "00")[:3]) if frac else 0)
        # cross-check with fromisoformat where python reads the form
        try:
            p = dt.datetime.fromisoformat(s.replace("t", "T").replace("z", "Z")[:19] + s[19 + (len(frac) + 1 if frac else 0):].replace("z", "Z"))
            assert (p - EPOCH) // dt.timedelta(milliseconds=1) + (int((frac + "00")[:3]) if frac else 0) == ms
        except ValueError: pass
        return str(ms)
    except ValueError: return "ERR"
cases = []
def add(g, *xs): cases.extend((g, x) for x in xs)
add("offsets", "2026-11-04T14:05:09Z", "2026-11-04T14:05:09+00:00", "2026-11-04T14:05:09-00:00", "2026-11-04T14:05:09+01:00",
    "2026-11-04T14:05:09-05:30", "2026-11-04T14:05:09+14:00", "2026-11-04T14:05:09-12:00", "2026-11-04T14:05:09+23:59",
    "2026-11-04T00:30:00+01:00", "2026-01-01T00:00:00+00:01", "2026-11-04T14:05:09+24:00", "2026-11-04T14:05:09+01:60",
    "2026-11-04T14:05:09+0100", "2026-11-04T14:05:09+01", "2026-11-04T14:05:09", "2026-11-04T14:05:09 Z")
add("fractions", *["2026-11-04T14:05:09." + "123456789"[:n] + "Z" for n in range(1, 10)],
    "2026-11-04T14:05:09.000Z", "2026-11-04T14:05:09.5+02:00", "2026-11-04T14:05:09.999999999-03:00", "2026-11-04T14:05:09.Z",
    "2026-11-04T14:05:09,5Z", "1969-12-31T23:59:59.999Z", "1970-01-01T00:00:00.001Z")
add("leap years", "2000-02-29", "1900-02-29", "2024-02-29", "2026-02-29", "2100-02-29", "2400-02-29", "1600-02-29T12:00:00Z",
    "2000-02-29T23:59:59Z", "1900-02-28T23:59:59Z", "2004-02-29T00:00:00+01:00")
add("month ends", *["2026-%02d-%02d" % (m, d) for m, d in [(1, 31), (1, 32), (4, 30), (4, 31), (6, 31), (9, 30), (9, 31), (11, 31), (12, 31), (12, 32), (2, 28), (2, 30)]],
    "2026-00-10", "2026-13-01", "2026-12-00", "2026-12-31T23:59:59-01:00")
add("times", "2026-11-04T00:00:00Z", "2026-11-04T23:59:59Z", "2026-11-04T24:00:00Z", "2026-11-04T23:60:00Z", "2026-11-04T23:59:60Z",
    "2016-12-31T23:59:60Z", "2026-11-04T25:00:00Z", "2026-11-04T1:05:09Z", "2026-11-04T14:5:09Z", "2026-11-04T14:05Z", "2026-11-04T14:05:9Z")
add("separators", "2026-11-04t14:05:09z", "2026-11-04 14:05:09Z", "2026-11-04t14:05:09.5z", "2026-11-04 14:05:09+01:00",
    "2026-11-04_14:05:09Z", "2026-11-04T14:05:09Z ", " 2026-11-04T14:05:09Z", "2026-11-04  14:05:09Z")
add("week and ordinal dates (not supported)", "2026-W45-3", "2026W453", "2026-309", "2026309", "2026-W45")
add("other ISO 8601 (not RFC 3339)", "2026-1-4 (lenient)".split(" ")[0], "20261104", "20261104T140509Z", "2026-11-04T140509Z", "+2026-11-04", "26-11-04")
add("years", "0001-01-01T00:00:00Z", "1583-01-01", "1969-12-31", "1970-01-01T00:00:00Z", "9999-12-31T23:59:59.999Z")
root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
with open(os.path.join(root, "test/time/rfc3339_cases.tsv"), "w") as f:
    for g, x in cases: f.write("%s\t%s\t%s\n" % (g, x, expect(x)))
print(len(cases), "cases")
