use std::fmt;
use cpu::*;
use cpu::micro_instructions::*;
use cpu::alu::Alu;

pub trait LoadStore<R,I>{
    fn load(&self,&R)->I;
    fn store(&mut self,&R,I); 
}

pub struct Registers {
    pub a: u8,
    pub f: Alu,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl fmt::Display for Registers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "A:{:02x} BC:{:02x}{:02x} DE:{:02x}{:02x} HL:{:02x}{:02x} SP:{:04x} PC:{:04x}",
            self.a, self.b, self.c, self.d, self.e, self.h, self.l, self.sp, self.pc
        )
    }
}

impl Registers {
    pub fn origin() -> Registers {
        Registers {
            a: 0,
            f:Alu::default(),
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }
}
impl LoadStore<WordRegister,u8> for Registers{
    fn load(&self,r:&WordRegister)->u8{
        match r{
            A=>self.a,
            B=>self.b,
            C=>self.c,
            D=>self.d,
            E=>self.e,
            H=>self.h,
            L=>self.l,
        }
    }
    fn store(&mut self,r:&WordRegister,v:u8){
        match r{
            A=>self.a = v,
            B=>self.b = v,
            C=>self.c = v,
            D=>self.d = v,
            E=>self.e = v,
            H=>self.h = v,
            L=>self.l = v,
        }
    }
}
impl LoadStore<DoubleRegister,u16> for Registers{
    fn load(&self,r:&DoubleRegister)->u16{
        match r{
            AF=>u8tou16(self.f.get_f(), self.a),
            BC=>u8tou16(self.c, self.b),
            DE=>u8tou16(self.e, self.d),
            HL=>u8tou16(self.l, self.h),
            SP=>self.sp,
            PC=>self.pc,
        }
    }
    fn store(&mut self,r:&DoubleRegister,v:u16){
        let (l,h) = u16tou8(v);
        match r{
            AF=>{self.f.set_f(l); self.a = h},
            BC=>{self.c = l; self.b = h},
            DE=>{self.e = l; self.d = h},
            HL=>{self.l = l; self.h = h},
            SP=>self.sp = v,
            PC=>self.pc = v,
        }
    }
}
