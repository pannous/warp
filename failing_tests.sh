#!/bin/bash
# Script to list only failing test names from cargo test
# Usage: ./failing_tests.sh

cargo test 2>&1 | grep "FAILED" | awk '{print $2}' | sort | uniq
