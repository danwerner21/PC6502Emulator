use super::flags;
use super::Cpu;

// ── Fetch helpers ──────────────────────────────────────────────────────────────

#[inline]
fn fetch<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> u8 {
    let b = read(cpu.pc);
    cpu.pc = cpu.pc.wrapping_add(1);
    b
}

#[inline]
fn fetch_word<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> u16 {
    let lo = fetch(cpu, read) as u16;
    let hi = fetch(cpu, read) as u16;
    (hi << 8) | lo
}

// ── Addressing modes → (effective_address, page_cross_penalty) ────────────────

#[inline]
fn mode_zp<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> (u16, u32) {
    (fetch(cpu, read) as u16, 0)
}

#[inline]
fn mode_zpx<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> (u16, u32) {
    let b = fetch(cpu, read);
    (b.wrapping_add(cpu.x) as u16, 0)
}

#[inline]
fn mode_zpy<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> (u16, u32) {
    let b = fetch(cpu, read);
    (b.wrapping_add(cpu.y) as u16, 0)
}

#[inline]
fn mode_abs<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> (u16, u32) {
    (fetch_word(cpu, read), 0)
}

#[inline]
fn mode_absx<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> (u16, u32) {
    let base = fetch_word(cpu, read);
    let addr = base.wrapping_add(cpu.x as u16);
    let p = if (base & 0xFF00) != (addr & 0xFF00) { 1 } else { 0 };
    (addr, p)
}

#[inline]
fn mode_absy<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> (u16, u32) {
    let base = fetch_word(cpu, read);
    let addr = base.wrapping_add(cpu.y as u16);
    let p = if (base & 0xFF00) != (addr & 0xFF00) { 1 } else { 0 };
    (addr, p)
}

#[inline]
fn mode_indx<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> (u16, u32) {
    let zp = fetch(cpu, read).wrapping_add(cpu.x);
    let lo = read(zp as u16) as u16;
    let hi = read(zp.wrapping_add(1) as u16) as u16;
    ((hi << 8) | lo, 0)
}

#[inline]
fn mode_indy<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> (u16, u32) {
    let zp = fetch(cpu, read);
    let lo = read(zp as u16) as u16;
    let hi = read(zp.wrapping_add(1) as u16) as u16;
    let base = (hi << 8) | lo;
    let addr = base.wrapping_add(cpu.y as u16);
    let p = if (base & 0xFF00) != (addr & 0xFF00) { 1 } else { 0 };
    (addr, p)
}

// ── Stack helpers ──────────────────────────────────────────────────────────────

#[inline]
fn push<W: FnMut(u16, u8)>(cpu: &mut Cpu, write: &mut W, val: u8) {
    write(0x0100 | cpu.sp as u16, val);
    cpu.sp = cpu.sp.wrapping_sub(1);
}

#[inline]
fn pull<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R) -> u8 {
    cpu.sp = cpu.sp.wrapping_add(1);
    read(0x0100 | cpu.sp as u16)
}

// ── ALU helpers ────────────────────────────────────────────────────────────────

#[inline]
fn op_lda(cpu: &mut Cpu, val: u8) {
    cpu.a = val;
    cpu.p.set_nz(val);
}
#[inline]
fn op_ldx(cpu: &mut Cpu, val: u8) {
    cpu.x = val;
    cpu.p.set_nz(val);
}
#[inline]
fn op_ldy(cpu: &mut Cpu, val: u8) {
    cpu.y = val;
    cpu.p.set_nz(val);
}

#[inline]
fn op_adc(cpu: &mut Cpu, val: u8) {
    let carry = cpu.p.get(flags::C) as u16;
    let a = cpu.a as u16;
    let v = val as u16;
    let result = a + v + carry;
    cpu.p.set(flags::C, result > 0xFF);
    cpu.p.set(flags::V, (!(a ^ v) & (a ^ result) & 0x80) != 0);
    let r = result as u8;
    cpu.p.set_nz(r);
    cpu.a = r;
}

#[inline]
fn op_sbc(cpu: &mut Cpu, val: u8) {
    op_adc(cpu, !val);
}

