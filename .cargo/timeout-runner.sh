#!/bin/bash
# Smart timeout runner that allows longer timeouts when debugging
# Usage: Set NO_TIMEOUT=1 or DEBUG=1 to disable timeout

if [ -n "$NO_TIMEOUT" ] || [ -n "$DEBUG" ]; then
    # No timeout when debugging
    exec "$@"
elif [ -n "$LONG_TIMEOUT" ]; then
    # Extended timeout for slower tests
    exec gtimeout 3s "$@"
else
    # Normal quick timeout (enough for IDE initialization + test execution AFTER one NO_TIMEOUT run!)
    exec gtimeout 1s "$@"
fi
