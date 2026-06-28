/// Opcode decode stubs. Full implementation in WI-M1.
///
/// Each variant names the instruction mnemonic and its addressing mode.
#[derive(Debug, Clone, Copy)]
pub enum Opcode {
    // Load/Store
    Lda, Ldx, Ldy,
    Sta, Stx, Sty,

    // Arithmetic
    Adc, Sbc,
    Inc, Dec,
    Inx, Dex,
    Iny, Dey,

    // Logic
    And, Ora, Eor,
    Bit,

    // Shift/Rotate
    Asl, Lsr,
    Rol, Ror,

    // Comparison
    Cmp, Cpx, Cpy,

    // Branch
    Bcc, Bcs, Beq, Bne,
    Bmi, Bpl, Bvs, Bvc,

    // Jump/Call
    Jmp, Jsr, Rts,

    // Stack
    Pha, Pla,
    Php, Plp,

    // Transfer
    Tax, Txa,
    Tay, Tya,
    Tsx, Txs,

    // Flag
    Sec, Clc,
    Sei, Cli,
    Sed, Cld,
    Clv,

    // Interrupt
    Brk, Rti,
    Nop,

    // Illegal/unknown — treated as NOP for now
    Ill,
}

/// Decode a raw opcode byte into an `Opcode` variant.
/// Placeholder: returns `Ill` for all values until WI-M1 provides the full table.
pub fn decode(_byte: u8) -> Opcode {
    Opcode::Ill
}
