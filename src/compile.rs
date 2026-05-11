use crate::instr::Instr;

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("Unterminated loop, start at byte {0}")]
    UnterminatedLoop(usize),

    #[error("End of loop at byte {0} has no corresponding start")]
    NoLoopStart(usize),

    #[error("Program empty")]
    ProgramEmpty,
}

#[derive(Debug, Default)]
pub struct CompiledProgram {
    pub instrs: Vec<Instr>,
    pub _warnings: Vec<String>,
}

impl CompiledProgram {
    pub fn compile(source: &str, opt_level: u8) -> Result<Self, CompileError> {
        let mut program = Self::default();
        let mut loop_starts = Vec::new();

        for (i, ch) in source.chars().enumerate() {
            match ch {
                '>' => program.instrs.push(Instr::AddPtr(1)),
                '<' => program.instrs.push(Instr::SubPtr(1)),
                '+' => program.instrs.push(Instr::AddByte(1)),
                '-' => program.instrs.push(Instr::SubByte(1)),
                '[' => {
                    let idx = program.instrs.len();
                    loop_starts.push(idx);
                    program.instrs.push(Instr::JmpIfZero(0)); // placeholder, patched at ]
                }
                ']' => match loop_starts.pop() {
                    None => return Err(CompileError::NoLoopStart(i + 1)),
                    Some(start) => {
                        let end = program.instrs.len();
                        program.instrs.push(Instr::JmpIfNonzero(start));
                        program.instrs[start] = Instr::JmpIfZero(end); // [ -> patched to point to ]
                    }
                },
                '.' => program.instrs.push(Instr::Output),
                ',' => program.instrs.push(Instr::Input),
                _ => {}
            }
        }

        if program.instrs.is_empty() {
            return Err(CompileError::ProgramEmpty);
        }

        if let Some(start) = loop_starts.pop() {
            return Err(CompileError::UnterminatedLoop(start + 1));
        }

        // Limit to two optimization passes
        for _ in 0..(opt_level.min(2)) {
            program.optimize();
        }

        Ok(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instr::Instr;

    #[test]
    fn test_compile_add_byte() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("+", 0)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::AddByte(1));

        Ok(())
    }

    #[test]
    fn test_compile_sub_byte() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("-", 0)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SubByte(1));

        Ok(())
    }

    #[test]
    fn test_compile_add_ptr() -> Result<(), CompileError> {
        let program = CompiledProgram::compile(">", 0)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::AddPtr(1));

        Ok(())
    }

    #[test]
    fn test_compile_sub_ptr() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("<", 0)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SubPtr(1));

        Ok(())
    }

    #[test]
    fn test_compile_input() -> Result<(), CompileError> {
        let program = CompiledProgram::compile(",", 0)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::Input);

        Ok(())
    }

    #[test]
    fn test_compile_output() -> Result<(), CompileError> {
        let program = CompiledProgram::compile(".", 0)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::Output);

        Ok(())
    }

    #[test]
    fn test_compile_loop() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[]", 0)?;

        assert_eq!(program.instrs.len(), 2);
        assert_eq!(program.instrs[0], Instr::JmpIfZero(1));
        assert_eq!(program.instrs[1], Instr::JmpIfNonzero(0));

        Ok(())
    }

    #[test]
    fn test_compile_loop_nested() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[[]]", 0)?;

        assert_eq!(program.instrs.len(), 4);

        // First outer loop
        assert_eq!(program.instrs[0], Instr::JmpIfZero(3));
        assert_eq!(program.instrs[3], Instr::JmpIfNonzero(0));

        // Second inner loop
        assert_eq!(program.instrs[1], Instr::JmpIfZero(2));
        assert_eq!(program.instrs[2], Instr::JmpIfNonzero(1));

        Ok(())
    }

    #[test]
    fn test_compile_simple_program() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[+-]", 0)?;

        assert_eq!(program.instrs.len(), 4);
        assert_eq!(program.instrs[0], Instr::JmpIfZero(3));
        assert_eq!(program.instrs[1], Instr::AddByte(1));
        assert_eq!(program.instrs[2], Instr::SubByte(1));
        assert_eq!(program.instrs[3], Instr::JmpIfNonzero(0));

        Ok(())
    }

    #[test]
    fn test_compile_with_comment() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("+ hello world +", 0)?;

        assert_eq!(program.instrs.len(), 2);
        assert_eq!(program.instrs[0], Instr::AddByte(1));
        assert_eq!(program.instrs[1], Instr::AddByte(1));

        Ok(())
    }

    #[test]
    fn test_compile_empty() {
        assert!(matches!(
            CompiledProgram::compile("", 0),
            Err(CompileError::ProgramEmpty)
        ));
    }

    #[test]
    fn test_compile_empty_with_comment() {
        assert!(matches!(
            CompiledProgram::compile("hello world", 0),
            Err(CompileError::ProgramEmpty)
        ));
    }

    #[test]
    fn test_compile_whitespace_only() {
        assert!(matches!(
            CompiledProgram::compile("   \n\t  ", 0),
            Err(CompileError::ProgramEmpty)
        ));
    }

    #[test]
    fn test_compile_loop_open_malformed() {
        assert!(matches!(
            CompiledProgram::compile("[", 0),
            Err(CompileError::UnterminatedLoop(1))
        ));
    }

    #[test]
    fn test_compile_loop_closed_malformed() {
        assert!(matches!(
            CompiledProgram::compile("]", 0),
            Err(CompileError::NoLoopStart(1))
        ));
    }
}
