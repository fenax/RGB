use std::fmt;
use cpu::*;

pub struct Alu{
    pub Fzero:bool,
    pub Fsub:bool,
    pub Fhalf:bool,
    pub Fcarry:bool,
}

impl fmt::Display for Alu{
    fn fmt(&self, f: &mut fmt::Formatter<'_>)
     -> fmt::Result 
    {
        write!(f, "{}{}{}{}", 
               if self.Fzero {"Z"}else{"-"},
               if self.Fsub  {"S"}else{"-"},
               if self.Fhalf {"H"}else{"-"},
               if self.Fcarry{"C"}else{"-"})
    }
}

impl Alu{
    pub fn origin() -> Alu{
        Alu{
            Fzero:false,
            Fsub:false,
            Fhalf:false,
            Fcarry:false
        }
    }
    pub fn get_f(&self)->u8{
        let mut r = 0 as u8;
        if(self.Fzero) {r+= 1<<7};
        if(self.Fsub)  {r+= 1<<6};
        if(self.Fhalf) {r+= 1<<5};
        if(self.Fcarry){r+= 1<<4};
        r
    }
    pub fn set_f(&mut self, f: u8){
        self.Fzero = (f & 1<<7)!=0;
        self.Fsub  = (f & 1<<6)!=0;
        self.Fhalf = (f & 1<<5)!=0;
        self.Fcarry= (f & 1<<4)!=0;
    }
    pub fn set_flags(&mut self,z:bool,s:bool,h:bool,c:bool){
        self.Fzero = z;
        self.Fsub = s;
        self.Fhalf = h;
        self.Fcarry = c;
    }
    pub fn and(&mut self,a:&mut u8,b:u8)->Option<u8>{
        *a = *a & b;
        self.set_flags(*a==0, false,true,false);
        None
    }
    pub fn or(&mut self,a:&mut u8,b:u8)->Option<u8>{
        *a = *a | b;
        self.set_flags(*a==0,false,false,false);
        None
    }
    pub fn xor(&mut self,a:&mut u8,b:u8)->Option<u8>{
        *a = *a ^ b;
        self.set_flags(*a==0,false,false,false);
        None
    }
    pub fn add16(&mut self,l:&mut u8,h:&mut u8,b:u16)->Option<u8>{
        let HL = u8tou16(*l,*h);
        let (rl,rh) = u16tou8(self.add16_(HL,b));
        *h = rh;
        *l = rl;
        Some(1)
    }
    pub fn add16_(&mut self,a: u16, b: u16)->u16{
        self.Fhalf = (((a&0xfff) + (b&0xfff))>0xfff);
        self.Fsub = false;
        let (r,c) = a.overflowing_add(b);
        self.Fcarry = c;
        r
    }
    pub fn add(&mut self,a:&mut u8,b:u8)->Option<u8>{
        self.Fhalf = (((*a&0xf) + (b&0xf))>0xf);
        self.Fsub = false;
        let (r,c) = a.overflowing_add(b);
        self.Fzero = r==0;
        self.Fcarry = c;
        *a = r;
        None
    }
    pub fn sub16(&mut self,l:&mut u8,h:&mut u8,b:u8)->Option<u8>{
        let HL = u8tou16(*l,*h);
        self.Fhalf = HL&0xfff < (b as u16)&0xfff;
        self.Fsub = true;
        let (r,c) = HL.overflowing_sub(b.into());
        self.Fzero = r==0;
        self.Fcarry = c;
        let (rl,rh) = u16tou8(r);
        *h=rh;
        *l=rl;
        Some(1)
    }
    pub fn sub(&mut self,a:&mut u8,b:u8)->Option<u8>{
        self.Fhalf = *a & 0xf < b & 0xf;
        self.Fsub = true;
        let (r,c) = a.overflowing_sub(b);
        self.Fzero = r==0;
        self.Fcarry = c;
        *a = r;
        None
    }
    pub fn cp(&mut self,a: u8,b:u8)->Option<u8>{
        self.Fhalf = a & 0xf < b&0xf;
        self.Fsub = true;
        let (r,c) = a.overflowing_sub(b);
        self.Fzero = r==0;
        self.Fcarry = c;
        None
    }
    pub fn adc(&mut self,a:&mut u8,b:u8)->Option<u8>{
        if self.Fcarry {
            self.Fhalf = ((*a&0xf) + (b&0xf + 1))>0xf;
            self.Fsub = false;
            let (r1,c1) = a.overflowing_add(b);
            let (r,c2) =  r1.overflowing_add(1);
            self.Fzero = r==0;
            self.Fcarry = c1 || c2;
            *a = r;
            None
        }else{ self.add(a,b) }
    }
    pub fn sbc(&mut self,a:&mut u8,b:u8)->Option<u8>{
        if self.Fcarry {
            self.Fhalf = *a&0xf <= b&0xf;
            self.Fsub = true;
            let (r1,c1) = a.overflowing_sub(b);
            let (r, c2) = r1.overflowing_sub(1);
            self.Fzero = r==0;
            self.Fcarry = c1 || c2;
            *a = r;
            None
        }else{ self.sub(a,b) }
    }
    pub fn inc(&mut self,a:&mut u8)->Option<u8>{
        *a = a.wrapping_add(1);
        self.Fhalf = (*a&0xf) == 0;
        self.Fsub = false;
        self.Fzero = *a==0;
        None
    }
    pub fn dec(&mut self,a:&mut u8)->Option<u8>{
        *a = a.wrapping_sub(1);
        self.Fhalf = (*a&0xf) == 0xf;
        self.Fsub = true;
        self.Fzero = *a==0;
        None
    }
    pub fn inc16(& self,l:&mut u8,h:&mut u8)->Option<u8>{
        let mut r = u8tou16(*l,*h);
        r = r.wrapping_add(1);
        let (rl,rh) = u16tou8(r);
        *l = rl;
        *h = rh;
        Some(1)
    }
    pub fn dec16(& self,l:&mut u8,h:&mut u8)->Option<u8>{
        let mut r = u8tou16(*l,*h);
        r = r.wrapping_sub(1);
        let (rl,rh) = u16tou8(r);
        *l = rl;
        *h = rh;
        Some(1)
    }    

}
