use crate::{compile::CompiledProgram, instr::Instr};
use std::io;

pub struct LlvmEmitter<'a> {
    output: Box<dyn io::Write + 'a>,
    label_counter: usize,
    loop_stack: Vec<usize>,
    tape_size: usize,
}

impl<'a> LlvmEmitter<'a> {
    pub fn emit(
        output: Box<dyn io::Write + 'a>,
        program: &CompiledProgram,
        tape_size: usize,
    ) -> io::Result<()> {
        let mut emitter = Self {
            output,
            label_counter: 0,
            loop_stack: Vec::new(),
            tape_size,
        };
        emitter.emit_prelude()?;
        emitter.emit_instrs(&program.instrs)?;
        emitter.emit_epilogue()?;
        Ok(())
    }

    #[inline]
    const fn new_label(&mut self) -> usize {
        let n = self.label_counter;
        self.label_counter += 1;
        n
    }

    #[inline]
    fn emit_prelude(&mut self) -> io::Result<()> {
        writeln!(self.output, "; Prelude")?;
        writeln!(self.output)?;
        writeln!(self.output, "declare i32 @putchar(i32)")?;
        writeln!(self.output, "declare i32 @getchar()")?;
        writeln!(
            self.output,
            "declare void @llvm.memset.p0.i64(ptr, i8, i64, i1)"
        )?;
        writeln!(self.output)?;
        writeln!(self.output, "; Begin main function")?;
        writeln!(self.output)?;
        writeln!(self.output, "define i32 @main() {{")?;
        writeln!(self.output, "entry:")?;
        writeln!(
            self.output,
            "  %tape = alloca [{} x i8], align 1",
            self.tape_size
        )?;
        writeln!(self.output, "  %ptr = alloca ptr, align 8")?;
        writeln!(
            self.output,
            "  %tape_start = getelementptr [{} x i8], ptr %tape, i32 0, i32 0",
            self.tape_size
        )?;
        writeln!(
            self.output,
            "  call void @llvm.memset.p0.i64(ptr %tape_start, i8 0, i64 {}, i1 false)",
            self.tape_size,
        )?;
        writeln!(self.output, "  store ptr %tape_start, ptr %ptr")?;

        Ok(())
    }

    #[inline]
    fn emit_epilogue(&mut self) -> io::Result<()> {
        writeln!(self.output, "  ret i32 0")?;
        writeln!(self.output, "}}")?;
        writeln!(self.output)?;

        Ok(())
    }

    #[inline]
    fn emit_add_ptr(&mut self, n: usize) -> io::Result<()> {
        writeln!(self.output, "  ; BF instruction >")?;
        writeln!(
            self.output,
            "  %ptr_val_{0} = load ptr, ptr %ptr",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %ptr_new_{0} = getelementptr i8, ptr %ptr_val_{0}, i64 {1}",
            self.label_counter, n
        )?;
        writeln!(
            self.output,
            "  store ptr %ptr_new_{0}, ptr %ptr",
            self.label_counter
        )?;
        writeln!(self.output)?;

        self.label_counter += 1;

        Ok(())
    }

    #[inline]
    fn emit_sub_ptr(&mut self, n: usize) -> io::Result<()> {
        writeln!(self.output, "  ; BF instruction <")?;
        writeln!(
            self.output,
            "  %ptr_val_{0} = load ptr, ptr %ptr",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %ptr_new_{0} = getelementptr i8, ptr %ptr_val_{0}, i64 -{1}",
            self.label_counter, n
        )?;
        writeln!(
            self.output,
            "  store ptr %ptr_new_{0}, ptr %ptr",
            self.label_counter
        )?;
        writeln!(self.output)?;

        self.label_counter += 1;

        Ok(())
    }

    #[inline]
    fn emit_add_byte(&mut self, n: u8) -> io::Result<()> {
        writeln!(self.output, "  ; BF instruction +")?;
        writeln!(
            self.output,
            "  %cell_ptr_{0} = load ptr, ptr %ptr",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %cell_val_{0} = load i8, ptr %cell_ptr_{0}",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %cell_new_{0} = add i8 %cell_val_{0}, {1}",
            self.label_counter, n
        )?;
        writeln!(
            self.output,
            "  store i8 %cell_new_{0}, ptr %cell_ptr_{0}",
            self.label_counter
        )?;
        writeln!(self.output)?;

        self.label_counter += 1;

        Ok(())
    }

    #[inline]
    fn emit_sub_byte(&mut self, n: u8) -> io::Result<()> {
        writeln!(self.output, "  ; BF instruction -")?;
        writeln!(
            self.output,
            "  %cell_ptr_{0} = load ptr, ptr %ptr",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %cell_val_{0} = load i8, ptr %cell_ptr_{0}",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %cell_new_{0} = sub i8 %cell_val_{0}, {1}",
            self.label_counter, n
        )?;
        writeln!(
            self.output,
            "  store i8 %cell_new_{0}, ptr %cell_ptr_{0}",
            self.label_counter
        )?;
        writeln!(self.output)?;

        self.label_counter += 1;

        Ok(())
    }

    #[inline]
    fn emit_set_byte(&mut self, n: u8) -> io::Result<()> {
        writeln!(self.output, "  ; Set byte")?;
        writeln!(
            self.output,
            "  %set_ptr_{0} = load ptr, ptr %ptr",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  store i8 {1}, ptr %set_ptr_{0}",
            self.label_counter, n
        )?;
        writeln!(self.output)?;

        self.label_counter += 1;

        Ok(())
    }

