use std::io;
use std::io::prelude::*;
use std::fs::File;


mod cpu;

use cpu::*;

struct Gameboy{
    ram :Ram,
    reg :Registers,
    alu :Alu,
}
impl Gameboy{
    fn Origin() -> Gameboy{
        let mut r = Gameboy{
            ram : Ram::origin(),
            reg : Registers::origin(),
            alu : Alu::origin()
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
           if clock > 100000 {break}

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
