use cpu::ram::io::*;
use cpu::*;
use cpu::ram::Ram;


const Audio_Debug :bool = false;

/*
     NRx0 NRx1 NRx2 NRx3 NRx4
    ---------------------------
NR1x  $80  $3F $00  $FF  $BF 
NR2x  $FF  $3F $00  $FF  $BF 
NR3x  $7F  $FF $9F  $FF  $BF 
NR4x  $FF  $FF $00  $00  $BF 
NR5x  $00  $00 $70

$FF27-$FF2F always read back as $FF
*/


pub struct Noise{
    length:u8,
    envelope_volume : u8,
    envelope_add_mode: bool,
    envelope_period : u8,

    volume:u8,
    period:u8,

    clock_shift: u8,
    width_mode : bool,
    divisor_code:u8,

    must_trigger : bool,
    length_enable : bool,

    enable:bool,
    high:bool,
    total:u8,
    count:u8,
    next_shift:u32,

    shift_reg:u16,

}

impl Noise{
    pub fn origin()->Noise{
        Noise{
            length:63,
            envelope_volume : 15,
            envelope_add_mode : false,
            envelope_period:3,

            volume:0,
            period:0,

            clock_shift:0,
            width_mode:false,
            divisor_code:0,

            must_trigger:false,
            length_enable:false,
            enable:false,
            high:false,
            total:0,
            count:0,
            next_shift:0,
            shift_reg:1,
        }
    }
    pub fn clear(&mut self){
        self.write_envelope(0);
        self.write_frequency_hi(0);
        self.write_shift_reg(0);
        self.write_lp(0);
        self.period = 0;
        self.enable = false;
    }

    pub fn change_after(&self)->u32{
        match self.divisor_code{
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 6,
            4 => 8,
            5 => 10,
            6 => 12,
            7 => 14,
            _ => panic!(),
        }
    }

    pub fn step(&mut self, clock:u32){
        if self.must_trigger{
            self.next_shift = clock + self.change_after();

            self.enable = true;
            self.volume = self.envelope_volume;
            self.period = self.envelope_period;
            //self.change(clock);
            if self.length == 0 {
                self.length = 63;
            }
            self.must_trigger = false;
        }
        if self.enable{
            if clock >= self.next_shift{
                self.next_shift = clock + self.change_after();
                let last_bit = self.shift_reg&1;
                self.high =  last_bit ==0;
                self.count +=1;
                self.total += (1 - last_bit) as u8;
                let bit = ((self.shift_reg>>1)&1) ^ (self.shift_reg&1);
                self.shift_reg = self.shift_reg>>1;
                if self.width_mode{
                    self.shift_reg = self.shift_reg & 0x3f;
                    self.shift_reg |= bit<<6;
                }else{
                    self.shift_reg = self.shift_reg & ((1<<14)-1);
                    self.shift_reg |= bit<<14;
                }
            }
        }
    }

    pub fn change(&mut self,sample_len:f64,clock:u32)->f64{
        if self.count == 0{
            if self.high{
                1.0
            }else{
                0.0
            }
        }else{
            let ret = self.total as f64 / self.count as f64;
            self.total = 0;
            self.count = 0;
            ret
        }
    }


    pub fn step_sample(&mut self,sample_len:f64, clock:u32)->f64{
        if self.enable{
            (self.change(sample_len, clock) * self.envelope_volume as f64)
            /16.0 - 0.5
        }else{
            0.0
        }
    }




