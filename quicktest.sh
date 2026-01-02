#!/bin/bash
# Quick test with tee - simpler version
cargo test 2>&1 | tee test_results.txt
