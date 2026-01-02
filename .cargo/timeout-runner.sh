#!/bin/bash
# Smart timeout runner that allows longer timeouts when debugging
# Usage: Set NO_TIMEOUT=1 or DEBUG=1 to disable timeout

if [ -n "$DEBUG" ]; then
    # No timeout when debugging
    exec "$@"
elif [ -n "$FAST_TIMEOUT" ]; then
    # Normal quick timeout (enough for IDE initialization + test execution AFTER one NO_TIMEOUT run!)
    exec gtimeout 1s "$@"
else
    # No timeout when debugging
    exec "$@"
fi