    pub fn write_lp(&mut self, v:u8){
        self.length = 63 - (v&0x3f);
        if Audio_Debug{
            println!("NOISE write length {}",self.length);
        }
    }
    pub fn read_lp(&self)->u8{
        0xff
    }
    pub fn write_envelope(&mut self, v:u8){
        self.envelope_volume = 
                (v >> 4) & 0xf;
        self.envelope_add_mode = 
                v&0x8 != 0;
        self.envelope_period = 
                v&0x7;
        if Audio_Debug{
            println!("NOISE write envelope {} {} {}",self.envelope_volume,
            self.envelope_add_mode,self.envelope_period);
        }
    }
    pub fn read_envelope(&self)->u8{
        let mut r = self.envelope_volume<<4;
        if self.envelope_add_mode {r|=0x8};
        r |= self.envelope_period;
        r
    }
    pub fn write_shift_reg(&mut self, v:u8){
        self.clock_shift = 
                (v >> 4) & 0xf;
        self.width_mode = 
                v&0x8 != 0;
        self.divisor_code = 
                v&0x7;
        if Audio_Debug{
           println!("NOISE write shift reg {} {} {}",self.clock_shift,
            self.width_mode,self.divisor_code);
        }
    }
    pub fn read_shift_reg(&self)->u8{
        let mut r = self.clock_shift << 4;
        if self.width_mode {r|=0x8};
        r|= self.divisor_code;
        r
    }
    pub fn write_frequency_hi(&mut self, v:u8){
        self.must_trigger = v&0x80 != 0;
        self.length_enable = v&0x40 != 0;
        if Audio_Debug{
            println!("NOISE write triggers{}{}",
            self.must_trigger,self.length_enable);
        }
    }
    pub fn read_frequency_hi(&self)->u8{
        if self.length_enable {0xff}else{0xbf}
    }
    pub fn write_register(&mut self,a:u16, v:u8){
        match a{
            0 => {},
            1 => self.write_lp(v),
            2 => self.write_envelope(v),
            3 => self.write_shift_reg(v),
            4 => self.write_frequency_hi(v),
            _ => panic!("writing impossible value"),
        }
    }
    pub fn read_register(& self,a:u16)->u8{
        match a{
            0 => 0xff,
            1 => self.read_lp(),
            2 => self.read_envelope(),
            3 => self.read_shift_reg(),
            4 => self.read_frequency_hi(),
            _ => panic!("reading impossible value"),
        }
    }}

const wave_clock_factor :u32 =  2;
pub struct Wave{
    frequency:u16,
    volume:f64,
    length:u8,
    power:bool,
    must_trigger:bool,
    length_enable:bool,
    samples:[u8;32],
    next_change:u32,
    cursor:u8,
    enable:bool,
    save_volume:u8,
}

impl Wave{
    pub fn origin()->Wave{
        Wave{
            frequency:0,
            volume:0.0,
            length:255,
            power:false,
            must_trigger:false,
            length_enable:false,
            next_change:0,
            samples:[0x8,0x4,0x4,0x0,0x4,0x3,0xA,0xA,
                     0x2,0xD,0x7,0x8,0x9,0x2,0x3,0xC,
                     0x6,0x0,0x5,0x9,0x5,0x9,0xB,0x0,
                     0x3,0x4,0xB,0x8,0x2,0xE,0xD,0xA],
            cursor:0,
            enable:false,
            save_volume:0,
        }
    }
    pub fn clear(&mut self){
        self.write_frequency_hi(0);
        self.write_frequency_lo(0);
        self.write_lp(0);
        self.write_power(0);
        self.write_volume(0);
        self.cursor = 0;
        self.save_volume = 0;
        self.enable = false;

    }

    pub fn step_frequency(&self)->u32{
        (2048 - self.frequency)as u32
    }
    pub fn lenght_decr(&mut self){
        if self.enable{
            if self.length_enable{
                if self.length == 0 {
                    self.enable = false;
                }
                self.length = self.length.saturating_sub(1);
            }
        }
    }