#[inline]
fn op_and(cpu: &mut Cpu, val: u8) {
    cpu.a &= val;
    let a = cpu.a;
    cpu.p.set_nz(a);
}
#[inline]
fn op_ora(cpu: &mut Cpu, val: u8) {
    cpu.a |= val;
    let a = cpu.a;
    cpu.p.set_nz(a);
}
#[inline]
fn op_eor(cpu: &mut Cpu, val: u8) {
    cpu.a ^= val;
    let a = cpu.a;
    cpu.p.set_nz(a);
}

#[inline]
fn op_bit(cpu: &mut Cpu, val: u8) {
    cpu.p.set(flags::N, val & 0x80 != 0);
    cpu.p.set(flags::V, val & 0x40 != 0);
    cpu.p.set(flags::Z, cpu.a & val == 0);
}

#[inline]
fn op_cmp(cpu: &mut Cpu, reg: u8, val: u8) {
    let r = reg.wrapping_sub(val);
    cpu.p.set(flags::C, reg >= val);
    cpu.p.set_nz(r);
}

#[inline]
fn op_asl(cpu: &mut Cpu, val: u8) -> u8 {
    cpu.p.set(flags::C, val & 0x80 != 0);
    let r = val << 1;
    cpu.p.set_nz(r);
    r
}
#[inline]
fn op_lsr(cpu: &mut Cpu, val: u8) -> u8 {
    cpu.p.set(flags::C, val & 0x01 != 0);
    let r = val >> 1;
    cpu.p.set_nz(r);
    r
}
#[inline]
fn op_rol(cpu: &mut Cpu, val: u8) -> u8 {
    let c = cpu.p.get(flags::C) as u8;
    cpu.p.set(flags::C, val & 0x80 != 0);
    let r = (val << 1) | c;
    cpu.p.set_nz(r);
    r
}
#[inline]
fn op_ror(cpu: &mut Cpu, val: u8) -> u8 {
    let c = cpu.p.get(flags::C) as u8;
    cpu.p.set(flags::C, val & 0x01 != 0);
    let r = (val >> 1) | (c << 7);
    cpu.p.set_nz(r);
    r
}

#[inline]
fn branch<R: FnMut(u16) -> u8>(cpu: &mut Cpu, read: &mut R, cond: bool) -> u32 {
    let offset = fetch(cpu, read) as i8 as i16;
    if cond {
        let old = cpu.pc;
        cpu.pc = ((cpu.pc as i16).wrapping_add(offset)) as u16;
        if (old & 0xFF00) != (cpu.pc & 0xFF00) { 2 } else { 1 }
    } else {
        0
    }
}

// ── Main dispatch ──────────────────────────────────────────────────────────────

