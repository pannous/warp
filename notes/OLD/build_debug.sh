#!/bin/bash
# Build with full debug symbols for RustRover debugging
# Only rebuilds if not already built with debug info
CARGO_PROFILE_DEV_DEBUG=true cargo build --tests
