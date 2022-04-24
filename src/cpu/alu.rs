use crate::cpu::*;
use defmt::intern;
use defmt::Format;

pub struct Alu {
    pub flag_zero: bool,
    pub flag_substract: bool,
    pub flag_halfcarry: bool,
    pub flag_carry: bool,
}

impl Format for Alu {
    fn format(&self, _fmt: defmt::Formatter) {
        let t = intern!("{=char}{=char}{=char}{=char}");
        defmt::export::istr(&t);
        defmt::export::char(&if self.flag_zero { 'Z' } else { '-' });
        defmt::export::char(&if self.flag_substract { 'S' } else { '-' });
        defmt::export::char(&if self.flag_halfcarry { 'H' } else { '-' });
        defmt::export::char(&if self.flag_carry { 'C' } else { '-' });
    }
}

impl Alu {
    pub fn origin() -> Alu {
        Alu {
            flag_zero: false,
            flag_substract: false,
            flag_halfcarry: false,
            flag_carry: false,
        }
    }
    pub fn get_f(&self) -> u8 {
        let mut r = 0 as u8;
        if self.flag_zero {
            r += 1 << 7
        };
        if self.flag_substract {
            r += 1 << 6
        };
        if self.flag_halfcarry {
            r += 1 << 5
        };
        if self.flag_carry {
            r += 1 << 4
        };
        r
    }
    pub fn set_f(&mut self, f: u8) {
        self.flag_zero = (f & 1 << 7) != 0;
        self.flag_substract = (f & 1 << 6) != 0;
        self.flag_halfcarry = (f & 1 << 5) != 0;
        self.flag_carry = (f & 1 << 4) != 0;
    }
    pub fn set_flags(&mut self, z: bool, s: bool, h: bool, c: bool) {
        self.flag_zero = z;
        self.flag_substract = s;
        self.flag_halfcarry = h;
        self.flag_carry = c;
    }
    pub fn and(&mut self, a: &mut u8, b: u8) -> CpuState {
        *a = *a & b;
        self.set_flags(*a == 0, false, true, false);
        CpuState::None
    }
    pub fn or(&mut self, a: &mut u8, b: u8) -> CpuState {
        *a = *a | b;
        self.set_flags(*a == 0, false, false, false);
        CpuState::None
    }
    pub fn xor(&mut self, a: &mut u8, b: u8) -> CpuState {
        *a = *a ^ b;
        self.set_flags(*a == 0, false, false, false);
        CpuState::None
    }
    pub fn add16(&mut self, l: &mut u8, h: &mut u8, b: u16) -> CpuState {
        let reg_hl = u8tou16(*l, *h);
        let (rl, rh) = u16tou8(self.add16_(reg_hl, b));
        *h = rh;
        *l = rl;
        CpuState::Wait(1)
    }
    pub fn add16_(&mut self, a: u16, b: u16) -> u16 {
        self.flag_halfcarry = ((a & 0xfff) + (b & 0xfff)) > 0xfff;
        self.flag_substract = false;
        let (r, c) = a.overflowing_add(b);
        self.flag_carry = c;
        r
    }
    pub fn add(&mut self, a: &mut u8, b: u8) -> CpuState {
        self.flag_halfcarry = ((*a & 0xf) + (b & 0xf)) > 0xf;
        self.flag_substract = false;
        let (r, c) = a.overflowing_add(b);
        self.flag_zero = r == 0;
        self.flag_carry = c;
        *a = r;
        CpuState::None
    }
    pub fn sub(&mut self, a: &mut u8, b: u8) -> CpuState {
        self.flag_halfcarry = *a & 0xf < b & 0xf;
        self.flag_substract = true;
        let (r, c) = a.overflowing_sub(b);
        self.flag_zero = r == 0;
        self.flag_carry = c;
        *a = r;
        CpuState::None
    }
    pub fn cp(&mut self, a: u8, b: u8) -> CpuState {
        self.flag_halfcarry = a & 0xf < b & 0xf;
        self.flag_substract = true;
        let (r, c) = a.overflowing_sub(b);
        self.flag_zero = r == 0;
        self.flag_carry = c;
        CpuState::None
    }
    pub fn adc(&mut self, a: &mut u8, b: u8) -> CpuState {
        if self.flag_carry {
            self.flag_halfcarry = ((*a & 0xf) + ((b & 0xf) + 1)) > 0xf;
            self.flag_substract = false;
            let (r1, c1) = a.overflowing_add(b);
            let (r, c2) = r1.overflowing_add(1);
            self.flag_zero = r == 0;
            self.flag_carry = c1 || c2;
            *a = r;
            CpuState::None
        } else {
            self.add(a, b)
        }
    }
    pub fn sbc(&mut self, a: &mut u8, b: u8) -> CpuState {
        if self.flag_carry {
            self.flag_halfcarry = *a & 0xf <= b & 0xf;
            self.flag_substract = true;
            let (r1, c1) = a.overflowing_sub(b);
            let (r, c2) = r1.overflowing_sub(1);
            self.flag_zero = r == 0;
            self.flag_carry = c1 || c2;
            *a = r;
            CpuState::None
        } else {
            self.sub(a, b)
        }
    }
    pub fn inc(&mut self, a: &mut u8) -> CpuState {
        *a = a.wrapping_add(1);
        self.flag_halfcarry = (*a & 0xf) == 0;
        self.flag_substract = false;
        self.flag_zero = *a == 0;
        CpuState::None
    }
    pub fn dec(&mut self, a: &mut u8) -> CpuState {
        *a = a.wrapping_sub(1);
        self.flag_halfcarry = (*a & 0xf) == 0xf;
        self.flag_substract = true;
        self.flag_zero = *a == 0;
        CpuState::None
    }
    pub fn inc16(&self, l: &mut u8, h: &mut u8) -> CpuState {
        let mut r = u8tou16(*l, *h);
        r = r.wrapping_add(1);
        let (rl, rh) = u16tou8(r);
        *l = rl;
        *h = rh;
        CpuState::Wait(1)
    }
    pub fn dec16(&self, l: &mut u8, h: &mut u8) -> CpuState {
        let mut r = u8tou16(*l, *h);
        r = r.wrapping_sub(1);
        let (rl, rh) = u16tou8(r);
        *l = rl;
        *h = rh;
        CpuState::Wait(1)
    }
}
