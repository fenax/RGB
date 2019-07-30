use std::io;
use std::io::prelude::*;
use std::fs::File;


mod cpu;

use cpu::*;

struct Gameboy{
    ram :cpu::ram::Ram,
    reg :cpu::registers::Registers,
    alu :cpu::alu::Alu,
}
impl Gameboy{
    fn Origin() -> Gameboy{
        let mut r = Gameboy{
            ram : cpu::ram::Ram::origin(),
            reg : cpu::registers::Registers::origin(),
            alu : cpu::alu::Alu::origin()
        };
        r.reg.PC=0x100;
        r
    }
    fn main_loop(&mut self)
    {
        let mut clock = 0 as u32;
        let mut cpu_wait = 0;
        loop {
  //          clock += 1;
           if cpu_wait == 0{
               clock += 1;
               cpu_wait =
                   instruct(&mut self.ram,
                            &mut self.reg,
                            &mut self.alu)
                   .unwrap_or(0);
      //         println!("{}{}",self.alu,self.reg);
           }else{
               cpu_wait -= 1;
           }
           if clock > 100000000 {break}

        }
    }
}
fn main() -> io::Result<()>{
    let mut gb = Gameboy::Origin();

    let mut f = File::open("test.gb")?;

    // read exactly 10 bytes
    f.read_exact(&mut gb.ram.rom)?;
    f.read_exact(&mut gb.ram.romswitch)?;
    gb.main_loop();
    Ok(())

    
}
