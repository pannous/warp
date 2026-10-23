#!/usr/bin/env bash
# zlib in pure warp over plain C calls (lib/native/z.warp, topic ffi-libraries-roadmap): checksums, compress and
# uncompress with bytes results, the z_stream API, gzip, against python's zlib values; also tests/ffi's zlib checks
# (test_ffi_gaps, test_ffi_text_results) on this lane. Run it through the queue: tests/queue.sh tests/host/test_ffi_z.sh
LIBRARY=z
PRELUDE=('use z')
source "$(dirname "$0")/ffi_library_test.sh"
has_library z || { echo "- z: no libz here, skipped"; exit 0; }
# 300 bytes, a third of them NUL; 100000 bytes i % 251
big=('b = []' 'i = 0' 'while i < 100000 { b += [i % 251]; i += 1 }')
small=('d = []' 'i = 0' 'while i < 50 { d += [0, 1, 0, 2, 0, 255]; i += 1 }')
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
program crc_text 'print crc32_of("abc")'
program adler_nul 'print adler32_of([97, 0, 98])'
# compress and uncompress: one call each, bytes results
program round_trip "${small[@]}" 'c = compressed(d)' 'print #c' 'print crc32_of(c)' 'print(if uncompressed(c, #d) == d then "same" else "differs")'
program level "${small[@]}" 'print crc32_of(compressed_at(d, 9))'
program large "${big[@]}" 'c = compressed(b)' 'print #c' 'print crc32_of(c)' 'u = uncompressed(c, #b)' 'print #u' 'print crc32_of(u)'
program known_input "print uncompressed($known, 100)" 'print crc32_of(uncompressed('"$known"', 100))'
program too_small "${small[@]}" 'uncompressed(compressed(d), 10)'
# the stream API
program stream_round_trip "${big[@]}" 'c = deflated(b, Z_DEFAULT_COMPRESSION)' 'print crc32_of(c)' 'u = inflated(c)' 'print #u' 'print crc32_of(u)'
program stream_known "print inflated($known)" 'print crc32_of(inflated('"$known"'))'
program gzip "${small[@]}" 'g = gzipped(d)' 'print g#1' 'print g#2' 'print(if gunzipped(g) == d then "same" else "differs")'
program bad_stream 'try inflated([1, 2, 3]) catch e { print "caught " + e }'
program truncated "${small[@]}" 'c = deflated(d, 9)' 'try inflated(c[1..#c - 4]) catch e { print "caught " + e }'

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
check large "100000 bytes compressed to python's 713 and back" 0 "713" "97567627" "100000" "3008608506" ! "Error"
check known_input "uncompress of python's zlib.compress" 0 "[104 101 108 108 111 44 32 119 97 114 112 0 97 110 100" "2716549724"
check too_small "an output buffer too small fails the call with zlib's status" 1 "uncompress failed (C status -5)"
check stream_round_trip "deflate and inflate streams of 100000 bytes" 0 "97567627" "100000" "3008608506" ! "Error"
check stream_known "inflate of python's zlib.compress, its size unknown" 0 "[104 101 108 108 111" "2716549724"
check gzip "gzip through a stream: its magic bytes, and back" 0 "31" "139" "same" ! "differs"
check bad_stream "inflate of no zlib data throws zlib's message" 0 "caught zlib: incorrect header check"
check truncated "inflate of a truncated stream throws" 0 "caught zlib: the data ends before its stream does"
finish