/// Execute one instruction.  Returns cycles consumed.
pub fn execute<R, W>(cpu: &mut Cpu, read: &mut R, write: &mut W) -> u32
where
    R: FnMut(u16) -> u8,
    W: FnMut(u16, u8),
{
    let opc = fetch(cpu, read);
    match opc {
        // ── LDA ──
        0xA9 => { let v = fetch(cpu, read);                  op_lda(cpu, v); 2 }
        0xA5 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_lda(cpu, v); 3 }
        0xB5 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); op_lda(cpu, v); 4 }
        0xAD => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_lda(cpu, v); 4 }
        0xBD => { let (a,p) = mode_absx(cpu, read);let v = read(a); op_lda(cpu, v); 4+p }
        0xB9 => { let (a,p) = mode_absy(cpu, read);let v = read(a); op_lda(cpu, v); 4+p }
        0xA1 => { let (a,_) = mode_indx(cpu, read);let v = read(a); op_lda(cpu, v); 6 }
        0xB1 => { let (a,p) = mode_indy(cpu, read);let v = read(a); op_lda(cpu, v); 5+p }
        // ── LDX ──
        0xA2 => { let v = fetch(cpu, read);                  op_ldx(cpu, v); 2 }
        0xA6 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_ldx(cpu, v); 3 }
        0xB6 => { let (a,_) = mode_zpy(cpu, read); let v = read(a); op_ldx(cpu, v); 4 }
        0xAE => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_ldx(cpu, v); 4 }
        0xBE => { let (a,p) = mode_absy(cpu, read);let v = read(a); op_ldx(cpu, v); 4+p }
        // ── LDY ──
        0xA0 => { let v = fetch(cpu, read);                  op_ldy(cpu, v); 2 }
        0xA4 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_ldy(cpu, v); 3 }
        0xB4 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); op_ldy(cpu, v); 4 }
        0xAC => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_ldy(cpu, v); 4 }
        0xBC => { let (a,p) = mode_absx(cpu, read);let v = read(a); op_ldy(cpu, v); 4+p }
        // ── STA ──
        0x85 => { let (a,_) = mode_zp(cpu, read);  write(a, cpu.a); 3 }
        0x95 => { let (a,_) = mode_zpx(cpu, read); write(a, cpu.a); 4 }
        0x8D => { let (a,_) = mode_abs(cpu, read); write(a, cpu.a); 4 }
        0x9D => { let (a,_) = mode_absx(cpu, read);write(a, cpu.a); 5 }  // no page penalty on write
        0x99 => { let (a,_) = mode_absy(cpu, read);write(a, cpu.a); 5 }
        0x81 => { let (a,_) = mode_indx(cpu, read);write(a, cpu.a); 6 }
        0x91 => { let (a,_) = mode_indy(cpu, read);write(a, cpu.a); 6 }
        // ── STX ──
        0x86 => { let (a,_) = mode_zp(cpu, read);  write(a, cpu.x); 3 }
        0x96 => { let (a,_) = mode_zpy(cpu, read); write(a, cpu.x); 4 }
        0x8E => { let (a,_) = mode_abs(cpu, read); write(a, cpu.x); 4 }
        // ── STY ──
        0x84 => { let (a,_) = mode_zp(cpu, read);  write(a, cpu.y); 3 }
        0x94 => { let (a,_) = mode_zpx(cpu, read); write(a, cpu.y); 4 }
        0x8C => { let (a,_) = mode_abs(cpu, read); write(a, cpu.y); 4 }
        // ── ADC ──
        0x69 => { let v = fetch(cpu, read);                  op_adc(cpu, v); 2 }
        0x65 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_adc(cpu, v); 3 }
        0x75 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); op_adc(cpu, v); 4 }
        0x6D => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_adc(cpu, v); 4 }
        0x7D => { let (a,p) = mode_absx(cpu, read);let v = read(a); op_adc(cpu, v); 4+p }
        0x79 => { let (a,p) = mode_absy(cpu, read);let v = read(a); op_adc(cpu, v); 4+p }
        0x61 => { let (a,_) = mode_indx(cpu, read);let v = read(a); op_adc(cpu, v); 6 }
        0x71 => { let (a,p) = mode_indy(cpu, read);let v = read(a); op_adc(cpu, v); 5+p }
        // ── SBC ──
        0xE9 => { let v = fetch(cpu, read);                  op_sbc(cpu, v); 2 }
        0xE5 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_sbc(cpu, v); 3 }
        0xF5 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); op_sbc(cpu, v); 4 }
        0xED => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_sbc(cpu, v); 4 }
        0xFD => { let (a,p) = mode_absx(cpu, read);let v = read(a); op_sbc(cpu, v); 4+p }
        0xF9 => { let (a,p) = mode_absy(cpu, read);let v = read(a); op_sbc(cpu, v); 4+p }
        0xE1 => { let (a,_) = mode_indx(cpu, read);let v = read(a); op_sbc(cpu, v); 6 }
        0xF1 => { let (a,p) = mode_indy(cpu, read);let v = read(a); op_sbc(cpu, v); 5+p }
        // ── AND ──
        0x29 => { let v = fetch(cpu, read);                  op_and(cpu, v); 2 }
        0x25 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_and(cpu, v); 3 }
        0x35 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); op_and(cpu, v); 4 }
        0x2D => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_and(cpu, v); 4 }
        0x3D => { let (a,p) = mode_absx(cpu, read);let v = read(a); op_and(cpu, v); 4+p }
        0x39 => { let (a,p) = mode_absy(cpu, read);let v = read(a); op_and(cpu, v); 4+p }
        0x21 => { let (a,_) = mode_indx(cpu, read);let v = read(a); op_and(cpu, v); 6 }
        0x31 => { let (a,p) = mode_indy(cpu, read);let v = read(a); op_and(cpu, v); 5+p }
        // ── ORA ──
        0x09 => { let v = fetch(cpu, read);                  op_ora(cpu, v); 2 }
        0x05 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_ora(cpu, v); 3 }
        0x15 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); op_ora(cpu, v); 4 }
        0x0D => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_ora(cpu, v); 4 }
        0x1D => { let (a,p) = mode_absx(cpu, read);let v = read(a); op_ora(cpu, v); 4+p }
        0x19 => { let (a,p) = mode_absy(cpu, read);let v = read(a); op_ora(cpu, v); 4+p }
        0x01 => { let (a,_) = mode_indx(cpu, read);let v = read(a); op_ora(cpu, v); 6 }
        0x11 => { let (a,p) = mode_indy(cpu, read);let v = read(a); op_ora(cpu, v); 5+p }
        // ── EOR ──
        0x49 => { let v = fetch(cpu, read);                  op_eor(cpu, v); 2 }
        0x45 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_eor(cpu, v); 3 }
        0x55 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); op_eor(cpu, v); 4 }
        0x4D => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_eor(cpu, v); 4 }
        0x5D => { let (a,p) = mode_absx(cpu, read);let v = read(a); op_eor(cpu, v); 4+p }
        0x59 => { let (a,p) = mode_absy(cpu, read);let v = read(a); op_eor(cpu, v); 4+p }
        0x41 => { let (a,_) = mode_indx(cpu, read);let v = read(a); op_eor(cpu, v); 6 }
        0x51 => { let (a,p) = mode_indy(cpu, read);let v = read(a); op_eor(cpu, v); 5+p }
        // ── BIT ──
        0x24 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_bit(cpu, v); 3 }
        0x2C => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_bit(cpu, v); 4 }
        // ── CMP ──
        0xC9 => { let v = fetch(cpu, read);                  op_cmp(cpu, cpu.a, v); 2 }
        0xC5 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_cmp(cpu, cpu.a, v); 3 }
        0xD5 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); op_cmp(cpu, cpu.a, v); 4 }
        0xCD => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_cmp(cpu, cpu.a, v); 4 }
        0xDD => { let (a,p) = mode_absx(cpu, read);let v = read(a); op_cmp(cpu, cpu.a, v); 4+p }
        0xD9 => { let (a,p) = mode_absy(cpu, read);let v = read(a); op_cmp(cpu, cpu.a, v); 4+p }
        0xC1 => { let (a,_) = mode_indx(cpu, read);let v = read(a); op_cmp(cpu, cpu.a, v); 6 }
        0xD1 => { let (a,p) = mode_indy(cpu, read);let v = read(a); op_cmp(cpu, cpu.a, v); 5+p }
        // ── CPX ──
        0xE0 => { let v = fetch(cpu, read);                  op_cmp(cpu, cpu.x, v); 2 }
        0xE4 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_cmp(cpu, cpu.x, v); 3 }
        0xEC => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_cmp(cpu, cpu.x, v); 4 }
        // ── CPY ──
        0xC0 => { let v = fetch(cpu, read);                  op_cmp(cpu, cpu.y, v); 2 }
        0xC4 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); op_cmp(cpu, cpu.y, v); 3 }
        0xCC => { let (a,_) = mode_abs(cpu, read); let v = read(a); op_cmp(cpu, cpu.y, v); 4 }
        // ── ASL ──
        0x0A => { let v = cpu.a; cpu.a = op_asl(cpu, v); 2 }
        0x06 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); let r = op_asl(cpu, v); write(a, r); 5 }
        0x16 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); let r = op_asl(cpu, v); write(a, r); 6 }
        0x0E => { let (a,_) = mode_abs(cpu, read); let v = read(a); let r = op_asl(cpu, v); write(a, r); 6 }
        0x1E => { let (a,_) = mode_absx(cpu, read);let v = read(a); let r = op_asl(cpu, v); write(a, r); 7 }
        // ── LSR ──
        0x4A => { let v = cpu.a; cpu.a = op_lsr(cpu, v); 2 }
        0x46 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); let r = op_lsr(cpu, v); write(a, r); 5 }
        0x56 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); let r = op_lsr(cpu, v); write(a, r); 6 }
        0x4E => { let (a,_) = mode_abs(cpu, read); let v = read(a); let r = op_lsr(cpu, v); write(a, r); 6 }
        0x5E => { let (a,_) = mode_absx(cpu, read);let v = read(a); let r = op_lsr(cpu, v); write(a, r); 7 }
        // ── ROL ──
        0x2A => { let v = cpu.a; cpu.a = op_rol(cpu, v); 2 }
        0x26 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); let r = op_rol(cpu, v); write(a, r); 5 }
        0x36 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); let r = op_rol(cpu, v); write(a, r); 6 }
        0x2E => { let (a,_) = mode_abs(cpu, read); let v = read(a); let r = op_rol(cpu, v); write(a, r); 6 }
        0x3E => { let (a,_) = mode_absx(cpu, read);let v = read(a); let r = op_rol(cpu, v); write(a, r); 7 }
        // ── ROR ──
        0x6A => { let v = cpu.a; cpu.a = op_ror(cpu, v); 2 }
        0x66 => { let (a,_) = mode_zp(cpu, read);  let v = read(a); let r = op_ror(cpu, v); write(a, r); 5 }
        0x76 => { let (a,_) = mode_zpx(cpu, read); let v = read(a); let r = op_ror(cpu, v); write(a, r); 6 }
        0x6E => { let (a,_) = mode_abs(cpu, read); let v = read(a); let r = op_ror(cpu, v); write(a, r); 6 }
        0x7E => { let (a,_) = mode_absx(cpu, read);let v = read(a); let r = op_ror(cpu, v); write(a, r); 7 }
        // ── INC ──
        0xE6 => { let (a,_) = mode_zp(cpu, read);  let v = read(a).wrapping_add(1); cpu.p.set_nz(v); write(a, v); 5 }
        0xF6 => { let (a,_) = mode_zpx(cpu, read); let v = read(a).wrapping_add(1); cpu.p.set_nz(v); write(a, v); 6 }
        0xEE => { let (a,_) = mode_abs(cpu, read); let v = read(a).wrapping_add(1); cpu.p.set_nz(v); write(a, v); 6 }
        0xFE => { let (a,_) = mode_absx(cpu, read);let v = read(a).wrapping_add(1); cpu.p.set_nz(v); write(a, v); 7 }
        // ── DEC ──
        0xC6 => { let (a,_) = mode_zp(cpu, read);  let v = read(a).wrapping_sub(1); cpu.p.set_nz(v); write(a, v); 5 }
        0xD6 => { let (a,_) = mode_zpx(cpu, read); let v = read(a).wrapping_sub(1); cpu.p.set_nz(v); write(a, v); 6 }
        0xCE => { let (a,_) = mode_abs(cpu, read); let v = read(a).wrapping_sub(1); cpu.p.set_nz(v); write(a, v); 6 }
        0xDE => { let (a,_) = mode_absx(cpu, read);let v = read(a).wrapping_sub(1); cpu.p.set_nz(v); write(a, v); 7 }
        // ── INX DEX INY DEY ──
        0xE8 => { cpu.x = cpu.x.wrapping_add(1); let v = cpu.x; cpu.p.set_nz(v); 2 }
        0xCA => { cpu.x = cpu.x.wrapping_sub(1); let v = cpu.x; cpu.p.set_nz(v); 2 }
        0xC8 => { cpu.y = cpu.y.wrapping_add(1); let v = cpu.y; cpu.p.set_nz(v); 2 }
        0x88 => { cpu.y = cpu.y.wrapping_sub(1); let v = cpu.y; cpu.p.set_nz(v); 2 }
        // ── Transfer ──
        0xAA => { cpu.x = cpu.a; let v = cpu.x; cpu.p.set_nz(v); 2 }  // TAX
        0x8A => { cpu.a = cpu.x; let v = cpu.a; cpu.p.set_nz(v); 2 }  // TXA
        0xA8 => { cpu.y = cpu.a; let v = cpu.y; cpu.p.set_nz(v); 2 }  // TAY
        0x98 => { cpu.a = cpu.y; let v = cpu.a; cpu.p.set_nz(v); 2 }  // TYA
        0xBA => { cpu.x = cpu.sp; let v = cpu.x; cpu.p.set_nz(v); 2 } // TSX
        0x9A => { cpu.sp = cpu.x; 2 }                                   // TXS (no flags)
        // ── Flag ops ──
        0x38 => { cpu.p.set(flags::C, true);  2 }  // SEC
        0x18 => { cpu.p.set(flags::C, false); 2 }  // CLC
        0x78 => { cpu.p.set(flags::I, true);  2 }  // SEI
        0x58 => { cpu.p.set(flags::I, false); 2 }  // CLI
        0xF8 => { cpu.p.set(flags::D, true);  2 }  // SED
        0xD8 => { cpu.p.set(flags::D, false); 2 }  // CLD
        0xB8 => { cpu.p.set(flags::V, false); 2 }  // CLV
        // ── Branch ──
        0x90 => { let c = !cpu.p.get(flags::C); 2 + branch(cpu, read, c) }  // BCC
        0xB0 => { let c =  cpu.p.get(flags::C); 2 + branch(cpu, read, c) }  // BCS
        0xF0 => { let c =  cpu.p.get(flags::Z); 2 + branch(cpu, read, c) }  // BEQ
        0xD0 => { let c = !cpu.p.get(flags::Z); 2 + branch(cpu, read, c) }  // BNE
        0x30 => { let c =  cpu.p.get(flags::N); 2 + branch(cpu, read, c) }  // BMI
        0x10 => { let c = !cpu.p.get(flags::N); 2 + branch(cpu, read, c) }  // BPL
        0x70 => { let c =  cpu.p.get(flags::V); 2 + branch(cpu, read, c) }  // BVS
        0x50 => { let c = !cpu.p.get(flags::V); 2 + branch(cpu, read, c) }  // BVC
        // ── JMP ──
        0x4C => { cpu.pc = fetch_word(cpu, read); 3 }
        0x6C => {
            // Indirect JMP with 6502 page-wrap bug: high byte reads from same page as low byte
            let ptr = fetch_word(cpu, read);
            let lo = read(ptr) as u16;
            let hi = read((ptr & 0xFF00) | ((ptr.wrapping_add(1)) & 0x00FF)) as u16;
            cpu.pc = (hi << 8) | lo;
            5
        }
        // ── JSR ──
        0x20 => {
            let addr = fetch_word(cpu, read);
            let ret = cpu.pc.wrapping_sub(1);
            push(cpu, write, (ret >> 8) as u8);
            push(cpu, write, (ret & 0xFF) as u8);
            cpu.pc = addr;
            6
        }
        // ── RTS ──
        0x60 => {
            let lo = pull(cpu, read) as u16;
            let hi = pull(cpu, read) as u16;
            cpu.pc = ((hi << 8) | lo).wrapping_add(1);
            6
        }
        // ── BRK ──
        0x00 => {
            let _ = fetch(cpu, read); // skip signature/padding byte (PC now at BRK+2)
            let pc = cpu.pc;
            push(cpu, write, (pc >> 8) as u8);
            push(cpu, write, (pc & 0xFF) as u8);
            let p_pushed = cpu.p.0 | flags::B | flags::U;
            push(cpu, write, p_pushed);
            cpu.p.set(flags::I, true);
            let lo = read(0xFFFE) as u16;
            let hi = read(0xFFFF) as u16;
            cpu.pc = (hi << 8) | lo;
            7
        }
        // ── RTI ──
        0x40 => {
            let p = pull(cpu, read);
            cpu.p.0 = p | flags::U;
            let lo = pull(cpu, read) as u16;
            let hi = pull(cpu, read) as u16;
            cpu.pc = (hi << 8) | lo;
            6
        }
        // ── Stack ──
        0x48 => { let a = cpu.a; push(cpu, write, a); 3 }                            // PHA
        0x68 => { let v = pull(cpu, read); cpu.a = v; cpu.p.set_nz(v); 4 }           // PLA
        0x08 => { let p = cpu.p.0 | flags::B | flags::U; push(cpu, write, p); 3 }   // PHP
        0x28 => { let p = pull(cpu, read); cpu.p.0 = p | flags::U; 4 }               // PLP
        // ── NOP ──
        0xEA => 2,
        // ── Illegal / undocumented: treat as 2-cycle NOP ──
        _ => 2,
    }
}
