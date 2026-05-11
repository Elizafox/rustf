use crate::{compile::CompiledProgram, instr::Instr};

impl CompiledProgram {
    pub fn optimize(&mut self) {
        let instrs = std::mem::take(&mut self.instrs);
        let mut out: Vec<Instr> = Vec::with_capacity(instrs.len());
        let mut idx = 0;

        while idx < instrs.len() {
            match instrs[idx] {
                Instr::AddByte(_) | Instr::SubByte(_) => {
                    idx = Self::add_sub_byte(&instrs, &mut out, idx);
                }
                Instr::AddPtr(_) | Instr::SubPtr(_) => {
                    idx = Self::add_sub_ptr(&instrs, &mut out, idx);
                }
                Instr::JmpIfZero(_) => idx = Self::jmp_if_zero(&instrs, &mut out, idx),
                Instr::SetByte(n) => idx = Self::set_byte(&instrs, &mut out, idx, n),
                instr => {
                    out.push(instr);
                    idx += 1;
                }
            }
        }

        self.instrs = out;
        self.resolve_jumps();
    }

    #[inline]
    fn add_sub_byte(instrs: &[Instr], out: &mut Vec<Instr>, mut idx: usize) -> usize {
        let mut acc: i16 = match instrs[idx] {
            Instr::AddByte(n) => i16::from(n),
            Instr::SubByte(n) => -i16::from(n),
            _ => unreachable!(),
        };
        loop {
            match instrs.get(idx + 1) {
                Some(Instr::AddByte(m)) => acc += i16::from(*m),
                Some(Instr::SubByte(m)) => acc -= i16::from(*m),
                _ => break,
            }
            idx += 1;
        }

        match u8::try_from(acc.rem_euclid(256)).unwrap() {
            0 => {} // cancels out, emit nothing
            n if acc >= 0 => out.push(Instr::AddByte(n)),
            n => out.push(Instr::SubByte(n.wrapping_neg())),
        }

        idx + 1
    }

    #[inline]
    fn add_sub_ptr(instrs: &[Instr], out: &mut Vec<Instr>, mut idx: usize) -> usize {
        let mut acc: isize = match instrs[idx] {
            Instr::AddPtr(n) => n.cast_signed(),
            Instr::SubPtr(n) => -(n.cast_signed()),
            _ => unreachable!(),
        };
        loop {
            match instrs.get(idx + 1) {
                Some(Instr::AddPtr(m)) => acc += (*m).cast_signed(),
                Some(Instr::SubPtr(m)) => acc -= (*m).cast_signed(),
                _ => break,
            }
            idx += 1;
        }

        match acc {
            0 => {} // cancels out, emit nothing
            n if n > 0 => out.push(Instr::AddPtr(n.cast_unsigned())),
            n => out.push(Instr::SubPtr(n.unsigned_abs())),
        }

        idx + 1
    }

    #[inline]
    fn jmp_if_zero(instrs: &[Instr], out: &mut Vec<Instr>, mut idx: usize) -> usize {
        // [->+<] pattern: move add
        let is_move_add = match (
            instrs.get(idx + 1),
            instrs.get(idx + 2),
            instrs.get(idx + 3),
            instrs.get(idx + 4),
            instrs.get(idx + 5),
        ) {
            (
                Some(Instr::SubByte(1)),
                Some(Instr::AddPtr(n)),
                Some(Instr::AddByte(1)),
                Some(Instr::SubPtr(m)),
                Some(Instr::JmpIfNonzero(_)),
            ) if n == m => Some((*n).cast_signed()),
            (
                Some(Instr::SubByte(1)),
                Some(Instr::SubPtr(n)),
                Some(Instr::AddByte(1)),
                Some(Instr::AddPtr(m)),
                Some(Instr::JmpIfNonzero(_)),
            ) if n == m => Some(-((*n).cast_signed())),
            _ => None,
        };

        if let Some(offset) = is_move_add {
            out.push(Instr::MoveAdd(offset));
            idx += 6; // [->+<]: move add
        } else if matches!(
            (instrs.get(idx + 1), instrs.get(idx + 2)),
            (
                Some(Instr::SubByte(1) | Instr::AddByte(1)),
                Some(Instr::JmpIfNonzero(_))
            )
        ) {
            out.push(Instr::SetByte(0));
            idx += 3;
        // []: cell is zero to enter, unconditionally skipped, dead code
        } else if matches!(instrs.get(idx + 1), Some(Instr::JmpIfNonzero(_))) {
            idx += 2;
        } else {
            out.push(instrs[idx]);
            idx += 1;
        }

        idx
    }

