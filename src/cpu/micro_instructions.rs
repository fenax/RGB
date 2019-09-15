use cpu::*;
use cpu::ram::*;
use cpu::alu::*;
use cpu::registers::*;
use std::fmt;


#[derive(Debug)]
pub enum WordRegister{
    A,
    B,
    C,
    D,
    E,
    H,
    L,
}
impl fmt::Display for WordRegister{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{
        write!(f, "{}",
        match self{
            WordRegister::A=>"A",
            WordRegister::B=>"B",
            WordRegister::C=>"C",
            WordRegister::D=>"D",
            WordRegister::E=>"E",
            WordRegister::H=>"H",
            WordRegister::L=>"L",
        })
    }
}
#[derive(Debug)]
pub enum DoubleRegister{
    AF,
    BC,
    DE,
    HL,
    PC,
    SP,
}
impl fmt::Display for DoubleRegister{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{
        write!(f, "{}",
        match self{
            DoubleRegister::AF=>"AF",
            DoubleRegister::BC=>"BC",
            DoubleRegister::DE=>"DE",
            DoubleRegister::HL=>"HL",
            DoubleRegister::PC=>"PC",
            DoubleRegister::SP=>"SP",
        })
    }
}

trait Instruction: fmt::Display{
    fn run(&self,&mut Registers, &mut Ram);
}

pub struct InstructionLD{
    from:WordRegister,
    to:WordRegister,
}
impl fmt::Display for InstructionLD{
    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result{
        write!(f, "LD {}, {}", self.to, self.from)
    }
}
impl Instruction for InstructionLD{
    fn run(&self,reg: &mut Registers,ram: &mut Ram){
        let v = reg.load(&self.from);
        reg.store(&self.to,v);
    }
}

pub struct InstructionADD{
    from:WordRegister,
}

impl fmt::Display for InstructionADD{
    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result{
        write!(f, "ADD A, {}", self.from)
    }
}

impl Instruction for InstructionADD{
    fn run(&self, reg:&mut Registers, ram :&mut Ram){
        let a = reg.load(&WordRegister::A);
        let b = reg.load(&self.from);
        let (o,c) = a.overflowing_add(b);
        reg.store(&WordRegister::A,o);
        let half = ((a & 0xf) + (b & 0xf)) > 0xf;
        reg.f.set_flags(o==0,false,half,c);
    }
}

pub struct InstructionADC{
    from:WordRegister,
} 

impl fmt::Display for InstructionADC{
    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result{
        write!(f, "ADC A, {}", self.from)
    }
}

impl Instruction for InstructionADC{
    fn run(&self, reg:&mut Registers, ram:&mut Ram){
        let a = reg.load(&WordRegister::A);
        let b = reg.load(&self.from);
        let (r, c1) = a.overflowing_add(b);
        let (o, c2) = r.overflowing_add(if reg.f.flag_carry{1}else{0});
        let half = ((a & 0xf) + ((b & 0xf) + 1)) > 0xf;
        reg.f.set_flags(o==0,false,half,c1||c2);
    }
}

pub struct InstructionSUB{
    from:WordRegister,
}

impl fmt::Display for InstructionSUB{
    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result{
        write!(f, "SUB A, {}", self.from)
    }
}

impl Instruction for InstructionSUB{
    fn run(&self, reg:&mut Registers, ram:&mut Ram){
        let a = reg.load(&WordRegister::A);
        let b = reg.load(&self.from);
        let (o, c) = a.overflowing_sub(b);
        let halfcarry = *a & 0xf < b & 0xf;
        reg.f.set_flags(o==0,true,halfcarry,c);
    }
}

pub struct 