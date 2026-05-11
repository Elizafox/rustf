#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Instr {
    AddPtr(usize),
    SubPtr(usize),
    AddByte(u8),
    SubByte(u8),
    SetByte(u8),
    Output,
    Input,
    JmpIfZero(usize),
    JmpIfNonzero(usize),
    MoveAdd(isize), // Not in real BF programs, but used by the optimiser
}
