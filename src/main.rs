mod compile;
mod instr;
mod llvm;
mod optimizer;
mod run;

use std::{ffi::OsString, fs::File, io::prelude::*, process, process::ExitCode};

use crate::{compile::CompiledProgram, llvm::LlvmEmitter};

use clap::{Args, Parser, Subcommand};
use which::which;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Args, Debug)]
struct CommonArgs {
    /// Filename of the BF program
    filename: OsString,

    /// Optimization level (0, 1, or 2)
    #[arg(
        short = 'O',
        long = "opt-level",
        default_value_t = 2,
        value_name = "LEVEL"
    )]
    opt_level: u8,
}

#[derive(Args, Debug)]
struct RunArgs {
    #[command(flatten)]
    common: CommonArgs,
}

#[derive(Args, Debug)]
struct CompileArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// Binary output file
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<OsString>,

    /// Tape length to use
    #[arg(
        short = 't',
        long = "tape-length",
        default_value_t = 30000,
        value_name = "LENGTH"
    )]
    tape_length: usize,
}

#[derive(Args, Debug)]
struct EmitArgs {
    #[command(flatten)]
    common: CommonArgs,

    /// LLVM IR output file
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output: Option<OsString>,

    /// Tape length to use
    #[arg(
        short = 't',
        long = "tape-length",
        default_value_t = 30000,
        value_name = "LENGTH"
    )]
    tape_length: usize,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Interpret a BF program directly
    Run(RunArgs),

    /// Compile a BF program to a native binary via LLVM
    Compile(CompileArgs),

    /// Emit LLVM IR without compiling
    Emit(EmitArgs),
}

fn command_run(args: RunArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(args.common.filename)?;

    let mut source = String::new();
    file.read_to_string(&mut source)?;

    let program = CompiledProgram::compile(&source, args.common.opt_level)?;

    program.run_stdio()?;

    Ok(())
}

fn command_emit(args: EmitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open(&args.common.filename)?;

    let mut source = String::new();
    file.read_to_string(&mut source)?;

    let program = CompiledProgram::compile(&source, args.common.opt_level)?;

    let llvm_ir_filename = args.output.unwrap_or_else(|| {
        let mut filename = args.common.filename.clone();
        filename.push(".ll");
        filename
    });
    let llvm_ir_file = File::create(llvm_ir_filename)?;
    LlvmEmitter::emit(llvm_ir_file, &program, args.tape_length)?;

    Ok(())
}

fn command_compile(args: CompileArgs) -> Result<(), Box<dyn std::error::Error>> {
    let clang_path = which("clang")?;

    let mut file = File::open(&args.common.filename)?;

    let mut source = String::new();
    file.read_to_string(&mut source)?;

    let output_filename = args.output.unwrap_or_else(|| OsString::from("a.out"));

    let program = CompiledProgram::compile(&source, args.common.opt_level)?;

    let mut llvm_ir_file = tempfile::Builder::new()
        .prefix("rustfuck-")
        .suffix(".ll")
        .rand_bytes(16)
        .tempfile()?;
    let llvm_ir_name = llvm_ir_file.path().to_owned();
    LlvmEmitter::emit(
        Box::new(llvm_ir_file.as_file_mut()),
        &program,
        args.tape_length,
    )?;

    let status = process::Command::new(clang_path)
        .arg(llvm_ir_name)
        .arg("-o")
        .arg(output_filename)
        .arg(format!("-O{}", args.common.opt_level))
        .status()?;

    match status.code() {
        Some(0) => {}
        Some(code) => eprintln!("clang exited with failed status code: {code}"),
        None => eprintln!("clang terminated by signal"),
    }

    Ok(())
}

fn main() -> ExitCode {
    let args = Cli::parse();
    match args.command {
        Command::Run(args) => {
            if let Err(e) = command_run(args) {
                eprintln!("Error running program:");
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        }
        Command::Emit(args) => {
            if let Err(e) = command_emit(args) {
                eprintln!("Error emitting LLVM IR for program:");
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        }
        Command::Compile(args) => {
            if let Err(e) = command_compile(args) {
                eprintln!("Error compiling program:");
                eprintln!("{e}");
                return ExitCode::FAILURE;
            }
        }
    }

    ExitCode::SUCCESS
}