    pub fn change(&mut self,sample_len:f64,clock:u32)->f64{
        if clock*wave_clock_factor>=self.next_change{
            self.cursor += 1;
            let increment = self.step_frequency();

            let prop = (clock*2 - self.next_change)as f64 /(sample_len *2.0);

            self.next_change = self.next_change + increment;
//            println!("sound toggle in {} frequency is {} duty is {}",
//                increment, self.step_frequency(), self.duty);
            let last = self.samples[((self.cursor-1)%32) as usize] as f64;
            let new  = self.samples[((self.cursor)%32) as usize] as f64;
            self.cursor = self.cursor % 32;
//            println!("(1.0 - {}) * {} + {0} * {}",prop,last,new);
            (prop * last + (1.0 - prop) * new) 
        }else{
//            println!("nochange {} {}",self.cursor,self.samples[(self.cursor%32) as usize]);
            self.samples[(self.cursor%32) as usize] as f64
        }
        
    }

    pub fn step_sample(&mut self,sample_len:f64, clock:u32)->f64{
        if self.enable{
            ((self.change(sample_len, clock)-0.5) * self.volume as f64)/16.0
        }else{
            0.0
        }
    }

    pub fn step(&mut self,clock:u32){
        if self.must_trigger{
            self.enable = true;
            self.next_change = clock*wave_clock_factor+self.step_frequency();
            //self.change(clock);
            if self.length == 0 {
                self.length = 64;
            }
            self.must_trigger = false;
        }
    }
    pub fn write_volume(&mut self, v:u8){
        self.save_volume = v;
        self.volume = match bits(v,5,2){
            0 => 0.0,
            1 => 1.0,
            2 => 0.5,
            3 => 0.25,
            _ => panic!("impossible"),
        }
    }
    pub fn read_volume(&self)->u8{
        self.save_volume | 0x9f
    }
    pub fn write_power(&mut self, v:u8){
        self.power = v & 0x80 != 0;
    }
    pub fn read_power(&self)->u8{
        if self.power {0xff}else{0x7f}
    }
    pub fn write_lp(&mut self, v:u8){
        self.length = 255 - v;
        if Audio_Debug{
            println!("WAVE write length {} ",self.length);
        }
    }
    pub fn read_lp(&self)->u8{
        0xff
    }

    pub fn write_frequency_lo(&mut self, v:u8){
        self.frequency &= 0xff00;
        self.frequency |= v as u16;
        if Audio_Debug{
            println!("WAVE write half frequency");
        }
    }
    pub fn read_frequency_lo(&self)->u8{
        0xff
    }
    pub fn write_frequency_hi(&mut self, v:u8){
        self.frequency &= 0xff;
        self.frequency |= ((v&0x3) as u16)<<8;
        self.must_trigger = v&0x80 != 0;
        self.length_enable = v&0x40 != 0;
        if Audio_Debug{
            println!("WAVE write other half frequency {}{}{}",self.frequency,
            self.must_trigger,self.length_enable);
        }
    } 
    pub fn read_frequency_hi(&self)->u8{
        if self.length_enable{0xff}else{0xbf}
    }
    pub fn write_sample_ram(&mut self, a:u16, v:u8){
        self.samples[(a*2) as usize] = v >> 4;
        self.samples[(a*2) as usize +1] = v & 0xf;
    }
    pub fn read_sample_ram(&self, a:u16)->u8{
        (self.samples[(a*2) as usize] << 4) 
        | self.samples[(a*2) as usize +1]
    }
    pub fn write_register(&mut self, a:u16, v:u8){
        match a {
            0 => self.write_power(v),
            1 => self.write_lp(v),
            2 => self.write_volume(v),
            3 => self.write_frequency_lo(v),
            4 => self.write_frequency_hi(v),
            _ => panic!("writing impossible register"),
        }
    }
    pub fn read_register(&self, a:u16)->u8{
        match a {
            0 => self.read_power(),
            1 => self.read_lp(),
            2 => self.read_volume(),
            3 => self.read_frequency_lo(),
            4 => self.read_frequency_hi(),
            _ => panic!("writing impossible register"),
        }
    }}


