# Wasp Language Samples

This directory contains sample programs demonstrating various features of the Wasp language.

## ✅ Working Samples (11)

These samples compile and run successfully:

- **simple.wasp** - Basic arithmetic: `3*3`
- **fibonacci.wasp** - Fibonacci sequence using recursion
- **factorial.wasp** - Factorial calculation  
- **primes.wasp** - Prime number checking
- **gcd.wasp** - Greatest common divisor using Euclidean algorithm
- **sum.wasp** - Sum of numbers 1-10
- **power.wasp** - Exponentiation using recursion
- **collatz.wasp** - Collatz conjecture sequence
- **ackermann.wasp** - Ackermann function (recursive)
- **quadratic.wasp** - Quadratic formula with sqrt
- **fizzbuzz.wasp** - Classic FizzBuzz problem

## 🔧 Feature Requirements

These samples need specific features to be implemented:

### String Operations
- **hello.wasp** - String concatenation
- **comments.wasp** - Comment parsing (parse-only demo)

### Module System
- **main.wasp** - Uses `#use lib` directive
- **modules.wasp** - Module import/export

### Advanced Math
- **sine.wasp** - Needs τ and π constants, fraction literals
- **calculator.wasp** - Complex expression parser

### Advanced Language Features
- **json_parser.wasp** - Complex string manipulation
- **functions.wasp** - Higher-order functions, lambdas
- **control_flow.wasp** - Pattern matching, try/catch
- **async.wasp** - Async/await support
- **binary_tree.wasp** - Type definitions, optional types
- **quicksort.wasp** - Array filter, lambda functions
- **mandelbrot.wasp** - 2D arrays, complex iteration

### Data Structure Demos
- **data_structures.wasp** - Syntax examples (no executable output)
- **types.wasp** - Type system examples
- **html.wasp** - HTML generation
- **html_dsl.wasp** - DSL demonstrations

### Graphics & External Libraries  
- **raylib_*.wasp** (8 files) - Raylib FFI examples
- **webgpu.wasp** - WebGPU integration
- **sdl_red_square.wasp** - SDL integration
- **test_ffi*.wasp** (3 files) - FFI testing

### Complex Algorithms
- **game_of_life.wasp** - Cellular automaton
- **neural_net.wasp** - Neural network
- **sudoku.wasp** - Sudoku solver
- **raytracer.wasp** - Ray tracing
- **particles.wasp** - Particle system
- **snake.wasp** - Snake game

## Running Samples

```bash
# Run a working sample
cargo run -- samples/fibonacci.wasp

# Or use the test suite
cargo test --test test_samples
```

## Adding New Samples

When adding a new sample:
1. Add it to `samples/` directory with `.wasp` extension
2. If it should work, add a test in `tests/test_samples.rs`
3. Use `#[ignore]` attribute with explanation if feature not yet implemented
4. Update this README

## Current Test Status

```bash
cargo test --test test_samples
# 11 passed; 0 failed; 3 ignored
```
