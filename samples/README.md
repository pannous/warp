# Warp Language Samples

This directory contains sample programs demonstrating various features of the Warp language.

## ✅ Working Samples (11)

These samples compile and run successfully:

- **simple.warp** - Basic arithmetic: `3*3`
- **fibonacci.warp** - Fibonacci sequence using recursion
- **factorial.warp** - Factorial calculation  
- **primes.warp** - Prime number checking
- **gcd.warp** - Greatest common divisor using Euclidean algorithm
- **sum.warp** - Sum of numbers 1-10
- **power.warp** - Exponentiation using recursion
- **collatz.warp** - Collatz conjecture sequence
- **ackermann.warp** - Ackermann function (recursive)
- **quadratic.warp** - Quadratic formula with sqrt
- **fizzbuzz.warp** - Classic FizzBuzz problem

## 🔧 Feature Requirements

These samples need specific features to be implemented:

### String Operations
- **hello.warp** - String concatenation
- **comments.warp** - Comment parsing (parse-only demo)

### Module System
- **main.warp** - Uses `#use lib` directive
- **modules.warp** - Module import/export

### Advanced Math
- **sine.warp** - Needs τ and π constants, fraction literals
- **calculator.warp** - Complex expression parser

### Advanced Language Features
- **json_parser.warp** - Complex string manipulation
- **functions.warp** - Higher-order functions, lambdas
- **control_flow.warp** - Pattern matching, try/catch
- **async.warp** - Async/await support
- **binary_tree.warp** - Type definitions, optional types
- **quicksort.warp** - Array filter, lambda functions
- **mandelbrot.warp** - 2D arrays, complex iteration

### Data Structure Demos
- **data_structures.warp** - Syntax examples (no executable output)
- **types.warp** - Type system examples
- **html.warp** - HTML generation
- **html_dsl.warp** - DSL demonstrations

### Graphics & External Libraries  
- **raylib_*.warp** (8 files) - Raylib FFI examples
- **webgpu.warp** - WebGPU integration
- **sdl_red_square.warp** - SDL integration
- **test_ffi*.warp** (3 files) - FFI testing

### Complex Algorithms
- **game_of_life.warp** - Cellular automaton
- **neural_net.warp** - Neural network
- **sudoku.warp** - Sudoku solver
- **raytracer.warp** - Ray tracing
- **particles.warp** - Particle system
- **snake.warp** - Snake game

## Running Samples

```bash
# Run a working sample
cargo run -- samples/fibonacci.warp

# Or use the test suite
cargo test --test test_samples
```

## Adding New Samples

When adding a new sample:
1. Add it to `samples/` directory with `.warp` extension
2. If it should work, add a test in `tests/programs/test_samples.rs`
3. Use `#[ignore]` attribute with explanation if feature not yet implemented
4. Update this README

## Current Test Status

```bash
cargo test --test test_samples
# 11 passed; 0 failed; 3 ignored
```