pub struct Square{
    //Frequency = 4194304/(32*(2048-x)) Hz
    frequency:u16,
    volume:u8,
    last_rise:u32,
    next_change:u32,//in 1/8 of clock
    shadow_frequency:u16,
    envelope_volume : u8,
    envelope_add_mode: bool,
    envelope_period : u8,
    period:u8,
    trigger : bool,
    must_trigger : bool,
    length_enable : bool,
    duty : u8,
    //Sound Length = (64-t1)*(1/256) seconds
    length:u8,
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
            volume:15,
            last_rise:0,
            next_change:0,
            shadow_frequency:0,
            envelope_volume : 15,
            envelope_add_mode : false,
            envelope_period:3,
            period:0,
            trigger : false,
            must_trigger: false,
            high :false,
            length_enable :false,
            duty:2,
            length:63,
            sweep_period:0,
            sweep_negate:true,
            sweep_shift:0,
            sweep_enable:false,
            sweep_timer:0,
            enable:false,
        }
    }

    pub fn clear(&mut self){
        self.write_envelope(0);
        self.write_frequency_hi(0);
        self.write_frequency_lo(0);
        self.write_lp(0);
        self.write_sweep(0);
        self.frequency = 0;
        self.next_change = 0;
        self.high = false;
        self.enable = false;
    }

    pub fn step_frequency(&self)->u32{
        (2048 - self.frequency)as u32
    }

    pub fn lenght_decr(&mut self){
        if self.length_enable{
            self.length = self.length.saturating_sub(1);
        }
        if self.length == 0 {
            self.enable = false;
        }
        
    }

    pub fn step_envelope(&mut self){
        if self.enable == false || self.period == 0 {return;}
        self.period -= 1;
        let t = if self.envelope_add_mode{
            self.volume.wrapping_add(1)
        }else{
            self.volume.wrapping_sub(1)
        };
        if t>=0 && t<=15{
            self.volume = t;
        }
        if t== 0 {
            self.enable = false
            };
    }
    pub fn step_sweep(&mut self){
        if self.sweep_enable && self.sweep_period>0{
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
  
    pub fn step(&mut self,clock:u32){
        if self.must_trigger{
            self.last_rise = clock;
            self.shadow_frequency = self.frequency;
            self.sweep_enable = self.sweep_period != 0 || self.sweep_shift != 0;
            self.enable = true;
            self.next_change = clock*8 + self.toggle_after(false,clock);
            self.volume = self.envelope_volume;
            self.period = self.envelope_period;
            //self.change(clock);
            if self.length == 0 {
                self.length = 64;
            }
            self.calculate_sweep();
            self.must_trigger = false;
        }
    }

    pub fn toggle_after(&self, level:bool,clock_time:u32)->u32{
        self.step_frequency() * match(level,self.duty){
            (true,0) => 1,
            (true,1) => 2,
            (true,2) => 4,
            (true,3) => 6,
            (false,0)=> 7,
            (false,1)=> 6,
            (false,2)=> 4,
            (false,3)=> 2,
            _ => panic!("impossible duty cycle"),
        }
    }

    pub fn change(&mut self,sample_len:f64,clock:u32)->f64{
        if clock*8>=self.next_change{
            self.high = !self.high;
            let increment = self.toggle_after(self.high, self.next_change);

            let ret = (clock*8 - self.next_change) as f64/(sample_len *8.0);

            self.next_change = self.next_change + increment;
//            println!("sound toggle in {} frequency is {} duty is {} ret is {}\n    {}*8 - {} / {}",
//                increment, self.step_frequency(), self.duty,ret,clock,self.next_change,sample_len);
            if self.high{
                ret - 0.5
            }else{
                0.5 - ret
            }
        }else{
            if self.high{
                0.5
            }else{
                -0.5
            }
        }
        
    }

    pub fn step_sample(&mut self,sample_len:f64, clock:u32)->f64{
        if self.enable{
            self.change(sample_len, clock) * self.volume as f64
            /16.0
        }else{
            0.0
        }
    }

    pub fn write_sweep(&mut self,v:u8){
        self.sweep_period = (v >> 4) & 0x7;
        self.sweep_negate = (v & 0x8) != 0;
        self.sweep_shift  = v & 0x7;
        if Audio_Debug{
            println!("write sweep period {} negate {} shift {}",
                self.sweep_period,self.sweep_negate,self.sweep_shift);
        }
    }
    pub fn read_sweep(&self)->u8{
        let mut r = self.sweep_period << 4;
        if self.sweep_negate {r |= 0x8};
        r |= self.sweep_shift;
        r | 0x80
    }
    pub fn write_lp(&mut self, v:u8){
        self.duty = (v >> 6) & 0x3;
        self.length = 63 - (v&0x3f);
        if Audio_Debug{
            println!("write length {} duty {}",
                    self.length,self.duty);
        }
    }
    pub fn read_lp(&self)->u8{
        (self.duty << 6)|0x3f
    }
    pub fn write_envelope(&mut self, v:u8){
        self.envelope_volume = 
                (v >> 4) & 0xf;
        self.envelope_add_mode = 
                v&0x8 != 0;
        self.envelope_period = 
                v&0x7;
        if Audio_Debug{
            println!("write envelope {} {} {}",self.envelope_volume,
            self.envelope_add_mode,self.envelope_period);
        }
    }
    pub fn read_envelope(&self)->u8{
        let mut r = self.envelope_volume << 4;
        if self.envelope_add_mode {r |= 0x8};
        r |= self.envelope_period;
        r
    }
    pub fn write_frequency_lo(&mut self, v:u8){
        self.frequency &= 0xff00;
        self.frequency |= v as u16;
        if Audio_Debug{
            println!("write half frequency");
        }
    }
    pub fn read_frequency_lo(&self)->u8{
        0xff
    }
    pub fn write_frequency_hi(&mut self, v:u8){
        self.frequency &= 0xff;
        self.frequency |= ((v&0x3) as u16)<<8;
        self.must_trigger = v&0x80 != 0;
        self.length_enable = v&0x40 != 0;
        if Audio_Debug{
            println!("write other half frequency {}{}{}",self.frequency,
                self.must_trigger,self.length_enable);
        }
    }
    pub fn read_frequency_hi(&self)->u8{
        if self.length_enable {0xff} else {0xbf}
    }
    pub fn write_register(&mut self, a:u16, v:u8){
        match a{
            0 => self.write_sweep(v),
            1 => self.write_lp(v),
            2 => self.write_envelope(v),
            3 => self.write_frequency_lo(v),
            4 => self.write_frequency_hi(v),
            _ => panic!("writing impossible register"),
        }
    }
    pub fn read_register(&self, a:u16)->u8{
        match a{
            0 => self.read_sweep(),
            1 => self.read_lp(),
            2 => self.read_envelope(),
            3 => self.read_frequency_lo(),
            4 => self.read_frequency_hi(),
            _ => panic!("reading impossible register"),
        }
    }
}


pub struct Audio{
    next_sample  : u32,
    next_samplef : f64,
    sample_len   : f64,
    power : bool,
    pub square1 : Square,
    pub square2 : Square,
    pub wave3   : Wave,
    pub noise4  : Noise,
    out_frequency :u32,

    volume_left:u8,
    volume_right:u8,
    left_enable:bool,
    right_enable:bool,

    sound1_to_left:bool,
    sound2_to_left:bool,
    sound3_to_left:bool,
    sound4_to_left:bool,

    sound1_to_right:bool,
    sound2_to_right:bool,
    sound3_to_right:bool,
    sound4_to_right:bool,
}

impl Audio{
    pub fn origin()->Audio{
        Audio{
            next_sample : 0,
            next_samplef:0.0,
            sample_len  : 1048576.0 / 44100.0,
            power:true,
            square1:Square::origin(),
            square2:Square::origin(),
            wave3:Wave::origin(),
            noise4:Noise::origin(),
            out_frequency : 44100,

            volume_left: 7,
            volume_right: 7,
            left_enable : true,
            right_enable :true,

            sound1_to_left:true,
            sound2_to_left:true,
            sound3_to_left:true,
            sound4_to_left:true,

            sound1_to_right:true,
            sound2_to_right:true,
            sound3_to_right:true,
            sound4_to_right:true,

        }
    }
    
    pub fn write_register(&mut self,a:u16,v:u8){
        match a{
            0x26 => self.write_power_flag(v),
            0x30...0x3f => self.wave3.write_sample_ram(a-0x30, v),
            _ => {
                if self.power{
                    match a{
                        0x10...0x14 => {
                            if Audio_Debug{
                                print!("SQUARE 1 ");
                            }
                            self.square1.write_register(a-0x10,v);
                        },
                        0x16...0x19 => {
                            if Audio_Debug{
                                print!("SQUARE 2 ");
                            }
                            self.square2.write_register(a-0x15,v);
                        },
                        0x1a...0x1e => {
                            if Audio_Debug{
                                print!("WAVE ");
                            }
                            self.wave3.write_register(a-0x1a,v);
                        },
                        0x20...0x23 => {
                            if Audio_Debug{
                                print!("NOISE ");
                            }
                            self.noise4.write_register(a-0x1f,v);
                        },
                        0x24 => self.write_stereo_volume(v),
                        0x25 => self.write_output_selection(v),
                        _ => {},
                    }
                }
            }
        }
    }
    pub fn read_register(&self,a:u16)->u8{
        match a{
            0x10...0x14 => self.square1.read_register(a-0x10),
            0x16...0x19 => self.square2.read_register(a-0x15),
            0x1a...0x1e => self.wave3.read_register(a-0x1a),
            0x20...0x23 => self.noise4.read_register(a-0x1f),
            0x24 => self.read_stereo_volume(),
            0x25 => self.read_output_selection(),
            0x26 => self.read_power_flag(),
            0x30...0x3f => self.wave3.read_sample_ram(a-0x30),
            _ => 0xff,
        }
    }
    pub fn write_stereo_volume(&mut self, v:u8){
        self.left_enable = bit(v,7);
        self.right_enable =bit(v,3);
        self.volume_left = bits(v,4,3);
        self.volume_right= bits(v,0,3);
        if Audio_Debug{
            println!("setting volume left:{}:{} Right:{}:{}",self.left_enable,
               self.volume_left,self.right_enable,self.volume_right);
        }
    }
    pub fn read_stereo_volume(&self)->u8{
        let mut r =0;
        if self.left_enable { r |= 1 << 7;}
        if self.right_enable{ r |= 1 << 3;}
        r |= (self.volume_left & 0x7) << 4;
        r |= self.volume_right & 0x7;
        r
    }

    pub fn write_output_selection(&mut self,v:u8){
    //    println!("setting audio output selection {:02x}",v);
        self.sound4_to_left = bit(v,7);
        self.sound3_to_left = bit(v,6);
        self.sound2_to_left = bit(v,5);
        self.sound1_to_left = bit(v,4);

        self.sound4_to_right = bit(v,3);
        self.sound3_to_right = bit(v,2);
        self.sound2_to_right = bit(v,1);
        self.sound1_to_right = bit(v,0);
    }
    pub fn read_output_selection(&self)->u8{
        bit_merge(self.sound1_to_right,self.sound2_to_right,
                  self.sound3_to_right,self.sound4_to_right,
                  self.sound1_to_left,self.sound2_to_left,
                  self.sound3_to_left,self.sound4_to_left)
    }
    pub fn write_power_flag(&mut self, v:u8){
        self.power = bit(v,7);
        if !self.power{
            self.square1.clear();
            self.square2.clear();
            self.wave3.clear();
            self.noise4.clear();
            self.write_stereo_volume(0x00);
            self.write_output_selection(0x00);
        }
        if Audio_Debug{
            println!("setting audio power to {}",self.power);
        }
    }
    pub fn read_power_flag(&self)->u8{
        bit_merge(self.square1.enable,self.square2.enable,
                  self.wave3.enable,self.noise4.enable,
                  true,true,true,self.power)
    }

    pub fn step(&mut self,clock :u32)->Interrupt{
        self.square1.step(clock);
        self.square2.step(clock);
        self.wave3.step(clock);
        self.noise4.step(clock);
        if clock%0x1fff == 0 {
               //runs at 512 hz 
        }
        if clock%0x3fff == 0 {
               //runs at 256 hz
            self.square1.lenght_decr();
            self.square2.lenght_decr();
            self.wave3.lenght_decr();
        }
        if clock%0x7fff == 0 {
               //runs at 128 hz
            self.square1.step_sweep();
        }
        if clock%0xffff == 0 {
            self.square1.step_envelope();
            self.square2.step_envelope();
               //run at 64 hz    
        }

        if clock >= self.next_sample{
            self.next_samplef = self.next_samplef + self.sample_len;
            self.next_sample = self.next_samplef as u32;
            let sample1 = self.square1.step_sample(self.sample_len,clock);
            let sample2 = self.square2.step_sample(self.sample_len,clock);
            let sample3 = self.wave3.step_sample(self.sample_len, clock);
            let sample4 = self.noise4.step_sample(self.sample_len, clock);


            let out_left = {
                let mut o = 0.0;
                if self.sound1_to_left { o+= sample1 ;}
                if self.sound2_to_left { o+= sample2 ;}
                if self.sound3_to_left { o+= sample3 ;}
                if self.sound4_to_left { o+= sample4 ;}
                (((o * self.volume_left as f64)/16.0) as f32)
            };
            let out_right = {
                let mut o = 0.0;
                if self.sound1_to_right { o += sample1;}
                if self.sound2_to_right { o += sample2;}
                if self.sound3_to_right { o += sample3;}
                if self.sound4_to_right { o += sample4;}
                (((o * self.volume_right as f64)/16.0) as f32)
            };
            return Interrupt::AudioSample(out_left,out_right)
        }
        Interrupt::None
    }
}




#[cfg(test)]
mod tests {
    use cpu::ram::io::audio;
    const mask:[u8;22] = 
                 [0x80, 0x3F, 0x00, 0xFF, 0xBF, 0xFF, 0x3F, 0x00, 0xFF, 0xBF,
                  0x7F, 0xFF, 0x9F, 0xFF, 0xBF, 0xFF, 0xFF, 0x00, 0x00, 0xBF,
                  0x00, 0x00];
    #[test]
    fn registers_write_read(){
        let mut a = audio::Audio::origin();
        
        a.write_register(0x26,0xff);
        for v in 0..=255{
        for i in 0x10..=0x25{
            a.write_register(i,v);
            a.step(i as u32 + v as u32*256);
            assert_eq!(a.read_register(i),v|mask[i as usize-0x10]);
        }
        }
    }
    #[test]
    fn registers_ensure_off(){
        let mut a = audio::Audio::origin();
        a.write_register(0x26,0xff);
        for i in 0x10..=0x25{
            a.write_register(i,0xff);
        }
        for i in 0x10..=0x25{
            assert_eq!(a.read_register(i),0xff);
        }
        a.write_register(0x26,0x00);
        for i in 0x10..=0x25{
            a.write_register(i,0x00);
            assert_eq!(a.read_register(i),mask[i as usize - 0x10]);

        }
        for i in 0x10..=0x25{
            a.write_register(i,0xff);
            assert_eq!(a.read_register(i),mask[i as usize - 0x10]);

        }
        a.write_register(0x26,0xff);
        for i in 0x10..=0x25{
            a.write_register(i,0x00);
            assert_eq!(a.read_register(i),mask[i as usize - 0x10]);

        }
    }
    #[test]
    fn sample_ram(){
        let mut a = audio::Audio::origin();
        a.write_register(0x26,0xff);
        for v in 0..=255{
        for i in 0x30..=0x3f{
            a.write_register(i,v);
            assert_eq!(a.read_register(i),v);
        }
        }
    }
}