    #[inline]
    fn emit_output(&mut self) -> io::Result<()> {
        writeln!(self.output, "  ; BF instruction .")?;
        writeln!(
            self.output,
            "  %out_ptr_{0} = load ptr, ptr %ptr",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %out_val_{0} = load i8, ptr %out_ptr_{0}",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %out_ext_{0} = zext i8 %out_val_{0} to i32",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  call i32 @putchar(i32 %out_ext_{0})",
            self.label_counter
        )?;
        writeln!(self.output)?;

        self.label_counter += 1;

        Ok(())
    }

    #[inline]
    fn emit_input(&mut self) -> io::Result<()> {
        let id = self.label_counter;
        self.label_counter += 1;

        writeln!(self.output, "  ; BF instruction ,")?;
        writeln!(self.output, "  %in_ptr_{id} = load ptr, ptr %ptr")?;
        writeln!(self.output, "  %in_val_{id} = call i32 @getchar()")?;
        writeln!(self.output, "  %in_eof_{id} = icmp slt i32 %in_val_{id}, 0")?;
        writeln!(
            self.output,
            "  br i1 %in_eof_{id}, label %in_eof_block_{id}, label %in_ok_block_{id}"
        )?;
        writeln!(self.output, "in_eof_block_{id}:")?;
        writeln!(self.output, "  store i8 0, ptr %in_ptr_{id}")?;
        writeln!(self.output, "  br label %in_done_{id}")?;
        writeln!(self.output, "in_ok_block_{id}:")?;
        writeln!(
            self.output,
            "  %in_trunc_{id} = trunc i32 %in_val_{id} to i8"
        )?;
        writeln!(self.output, "  store i8 %in_trunc_{id}, ptr %in_ptr_{id}")?;
        writeln!(self.output, "  br label %in_done_{id}")?;
        writeln!(self.output, "in_done_{id}:")?;
        writeln!(self.output)?;

        Ok(())
    }

    #[inline]
    fn emit_move_add(&mut self, offset: isize) -> io::Result<()> {
        writeln!(self.output, "  ; Move-add")?;
        writeln!(
            self.output,
            "  %ma_src_ptr_{0} = load ptr, ptr %ptr",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %ma_src_val_{0} = load i8, ptr %ma_src_ptr_{0}",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %ma_dst_ptr_{0} = getelementptr i8, ptr %ma_src_ptr_{0}, i64 {1}",
            self.label_counter, offset
        )?;
        writeln!(
            self.output,
            "  %ma_dst_val_{0} = load i8, ptr %ma_dst_ptr_{0}",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  %ma_new_val_{0} = add i8 %ma_dst_val_{0}, %ma_src_val_{0}",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  store i8 %ma_new_val_{0}, ptr %ma_dst_ptr_{0}",
            self.label_counter
        )?;
        writeln!(
            self.output,
            "  store i8 0, ptr %ma_src_ptr_{0}",
            self.label_counter
        )?;
        writeln!(self.output)?;

        self.label_counter += 1;
        Ok(())
    }

    #[inline]
    fn emit_jmp_if_zero(&mut self) -> io::Result<()> {
        let id = self.new_label();
        self.loop_stack.push(id);

        writeln!(self.output, "  ; BF instruction [")?;
        writeln!(self.output, "  ; <start loop> ")?;
        writeln!(self.output, "  br label %loop_check_{id}")?;
        writeln!(self.output, "loop_check_{id}:")?;
        writeln!(self.output, "  %cond_ptr_{id} = load ptr, ptr %ptr")?;
        writeln!(
            self.output,
            "  %cond_val_{id} = load i8, ptr %cond_ptr_{id}"
        )?;
        writeln!(self.output, "  %cond_{id} = icmp eq i8 %cond_val_{id}, 0")?;
        writeln!(
            self.output,
            "  br i1 %cond_{id}, label %loop_end_{id}, label %loop_body_{id}"
        )?;
        writeln!(self.output, "loop_body_{id}:")?;
        writeln!(self.output)?;

        Ok(())
    }

    #[inline]
    fn emit_jmp_if_nonzero(&mut self) -> io::Result<()> {
        // Panic shouldn't happen
        let id = self.loop_stack.pop().expect("unmatched ]");

        writeln!(self.output, "  ; BF instruction ]")?;
        writeln!(self.output, "  br label %loop_check_{id}")?;
        writeln!(self.output, "loop_end_{id}:")?;
        writeln!(self.output, "  ; <end loop> ")?;
        writeln!(self.output)?;

        Ok(())
    }

    fn emit_instrs(&mut self, instrs: &[Instr]) -> io::Result<()> {
        for instr in instrs {
            match instr {
                Instr::AddPtr(n) => self.emit_add_ptr(*n)?,
                Instr::SubPtr(n) => self.emit_sub_ptr(*n)?,
                Instr::AddByte(n) => self.emit_add_byte(*n)?,
                Instr::SubByte(n) => self.emit_sub_byte(*n)?,
                Instr::SetByte(n) => self.emit_set_byte(*n)?,
                Instr::Output => self.emit_output()?,
                Instr::Input => self.emit_input()?,
                Instr::MoveAdd(offset) => self.emit_move_add(*offset)?,
                Instr::JmpIfZero(_) => self.emit_jmp_if_zero()?,
                Instr::JmpIfNonzero(_) => self.emit_jmp_if_nonzero()?,
            }
        }
        Ok(())
    }
}
