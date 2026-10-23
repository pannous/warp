#!/usr/bin/env bash
# zlib in pure warp over plain C calls (lib/native/z.warp, topic ffi-libraries-roadmap): checksums, compress and
# uncompress with bytes results, the z_stream API, gzip, against python's zlib values; also tests/ffi's zlib checks
# (test_ffi_gaps, test_ffi_text_results) on this lane. Run it through the queue: tests/queue.sh tests/host/test_ffi_z.sh
LIBRARY=z
PRELUDE=('use z')
source "$(dirname "$0")/ffi_library_test.sh"
has_library z || { echo "- z: no libz here, skipped"; exit 0; }
# 300 bytes, a third of them NUL; 100000 and 20000 bytes i % 251 (a byte list result past ~40000 runs out of GC heap
# yet, so the round trips are 20000)
middle=('b = []' 'i = 0' 'while i < 20000 { b += [i % 251]; i += 1 }')
big=('b = []' 'i = 0' 'while i < 100000 { b += [i % 251]; i += 1 }')
small=('d = []' 'i = 0' 'while i < 50 { d += [0, 1, 0, 2, 0, 255]; i += 1 }')
# 1 MiB i % 251, and a byte list result of that size (c_bytes, uncompress, inflate) built in one call (new_bytes)
huge=('b = []' 'i = 0' 'while i < 1048576 { b += [i % 251]; i += 1 }')
# python: zlib.compress("hello, warp\0and zlib".encode())
known='[120, 156, 203, 72, 205, 201, 201, 215, 81, 40, 79, 44, 42, 96, 72, 204, 75, 81, 168, 202, 201, 76, 2, 0, 74, 250, 7, 31]'

# tests/ffi/test_ffi_gaps and test_ffi_text_results
program bound 'compressBound(100)'
PRELUDE=()
program bound_zlib 'use zlib' 'compressBound(100)'
program bound_import 'import compressBound from "z"' 'print compressBound(1000)'
PRELUDE=('use z')
program adler_text 'print adler32(1, "abc", 3)'
program version 'print(if count(zlibVersion()) > 2 then "versioned" else "no version")'
# checksums of bytes with NULs
program crc_nul 'print crc32_of([97, 0, 98])'
program crc_text 'print crc32_of("abc".bytes)'
program adler_nul 'print adler32_of([97, 0, 98])'
# compress and uncompress: one call each, bytes results
program round_trip "${small[@]}" 'c = compressed(d)' 'print #c' 'print crc32_of(c)' 'print(if uncompressed(c, #d) == d then "same" else "differs")'
program level "${small[@]}" 'print crc32_of(compressed_at(d, 9))'
program large "${big[@]}" 'c = compressed(b)' 'print #c' 'print crc32_of(c)'
program middle "${middle[@]}" 'c = compressed(b)' 'print #c' 'print crc32_of(c)' 'u = uncompressed(c, #b)' 'print #u' 'print crc32_of(u)'
program known_input "print uncompressed($known, 100)" 'print crc32_of(uncompressed('"$known"', 100))'
program roomy "${small[@]}" 'u = uncompressed(compressed(d), #d * 4)' 'print #u' 'print(if u == d then "same" else "differs")'
program huge "${huge[@]}" 'c = compressed(b)' 'print #c' 'print crc32_of(c)' 'u = uncompressed(c, #b)' 'print #u' 'print crc32_of(u)'
program huge_stream "${huge[@]}" 'u = inflated(deflated(b, Z_DEFAULT_COMPRESSION))' 'print #u' 'print crc32_of(u)'
program huge_c_bytes 'zeros = c_bytes(calloc(1, 1048576), 1048576)' 'print #zeros' 'print crc32_of(zeros)'
# the stream API
program stream_round_trip "${middle[@]}" 'c = deflated(b, Z_DEFAULT_COMPRESSION)' 'print crc32_of(c)' 'u = inflated(c)' 'print #u' 'print crc32_of(u)'
program stream_known "print inflated($known)" 'print crc32_of(inflated('"$known"'))'
program gzip "${small[@]}" 'g = gzipped(d)' 'print g#1' 'print g#2' 'print(if gunzipped(g) == d then "same" else "differs")'
program bad_stream "${small[@]}" 'v = try inflated([1, 2, 3]) catch e { "recovered" }' 'print v' 'print(if inflated(deflated(d, 9)) == d then "inflates again" else "broken")'
program truncated "${small[@]}" 'c = deflated(d, 9)' 'v = try inflated(c[0..#c - 4]) catch e { "recovered" }' 'print v' 'print(if inflated(c) == d then "whole" else "broken")'

check bound "use z: compressBound" 113 ! "Error"
check bound_zlib "use zlib: compressBound" 113 ! "Error"
check bound_import "import compressBound from \"z\"" 0 "1013"
check adler_text "adler32 of a text with its size" 0 "38600999"
check version "zlibVersion is a text" 0 "versioned"
check crc_nul "crc32 of a, NUL, b" 0 "367556721"
check crc_text "crc32 of a text's bytes" 0 "891568578"
check adler_nul "adler32 of a, NUL, b" 0 "25690308"
check round_trip "compress and uncompress 300 bytes with NULs: python's bytes" 0 "19" "3156985466" "same" ! "differs"
check level "compress2 at level 9: python's bytes" 0 "148507295"
check large "100000 bytes compressed to python's 713" 0 "713" "97567627" ! "Error"
check middle "20000 bytes compressed to python's 419 and back" 0 "419" "240245511" "20000" "908052199" ! "Error"
check known_input "uncompress of python's zlib.compress" 0 "[104 101 108 108 111 44 32 119 97 114 112 0 97 110 100" "2716549724"
check roomy "an output buffer larger than needed: the result is the bytes written" 0 "300" "same" ! "Error"
check stream_round_trip "deflate and inflate streams of 20000 bytes" 0 "240245511" "20000" "908052199" ! "Error"
check stream_known "inflate of python's zlib.compress, its size unknown" 0 "[104 101 108 108 111" "2716549724"
check gzip "gzip through a stream: its magic bytes, and back" 0 "31" "139" "same" ! "differs"
check bad_stream "a try around inflating no zlib data recovers, the next stream inflates" 0 "recovered" "inflates again" ! "Error"
check truncated "a try around inflating a truncated stream recovers, the whole one inflates" 0 "recovered" "whole" ! "Error"
check huge "1 MiB compressed to python's 4390 bytes and back" 0 "4390" "3128219806" "1048576" "4010696788" ! "Error"
check huge_stream "1 MiB deflated and inflated through a stream" 0 "1048576" "4010696788" ! "Error"
check huge_c_bytes "c_bytes of 1 MiB" 0 "1048576" "2805525020" ! "Error"
finish
