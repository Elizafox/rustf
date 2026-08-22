# rustf
A Brainfuck compiler and interpreter written in Rust, with an optimising bytecode compiler and LLVM
backend.

## What is Brainfuck?
For the uninitiated, Brainfuck is an esoteric programming language created in 1993 by Urban Müller.
It operates on an array of memory cells using just eight commands:

| Command | Description |
|---|---|
| `>` | Move the data pointer right |
| `<` | Move the data pointer left |
| `+` | Increment the current cell |
| `-` | Decrement the current cell |
| `.` | Output the current cell as ASCII |
| `,` | Read one byte of input into the current cell |
| `[` | Jump past the matching `]` if the current cell is zero |
| `]` | Jump back to the matching `[` if the current cell is nonzero |

Everything else is a comment. Despite its simplicity, Brainfuck is
[Turing-complete](https://en.wikipedia.org/wiki/Turing_completeness).

## Features
- Interpreter — runs brainfuck programs directly via an optimising bytecode compiler
- LLVM backend — compiles brainfuck programs to native binaries via LLVM IR
- Optimiser — coalesces repeated operations, eliminates clear loops, folds constants, and 
  detects move-add patterns
- Configurable tape — set the tape length for compiled programs

## Requirements
- Rust 1.87+
- LLVM/clang (for `compile` and `emit` commands)

## Installation
```bash
cargo install --path .
```

## Usage
### Interpret a brainfuck program
```bash
rustf run hello.bf
rustf run -O0 hello.bf  # disable optimiser
```

### Compile to a native binary
```bash
rustf compile hello.bf           # outputs a.out
rustf compile hello.bf -o hello  # custom output name
rustf compile hello.bf -O3       # maximum optimisation
rustf compile hello.bf -t 60000  # larger tape
```

### Emit LLVM IR
```bash
rustf emit hello.bf              # prints IR to stdout
rustf emit hello.bf -o hello.ll  # write IR to file
```

## Options
### `run`
| Option | Description | Default |
|--------|-------------|---------|
| `-O, --opt-level <LEVEL>` | Optimisation level (0, 1, or 2) | `2` |

### `compile`
| Option | Description | Default |
|--------|-------------|---------|
| `-O, --opt-level <LEVEL>` | Optimisation level (0, 1, 2, or 3) | `2` |
| `-o, --output <FILE>` | Output binary path | `a.out` |
| `-t, --tape-length <LENGTH>` | Tape size in cells | `30000` |

### `emit`
| Option | Description | Default |
|--------|-------------|---------|
| `-O, --opt-level <LEVEL>` | Optimisation level (0, 1, 2, or 3) | `2` |
| `-o, --output <FILE>` | Output IR file (omit for stdout) | stdout |
| `-t, --tape-length <LENGTH>` | Tape size in cells | `30000` |

## Optimisations
The optimiser runs up to two passes over the bytecode:

- Run-length coalescing — `+++` becomes `AddByte(3)`, `>>><<` becomes `AddPtr(1)`
- Opposing op folding — `+++--` becomes `AddByte(1)`, cancelling ops emit nothing
- Clear loop detection — `[-]` and `[+]` become `SetByte(0)`
- Constant folding — `SetByte` followed by arithmetic folds into a single `SetByte`
- Dead loop elimination — `[]` is unreachable dead code and is removed
- Move-add detection — `[->+<]` becomes a single `MoveAdd` instruction

## Performance
Measured on Apple Silicon (ARM64), rendering the Brainfuck Mandelbrot:

| Mode | Time |
|------|------|
| Interpreter, `-O0` | 12.58s |
| Interpreter, `-O2` | 5.16s |
| Native, clang `-O0` | 8.50s |
| Native, clang `-O1` | 0.57s |
| Native, clang `-O2` | 0.39s |

Unoptimised native code is slower than the optimised interpreter. Without any optimisation passes
enabled, LLVM emits naive code that can't compete with a bytecode interpreter that has already
collapsed the most expensive patterns. At `-O1` and above, LLVM wins decisively; LLVM is now doing
what it was made to do, and no Brainfuck interpreter is ever going to win against it.

## Caveats
- **It's Brainfuck.** Manage your expectations accordingly.
- The optimiser does not perform dataflow analysis across loop boundaries, thus opportunities for 
  constant folding inside loops are missed.
- The move-add detection (`[->+<]`) only handles the simple single-cell case. Multiply loops 
  (`[->>+++<<]`), etc. are not detected and will run as ordinary loops.
- The optimiser can't work miracles. **It's Brainfuck**.
- The optimiser is limited in effectiveness. This is partly a consequence of the halting problem,
  and partly a consequence of the fact that it optimises Brainfuck, a language that was designed as
  a joke and is inherently difficult to optimise. Impressive results (given the constraints) have
  nonetheless been observed, which says more about LLVM than about Brainfuck.
- Tape bounds are not checked at runtime in compiled mode; walking off either end of the tape is
  undefined behaviour and will probably segfault. If you need a tape length of more than 30,000
  bytes, use the `-t` option, and professional help is strongly advised.
- The LLVM backend requires clang to be installed and on your PATH. If you don't have clang, `run`
  still works fine.
- Optimisation level `-O` refers to `rustf`'s own bytecode optimiser, not clang's. In `compile`
  mode both are applied independently, so `-O0` gives you unoptimised bytecode fed to clang at
  `-O0`, which produces truly artisanal compiler-crafted ~~slop~~slow code.
- EOF behaviour follows the "store 0" convention. Programs written for the "store -1" or "no
  change" conventions may behave incorrectly on input.
- This has been tested on macOS. It probably works on Linux. It has not been tested on Windows and
  the author makes no promises, though it'll probably work as long as clang is installed.
- If you are using this tool to solve real problems at work, please update your CV and quit.
- The existence of this project is itself a cry for help.

## License
This is free and unencumbered software released into the public domain.

Anyone is free to copy, modify, publish, use, compile, sell, or distribute this software, either in
source code form or as a compiled binary, for any purpose, commercial or non-commercial, and by any
means.

See [unlicense.org](https://unlicense.org) for more information. A copy of the Unlicense is
available in the `LICENSE` file.
