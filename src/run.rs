use std::io::{self, prelude::*};

use crate::{compile::CompiledProgram, instr::Instr};

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Pointer underflow")]
    PointerUnderflow,
}

impl CompiledProgram {
    pub fn run_stdio(&self) -> Result<(), RuntimeError> {
        let stdin = io::stdin();
        let stdout = io::stdout();

        let mut stdout = stdout.lock();

        let res = self.run(&mut stdin.lock(), &mut stdout);
        stdout.flush()?;
        res
    }

    pub fn run(&self, input: &mut impl Read, output: &mut impl Write) -> Result<(), RuntimeError> {
        let mut data_cells = vec![0u8; 30000]; // Extendable, but traditional
        let mut data_ptr = 0usize;
        let mut instr_ptr = 0usize;

        let instr_len = self.instrs.len();
        while instr_ptr < instr_len {
            match self.instrs[instr_ptr] {
                Instr::Input => {
                    let mut ch = [0u8; 1];
                    match input.read(&mut ch)? {
                        0 => data_cells[data_ptr] = 0, // EOF -> 0
                        _ => data_cells[data_ptr] = ch[0],
                    }
                }
                Instr::Output => {
                    let data = [data_cells[data_ptr]; 1];
                    output.write_all(&data)?;
                }
                Instr::AddPtr(n) => {
                    data_ptr += n;
                    if data_ptr >= data_cells.len() {
                        data_cells.resize(data_ptr + 1, 0);
                    }
                }
                Instr::SubPtr(n) => {
                    data_ptr = data_ptr
                        .checked_sub(n)
                        .ok_or(RuntimeError::PointerUnderflow)?;
                }
                Instr::AddByte(n) => {
                    data_cells[data_ptr] = data_cells[data_ptr].wrapping_add(n);
                }
                Instr::SubByte(n) => {
                    data_cells[data_ptr] = data_cells[data_ptr].wrapping_sub(n);
                }
                Instr::SetByte(n) => {
                    data_cells[data_ptr] = n;
                }
                Instr::JmpIfZero(n) => {
                    if data_cells[data_ptr] == 0 {
                        instr_ptr = n;
                        continue;
                    }
                }
                Instr::JmpIfNonzero(n) => {
                    if data_cells[data_ptr] != 0 {
                        instr_ptr = n;
                        continue;
                    }
                }
                Instr::MoveAdd(offset) => {
                    let src = data_cells[data_ptr];
                    if src != 0 {
                        let dst_signed = data_ptr.cast_signed() + offset;
                        if dst_signed < 0 {
                            return Err(RuntimeError::PointerUnderflow);
                        }
                        let dst = dst_signed.cast_unsigned();
                        if dst >= data_cells.len() {
                            data_cells.resize(dst + 1, 0);
                        }
                        data_cells[dst] = data_cells[dst].wrapping_add(src);
                        data_cells[data_ptr] = 0;
                    }
                }
            }

            instr_ptr += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::compile::CompiledProgram;

    fn run_program(source: &str, input: &[u8], opt: u8) -> Vec<u8> {
        let program = CompiledProgram::compile(source, opt).unwrap();
        let mut input = input;
        let mut output = Vec::new();

        program.run(&mut input, &mut output).unwrap();

        output
    }

    #[test]
    fn hello_world() {
        assert_eq!(
            run_program(
                "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.",
                b"",
                2,
            ),
            b"Hello World!\n"
        );
    }

    #[test]
    fn cat() {
        assert_eq!(run_program(",[.,]", b"hello", 2), b"hello");
    }

    #[test]
    fn wrapping_add() {
        // cell starts at 0, subtract 1 should wrap to 255
        assert_eq!(run_program("-.", b"", 2)[0], 255);
    }

    #[test]
    fn wrapping_sub() {
        // 256 increments should wrap back to 0
        assert_eq!(run_program(&format!("{}.", "+".repeat(256)), b"", 2)[0], 0);
    }

    #[test]
    fn multiple_cells() {
        // set cell 0 to 1, cell 1 to 2, cell 2 to 3, output all three
        assert_eq!(run_program("+>++>+++<<.>.>.", b"", 2), &[1, 2, 3]);
    }

    #[test]
    fn clear_loop() {
        // set cell to 6, clear it, output should be 0
        assert_eq!(run_program("++++++[-].", b"", 2)[0], 0);
    }

    #[test]
    fn move_add() {
        // cell 0 = 3, cell 1 = 2, move cell 0 into cell 1 -> cell 0 = 0, cell 1 = 5
        assert_eq!(run_program("+++>++<[->+<]>.", b"", 2)[0], 5);
    }

    #[test]
    fn optimizer_equivalence_hello_world() {
        let src = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
        assert_eq!(run_program(src, b"", 0), run_program(src, b"", 2));
    }

    #[test]
    fn optimizer_equivalence_cat() {
        assert_eq!(
            run_program(",[.,]", b"hello world", 0),
            run_program(",[.,]", b"hello world", 2)
        );
    }
}
