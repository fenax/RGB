use cpu::*;

pub struct Joypad{
    p14  : bool,
    p15  : bool,
    up   : bool,
    down : bool,
    right: bool,
    left : bool,
    a    : bool,
    b    : bool,
    start: bool,
    select:bool,
}
impl Joypad{
    pub fn origin() -> Joypad{
        Joypad{
            p14 : true,
            p15 : true,

            up : false,
            down : false,
            right:false,
            left :false,
            a    :false,
            b    :false,
            start:false,
            select:false,
        }
    }
    /*           P14        P15
                  |          |
        P10-------O-Right----O-A
                  |          |
        P11-------O-Left-----O-B
                  |          |
        P12-------O-Up-------O-Select
                  |          |
        P13-------O-Down-----O-Start
                  |          |*/
    pub fn write(&mut self,v :u8){
        self.p14 = (v & (1<<4)) != 0;
        self.p15 = (v & (1<<5)) != 0;
    }
    pub fn read(&self)->u8{
        // unsure, assuming out port is out only.
        let mut r = 0;
        
        if(!self.p14){
            r|= (self.right as u8) << 0;
            r|= (self.left  as u8) << 1;
            r|= (self.up    as u8) << 2;
            r|= (self.down  as u8) << 3;
        }
        if(!self.p15){
            r|= (self.a     as u8) << 0;
            r|= (self.b     as u8) << 1;
            r|= (self.select as u8)<< 2;
            r|= (self.start as u8) << 3;
        }
        r
    }
    pub fn step(&self,clock:u32)->interrupt::Interrupt{
        interrupt::Interrupt::None
    }
}

pub struct Serial{
    start : bool,
    started:bool,
    internal_clock : bool,
    stoptime:u32,
    data : u8,
}
impl Serial{ 
    pub fn origin() -> Serial{
        Serial{
            start : false,
            started:false,
            internal_clock : false,
            data : 0,
            stoptime :0,
        }
    }
    pub fn write_data(&mut self,v :u8){
        println!("Serial {:02x} {}",v,v as char);
        self.data = v;
    }
    pub fn read_data(&self) -> u8{
        self.data
    }
    pub fn write_control(&mut self,v :u8){
        self.start = (v & (1<<7)) != 0;
        self.internal_clock = (v & 1) != 0;
    }
    pub fn read_control(&self)->u8{
        let mut r = 0;
        r |= self.internal_clock as u8;
        r |= (self.start as u8) << 7;
        r
    }
    pub fn step(&mut self ,clock:u32)->interrupt::Interrupt{
        if(self.started){
            if(clock==self.stoptime){
                self.start = false;
                self.started=false;
                self.data = 0xff;
                interrupt::Interrupt::SerialTransfer
            }else{
                interrupt::Interrupt::None
            }
        }else{
            if (self.start){
                self.started = true;
                self.stoptime = clock + 1024;
            } 
            interrupt::Interrupt::None
        }
    }

}

pub struct Dma{
    pub address : u8,
    pub todo : bool,
    pub doing : bool,
    stoptime : u32,
}
impl Dma{
    pub fn origin() -> Dma{
        Dma{
            address : 0,
            todo : false,
            doing : false,
            stoptime: 0,
        }
    }
    pub fn write(&mut self,v : u8){
        self.address = v ;
        self.todo = true;
    }
    pub fn read(&self)->u8{
        self.address
    }
    pub fn step(&mut self, clock:u32)->interrupt::Interrupt{
        if(self.doing){
            if(clock==self.stoptime){
                self.todo = false;
                self.doing = false;
                interrupt::Interrupt::DoDmaTransfer
            }else{ interrupt::Interrupt::None }
        }else{
            if self.todo {
                self.doing = true;
                self.stoptime = clock + 128;
            }
            interrupt::Interrupt::None
        }
        
    }
}

pub struct Timer{
    div : u8, 
    tima: u8,
    tma : u8,
    start : bool,
    div_sel : u8,
}
impl Timer{
    pub fn origin() -> Timer{
        // 00: 4.096 KHz    (~4.194 KHz SGB)  /256
        // 01: 262.144 Khz  (~268.4 KHz SGB)  /4
        // 10: 65.536 KHz   (~67.11 KHz SGB)  /16
        // 11: 16.384 KHz   (~16.78 KHz SGB)  /64
        Timer{
            div : 0,
            tima: 0,
            tma : 0,
            div_sel: 0,
            start:false,
        }
    }
    pub fn write_div(&mut self,v: u8 ){
        self.div = 0;
    }
    pub fn write_tima(&mut self,v: u8){
        self.tima = v;
    }
    pub fn write_tma(&mut self, v: u8){
        self.tma = v;
    }
    pub fn write_control(&mut self, v: u8){
        self.div_sel = v& 0x3;
        self.start = v& 0x4 != 0;
    }
    pub fn read_div(&self)->u8{
        self.div
    }
    pub fn read_tima(&self)->u8{
        self.tima
    }
    pub fn read_tma(&self)->u8{
        self.tma
    }
    pub fn read_control(&self)->u8{
        self.div_sel | ((self.start as u8)<<2) 
    }
    pub fn step(&mut self,clock :u32)->interrupt::Interrupt{
        if(clock & 63 == 0){
            self.div = self.div.wrapping_add(1);
        }
        if(self.start && 
           (clock & match self.div_sel {
               0=> 255,1=>3,2=>15,3=>63,_=>panic!()}) == 2){
               let (r,o) = self.tima.overflowing_add(1);
               if o {
                   self.tima = self.tma;
                   interrupt::Interrupt::TimerOverflow
               }else{
                   self.tima = r;
                   interrupt::Interrupt::None
               }
           }else{
               interrupt::Interrupt::None
           }
    }
}