    #[inline]
    fn set_byte(instrs: &[Instr], out: &mut Vec<Instr>, mut idx: usize, mut n: u8) -> usize {
        // redundant SetByte: SetByte(x), SetByte(y) -> SetByte(y)
        // keep consuming until no more SetBytes
        while let Some(Instr::SetByte(m)) = instrs.get(idx + 1) {
            n = *m;
            idx += 1;
        }
        // fold trailing AddByte/SubByte into the known value
        loop {
            match instrs.get(idx + 1) {
                Some(Instr::AddByte(m)) => {
                    n = n.wrapping_add(*m);
                    idx += 1;
                }
                Some(Instr::SubByte(m)) => {
                    n = n.wrapping_sub(*m);
                    idx += 1;
                }
                _ => break,
            }
        }
        out.push(Instr::SetByte(n));

        idx + 1
    }

    #[inline]
    fn resolve_jumps(&mut self) {
        let mut loop_starts: Vec<usize> = Vec::new();
        for idx in 0..self.instrs.len() {
            match self.instrs[idx] {
                Instr::JmpIfZero(_) => loop_starts.push(idx),
                Instr::JmpIfNonzero(_) => {
                    // If this panics, it's a bug
                    let start = loop_starts.pop().expect("unmatched ] in optimized output");
                    self.instrs[start] = Instr::JmpIfZero(idx);
                    self.instrs[idx] = Instr::JmpIfNonzero(start);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        compile::{CompileError, CompiledProgram},
        instr::Instr,
    };

    #[test]
    fn test_compile_optimize_fold_add_ptr() -> Result<(), CompileError> {
        let program = CompiledProgram::compile(">>>", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::AddPtr(3));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_fold_sub_ptr() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("<<<", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SubPtr(3));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_fold_add_sub_ptr_positive() -> Result<(), CompileError> {
        let program = CompiledProgram::compile(">>><<", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::AddPtr(1));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_fold_add_sub_ptr_negative() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("<<<>>", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SubPtr(1));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_fold_add_byte() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("+++", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::AddByte(3));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_fold_sub_byte() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("---", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SubByte(3));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_fold_add_sub_byte_positive() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("+++--", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::AddByte(1));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_fold_add_sub_byte_negative() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("---++", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SubByte(1));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_skip_empty_loop() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[]+", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::AddByte(1));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_clear_loop_sub() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[-]", 2)?;
        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SetByte(0));
        Ok(())
    }

    #[test]
    fn test_compile_optimize_clear_loop_add() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[+]", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SetByte(0));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_set_byte_fold_add() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[-]+++", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SetByte(3));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_set_byte_fold_sub() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[-]---", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SetByte(253));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_redundant_set_byte() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[-][-]", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::SetByte(0));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_move_add_forward() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[->+<]", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::MoveAdd(1));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_move_add_backward() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("[-<+>]", 2)?;

        assert_eq!(program.instrs.len(), 1);
        assert_eq!(program.instrs[0], Instr::MoveAdd(-1));

        Ok(())
    }

    #[test]
    fn test_compile_optimize_byte_wraps() -> Result<(), CompileError> {
        // 256 additions should cancel to zero and emit nothing...
        // or wrap to AddByte(0) which should also emit nothing
        let program = CompiledProgram::compile(&"+".repeat(256), 2)?;

        assert_eq!(program.instrs.len(), 0);

        Ok(())
    }

    #[test]
    fn test_compile_optimize_ptr_cancels() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("><", 2)?;
        assert_eq!(program.instrs.len(), 0);
        Ok(())
    }

    #[test]
    fn test_compile_optimize_byte_cancels() -> Result<(), CompileError> {
        let program = CompiledProgram::compile("+-", 2)?;

        assert_eq!(program.instrs.len(), 0);

        Ok(())
    }
}
