use defmt::info;

#[derive(Debug)]
pub enum Mbc {
    No,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc4,
    Mbc5,
    Mbc6,
    Mbc7,
    Mmm01,
    PocketCamera,
    BandaiTama5,
    HuC3,
    HuC1,
}
impl Default for Mbc {
    fn default() -> Mbc {
        Mbc::No
    }
}

pub struct Cartridge {
    pub mbc: Mbc,
    pub has_ram: bool,
    pub has_battery: bool,
    pub has_timer: bool,
    pub has_rumble: bool,
    pub has_sensor: bool,
    //    pub rom: [u8; 0x4000],
    //    pub romswitch: Vec<[u8; 0x4000]>,
    pub fullrom: &'static [u8],
    pub ramswitch: [[u8; 0x2000]; 1],
    pub cur_ram: usize,
    pub cur_rom: usize,
    //    filename: String,
    //    savefile: String,
}

impl Default for Cartridge {
    fn default() -> Cartridge {
        Cartridge {
            mbc: Mbc::No,
            has_ram: false,
            has_battery: false,
            has_timer: false,
            has_rumble: false,
            has_sensor: false,
            //            rom: [0; 0x4000],
            fullrom: include_bytes!(
                "../../../Legend of Zelda, The - Link's Awakening (U) (V1.2) [!].gb"
            ),
            ramswitch: [[0; 0x2000]; 1],
            cur_ram: 0,
            cur_rom: 0,
            //            filename: String::new(),
            //            savefile: String::new(),
        }
    }
}
impl Cartridge {
    pub fn setup(self) -> Cartridge {
        let mut c = self;
        match c.fullrom[0x147] {
            0x00 => {}
            0x01 => {
                c.mbc = Mbc::Mbc1;
            }
            0x02 => {
                c.mbc = Mbc::Mbc1;
                c.has_ram = true;
            }
            0x03 => {
                c.mbc = Mbc::Mbc1;
                c.has_ram = true;
                c.has_battery = true;
            }
            0x05 => {
                c.mbc = Mbc::Mbc2;
            }
            0x06 => {
                c.mbc = Mbc::Mbc2;
                c.has_battery = true;
            }
            0x08 => {
                c.has_ram = true;
            }
            0x09 => {
                c.has_ram = true;
                c.has_battery = true;
            }
            0x0b => {
                c.mbc = Mbc::Mmm01;
            }
            0x0c => {
                c.mbc = Mbc::Mmm01;
                c.has_ram = true;
            }
            0x0d => {
                c.mbc = Mbc::Mmm01;
                c.has_ram = true;
                c.has_battery = true;
            }
            0x0f => {
                c.mbc = Mbc::Mbc3;
                c.has_timer = true;
                c.has_battery = true;
            }
            0x10 => {
                c.mbc = Mbc::Mbc3;
                c.has_timer = true;
                c.has_battery = true;
                c.has_ram = true;
            }
            0x11 => {
                c.mbc = Mbc::Mbc3;
            }
            0x12 => {
                c.mbc = Mbc::Mbc3;
                c.has_ram = true;
            }
            0x13 => {
                c.mbc = Mbc::Mbc3;
                c.has_ram = true;
                c.has_battery = true;
            }
            0x19 => {
                c.mbc = Mbc::Mbc5;
            }
            0x1a => {
                c.mbc = Mbc::Mbc5;
                c.has_ram = true;
            }
            0x1b => {
                c.mbc = Mbc::Mbc5;
                c.has_ram = true;
                c.has_battery = true;
            }
            0x1c => {
                c.mbc = Mbc::Mbc5;
                c.has_rumble = true;
            }
            0x1d => {
                c.mbc = Mbc::Mbc5;
                c.has_rumble = true;
                c.has_ram = true;
            }
            0x1e => {
                c.mbc = Mbc::Mbc5;
                c.has_rumble = true;
                c.has_ram = true;
                c.has_battery = true;
            }
            0x20 => {
                c.mbc = Mbc::Mbc6;
            }
            0x22 => {
                c.mbc = Mbc::Mbc7;
                c.has_sensor = true;
                c.has_rumble = true;
                c.has_ram = true;
                c.has_battery = true;
            }
            0xfc => {
                c.mbc = Mbc::PocketCamera;
            }
            0xfd => {
                c.mbc = Mbc::BandaiTama5;
            }
            0xfe => {
                c.mbc = Mbc::HuC3;
            }
            0xff => {
                c.mbc = Mbc::HuC1;
                c.has_ram = true;
            }
            _ => panic!("I dont know that cartridge type"),
        }
        c
    }
    /*
    pub fn extract_title(&self) -> str {
        let mut s = [char;16];
        for i in 0x134..=0x142 {
            if self.rom[i] == 0 {
                return s;
            } else {
                s.push(self.rom[i] as char);
            }
        }
        s
    }*/
    pub fn set_rom_bank(&mut self, b: u8) {
        self.cur_rom = (core::cmp::max(b, 1) - 1) as usize;
        //TODO suport bigger rom
    }
    pub fn read_romswitch(&self, a: u16) -> u8 {
        //println!("read from romswitch {} :{:02x}",self.cur_rom,a);
        self.fullrom[self.cur_rom * 0x4000 + a as usize]
    }
    pub fn read_ramswitch(&self, a: u16) -> u8 {
        self.ramswitch[self.cur_ram][a as usize]
    }
    pub fn write_ramswitch(&mut self, a: u16, v: u8) {
        info!(
            "write to ramswitch {:02x}:{:04x} = {:02x} {}",
            self.cur_ram, a, v, v as char
        );
        if self.ramswitch.len() == 0 {
            return;
        };
        self.ramswitch[self.cur_ram][a as usize] = v;
    }
    pub fn is_cgb(&self) -> bool {
        self.fullrom[0x143] == 0x80
    }
    pub fn get_rom_bank_count(&self) -> u16 {
        match self.fullrom[0x148] {
            0x00 => 2,
            0x01 => 4,
            0x02 => 8,
            0x03 => 16,
            0x04 => 32,
            0x05 => 64,
            0x06 => 128,
            0x07 => 256,
            0x08 => 512,
            0x52 => 72,
            0x53 => 80,
            0x54 => 96,
            _ => panic!("unknown rom size"),
        }
    }

    pub fn get_ram_bank_count(&self) -> u16 {
        match self.fullrom[0x149] {
            0x00 => 0,
            0x01 => 1,
            0x02 => 1,
            0x03 => 4,
            0x04 => 16,
            0x05 => 8,
            _ => panic!("unknown ram size"),
        }
    }

    pub fn extract_info(&self) {
        //info!("{}", self.extract_title());
        if self.is_cgb() {
            info!("Is CGB");
        } else {
            info!("In not CGB");
        }
        info!("Old licensee code {:02x}", self.fullrom[0x14b]);
        info!(
            "New licensee code {:02x}{:02x}",
            self.fullrom[0x144], self.fullrom[0x145]
        );
        /*info!(
            "Memory controller : {:?}, ram {}, battery {}, timer {}",
            self.mbc, self.has_ram, self.has_battery, self.has_timer
        );*/
        info!(
            "{} Kbytes of rom, {} banks of ram",
            self.get_rom_bank_count() as u32 * 16,
            self.get_ram_bank_count()
        );
        if self.fullrom[0x14a] == 0 {
            info!("Japanese game");
        } else {
            info!("Non Japanese game");
        }
        info!("Game revision {}", self.fullrom[0x14c]);
    }
}
