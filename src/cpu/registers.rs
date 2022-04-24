use defmt::Format;
use defmt::intern;

pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Format for Registers {
    fn format(&self, _fmt: defmt::Formatter) {
        let t = intern!("A:{:02x} BC:{:02x}{:02x} DE:{:02x}{:02x} HL:{:02x}{:02x} SP:{:04x} PC:{:04x}");
        defmt::export::istr(&t);
        defmt::export::u8( &self.a);
        defmt::export::u8( &self.b);
        defmt::export::u8( &self.c);
        defmt::export::u8( &self.d);
        defmt::export::u8( &self.e);
        defmt::export::u8( &self.h);
        defmt::export::u8( &self.l);
        defmt::export::u16(&self.sp);
        defmt::export::u16(&self.pc);
    }
}

impl Registers {
    pub fn origin() -> Registers {
        Registers {
            a: 0,
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
