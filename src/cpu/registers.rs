use std::fmt;

pub struct Registers {
    pub a:u8,
    pub b:u8,
    pub c:u8,
    pub d:u8,
    pub e:u8,
    pub h:u8,
    pub l:u8,
    pub sp:u16,
    pub pc:u16,
}

impl fmt::Display for Registers{
    fn fmt(&self, f: &mut fmt::Formatter<'_>)        -> fmt::Result
    {
        write!(f, "A:{:02x} BC:{:02x}{:02x} DE:{:02x}{:02x} HL:{:02x}{:02x} SP:{:04x} PC:{:04x}",
               self.a,self.b,self.c,
               self.d,self.e,self.h,
               self.l,self.sp,self.pc)
    }
}

impl Registers{
    pub fn origin() -> Registers{
        Registers{
            a:0,
            b:0,
            c:0,
            d:0,
            e:0,
            h:0,
            l:0,
            sp:0,
            pc:0
        }
    }
}
