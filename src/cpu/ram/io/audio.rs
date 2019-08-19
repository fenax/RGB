use cpu::ram::io::*;
use cpu::*;
use cpu::ram::Ram;

pub struct Square{
    //Frequency = 4194304/(32*(2048-x)) Hz
    frequency:u16,
    last_rise:u32,
    next_change:u32,
    shadow_frequency:u16,
    envelope_volume : u8,
    envelope_add_mode: bool,
    envelope_period : u8,
    trigger : bool,
    must_trigger : bool,
    length_enable : bool,
    duty : u8,
    //Sound Length = (64-t1)*(1/256) seconds
    length_load:u8,
    sweep_period : u8,
    sweep_negate : bool,
    sweep_shift  : u8,
    sweep_enable :bool,
    sweep_timer  : u16,
    enable: bool,
    high:bool,
}

impl Square{
    pub fn origin()->Square{
        Square{
            frequency : 0,
            last_rise:0,
            next_change:0,
            shadow_frequency:0,
            envelope_volume : 15,
            envelope_add_mode : false,
            envelope_period:3,
            trigger : false,
            must_trigger: false,
            high :false,
            length_enable :false,
            duty:2,
            length_load:63,
            sweep_period:0,
            sweep_negate:true,
            sweep_shift:0,
            sweep_enable:false,
            sweep_timer:0,
            enable:false,
        }
    }
    /*
    pub fn next_fall(& self)->u32{
        let freq_timer = (2048 - self.frequency);

        let high_time = match self.duty{
            1 => 2,
            2 => 4,
            3 => 6,
            _ => 1,
        };
        self.last_rise + freq_timer * high_time as u32
    }*/

    pub fn step_frequency(&self)->u32{
        2048 - self.frequency as u32
    }
/*
    pub fn next_rise(& self){
        self.last_rise + (2048 - self.frequency) * 8
    }
*/

    pub fn lenght_decr(&mut self){
        if self.length_enable{
        self.length_load = self.length_load.saturating_sub(1);
        if self.length_load == 0 {
            self.enable = false;
        }
        }
    }

    pub fn step_envelope(&mut self){
        if self.enable == false {return;}
        let t = if(self.envelope_add_mode){
            self.envelope_volume.wrapping_add(1)
        }else{
            self.envelope_volume.wrapping_sub(1)
        };
        if t>=0 && t<=15{
            self.envelope_volume = t;
        }
        if t== 0 {
            self.enable = false
            };
    }
    pub fn step_sweep(&mut self){
        if(self.sweep_enable && self.sweep_period>0){
            self.sweep_period -= 1;
            self.frequency = self.shadow_frequency;
            self.calculate_sweep();
        }
    }
    pub fn calculate_sweep(&mut self){
        let t = self.shadow_frequency >> self.sweep_shift;
        let t = if self.sweep_negate {
            self.shadow_frequency.wrapping_sub(t)
        }else{
            self.shadow_frequency.wrapping_add(t)
        };
        if t<0 || t>2047 {
            self.enable = false;
        }else{
            self.shadow_frequency = t;
        }
    }
    pub fn trigger(&mut self){
        self.must_trigger = true;
    }
    pub fn step(&mut self,clock:u32){
        if self.must_trigger{
            self.last_rise = clock;
            self.shadow_frequency = self.frequency;
            self.sweep_enable = self.sweep_period != 0 || self.sweep_shift != 0;
            self.enable = true;
            self.next_change = clock;
            self.change(clock);
            if self.length_load == 0 {
                self.length_load = 64;
            }
            self.calculate_sweep();
            self.must_trigger = false;
        }
    }

    pub fn change(&mut self,clock:u32){
        if clock>=self.next_change{
            self.high = !self.high;
            self.next_change = self.next_change + self.step_frequency() *
            match (self.high,self.duty){
                (true,0) => 1,
                (true,1) => 2,
                (true,2) => 4,
                (true,3) => 6,
                (false,0)=> 7,
                (false,1)=> 6,
                (false,2)=> 4,
                (false,3)=> 2,
                _ => panic!("impossible duty cycle"),
            };
        }
    }

    pub fn step_sample(&mut self, clock:u32)->u16{
        if self.enable{
            self.change(clock);
        //    println!("{} * {}",self.high,self.envelope_volume);
            if self.high {
                self.envelope_volume as u16
            }else{
                0
            }
        }else{
            0
        }
    }
}

pub struct Audio{
    next_sample  : u32,
    next_samplef : f64,
    sample_len   : f64,
    power : bool,
    square1 : Square,
    out_frequency :u32,
}

impl Audio{
    pub fn origin()->Audio{
        Audio{
            next_sample : 0,
            next_samplef:0.0,
            sample_len  : 1048576.0 / 44100.0,
            power:true,
            square1:Square::origin(),
            out_frequency : 44100,
        }
    }
    pub fn write_sound_mode1_sweep(&mut self,v:u8){
        self.square1.sweep_period = (v >> 4) & 0x7;
        self.square1.sweep_negate = (v & 0x8) != 0;
        self.square1.sweep_shift  = v & 0x7;
    }
    pub fn write_sound_mode1_lp(&mut self, v:u8){
        self.square1.duty = (v >> 6) & 0x3;
        self.square1.length_load = v&0x3f;
    }
    pub fn write_sound_mode1_envelope(&mut self, v:u8){
        self.square1.envelope_volume = 
                (v >> 4) & 0xf;
        self.square1.envelope_add_mode = 
                v&0x8 != 0;
        self.square1.envelope_period = 
                v&0x7;
    }
    pub fn write_sound_mode1_frequency_lo(&mut self, v:u8){
        self.square1.frequency &= 0xff00;
        self.square1.frequency |= v as u16;
        println!("write half frequency");
    }
    pub fn write_sound_mode1_frequency_hi(&mut self, v:u8){
        self.square1.frequency &= 0xff;
        self.square1.frequency |= ((v&0x3) as u16)<<8;
        self.square1.must_trigger = v&0x80 != 0;
        self.square1.length_enable = v&0x4 != 0;
        println!("write other half frequency {}{}{}",self.square1.frequency,
        self.square1.must_trigger,self.square1.length_enable);
    }
    pub fn step(&mut self,clock :u32)->Interrupt{
        self.square1.step(clock);
        if clock%0x1fff == 0 {
               //runs at 512 hz 
            
        }
        if clock%0x3fff == 0 {
               //runs at 256 hz
            self.square1.lenght_decr();
        }
        if clock%0x7fff == 0 {
               //runs at 128 hz
            self.square1.step_sweep();
        }
        if clock%0xffff == 0 {
            self.square1.step_envelope();
               //run at 64 hz
                
        }
        if clock > self.next_sample{
            self.next_sample = clock;
            self.next_samplef = clock as f64;
        }
        if clock == self.next_sample{
            self.next_samplef = self.next_samplef + self.sample_len;
            self.next_sample = self.next_samplef as u32;

            return Interrupt::AudioSample((self.square1.step_sample(clock)*4) as u8)
        }
        Interrupt::None
    }
}