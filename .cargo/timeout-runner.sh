#!/bin/bash
# Smart timeout runner that allows longer timeouts when debugging
# Usage: Set NO_TIMEOUT=1 or DEBUG=1 to disable timeout

if [ -n "$NO_TIMEOUT" ] || [ -n "$DEBUG" ]; then
    # No timeout when debugging
    exec "$@"
elif [ -n "$LONG_TIMEOUT" ]; then
    # Extended timeout for slower tests
    exec gtimeout 120s "$@"
else
    # Normal timeout (enough for IDE initialization + test execution)
    exec gtimeout 30s "$@"
fi
