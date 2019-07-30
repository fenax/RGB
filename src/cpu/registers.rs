use std::fmt;
use cpu::*;

pub struct Registers {
    pub A:u8,
    pub B:u8,
    pub C:u8,
    pub D:u8,
    pub E:u8,
    pub H:u8,
    pub L:u8,
    pub SP:u16,
    pub PC:u16,
}

impl fmt::Display for Registers{
    fn fmt(&self, f: &mut fmt::Formatter<'_>)        -> fmt::Result
    {
        write!(f, "A:{:02x} BC:{:02x}{:02x} DE:{:02x}{:02x} HL:{:02x}{:02x} SP:{:04x} PC:{:04x}",
               self.A,self.B,self.C,
               self.D,self.E,self.H,
               self.L,self.SP,self.PC)
    }
}

impl Registers{
    pub fn origin() -> Registers{
        Registers{
            A:0,
            B:0,
            C:0,
            D:0,
            E:0,
            H:0,
            L:0,
            SP:0,
            PC:0
        }
    }
}
