use crate::cpu::ram::io::video::*;
use crate::{display_dma_line, display_end, display_line, display_wait_sync, Ipc};
use core::cell::RefCell;
use defmt::{debug, info};
use pio_proc::pio_asm;
use rp_pico::hal::pio::{PIOExt, ShiftDirection};
use rp_pico::hal::sio::SioFifo;

#[derive(Clone, Copy)]
pub struct Pixel {
    behind_bg: bool,
    palette: bool,
    color: u8,
}

#[derive(Clone, Copy)]
pub struct WindowPixel {
    transparent: bool,
    color: u8,
}

pub fn apply_palette(v: u8, palette: u8) -> u8 {
    (palette >> (v * 2)) & 0b11
}

trait PixelPipeline {
    fn init(oam: &[Sprite; 40], video: &Video, line: u8) -> Self;
    fn refresh(&mut self, vram: &VideoRam, oam: &[Sprite; 40], x: u8);
    fn pixel(&mut self, vram: &VideoRam, oam: &[Sprite; 40], video: &Video, x: u8) -> Option<u8>;
}

struct BgLineRenderer {
    enabled: bool,
    tile_set: bool,
    tile_map_line_offset: u16,
    line: u16,
    //tile_line: u16,
    tile_sub_line: u8,
    column_offset: u8,

    //tile_column: u8,
    //tile_sub_column: u8,
    //map_offset: u16,
    //tile: u8,
    tile_data: (u8, u8),
    tile_bit: u8,
}

macro_rules! retry {
    ($e:expr) => {
        loop {
            #[allow(unreachable_patterns)]
            match $e {
                None => {}
                Some(x) => break x,
            }
        }
    };
}

impl PixelPipeline for BgLineRenderer {
    fn init(_oam: &[Sprite; 40], video: &Video, line: u8) -> Self {
        let mut out = video.with_reg(|reg| {
            let tile_map = if reg.background_tile_map {
                0x1C00
            } else {
                0x1800
            };
            let bg_line = (line as u16 + reg.scroll_y as u16) % 256;
            let tile_line = bg_line / 8;
            BgLineRenderer {
                enabled: reg.enable_background,
                tile_set: reg.tile_set,
                tile_map_line_offset: tile_map + tile_line * 32,
                line: bg_line,
                tile_sub_line: (bg_line % 8) as u8,
                column_offset: reg.scroll_x,
                tile_bit: 8,
                tile_data: (0, 0),
            }
        });
        //out.refresh(vram, oam, 0);
        out
    }
    fn refresh(&mut self, vram: &VideoRam, _oam: &[Sprite; 40], x: u8) {
        if self.tile_bit >= 8 {
            let column = self.column_offset.wrapping_add(x);
            let tile_column = column / 8;
            self.tile_bit = column % 8;
            let map_offset = self.tile_map_line_offset + tile_column as u16;
            let tile = vram.vram[map_offset as usize];
            self.tile_data = vram.get_tile(self.tile_set, tile, self.tile_sub_line);
        }
    }
    fn pixel(&mut self, vram: &VideoRam, oam: &[Sprite; 40], _video: &Video, x: u8) -> Option<u8> {
        if self.enabled {
            //let color = vram.get_tile(
            //    self.tile_set,
            //    self.tile,
            //    self.tile_sub_line * 8 + self.tile_sub_column as u16,
            //);
            self.refresh(vram, oam, x);

            let color = get_tile_pixel(self.tile_data, self.tile_bit);
            // ((self.tile_data.0 >> self.tile_bit) & 1)
            //    + (((self.tile_data.1 >> self.tile_bit) & 1) * 2);
            //info!("color is {}", color);
            self.tile_bit += 1;
            Some(color)
        } else {
            None
        }
    }
}

fn get_tile_pixel(data: (u8, u8), bit: u8) -> u8 {
    ((data.0 >> 7 - bit) & 1) + (((data.1 >> 7 - bit) & 1) * 2)
}

struct SpriteRenderer {
    enabled: bool,
    yoffset: u8,
    sprite_size: bool,
    //    line_buffer: [Pixel;160],
    list: [Option<Sprite>; 10],
    line: u8,
}

impl SpriteRenderer {
    fn init(oam: &[Sprite; 40], video: &Video, line: u8) -> Self {
        let (sprite_size, enable_sprites) =
            video.with_reg(|reg| (reg.sprite_size, reg.enable_sprites));
        let yoffset;
        if sprite_size {
            yoffset = 16;
        } else {
            yoffset = 8;
        }

        if enable_sprites == false {
            return SpriteRenderer {
                enabled: false,
                yoffset,
                sprite_size,
                //                line_buffer:[Pixel{behind_bg:true,palette:false,color:0};160],
                line,
                list: [None; 10],
            };
        }

        let mut list: [Option<Sprite>; 40] = [None; 40];
        for i in 0..40 {
            if (oam[i].y <= line + 16 && oam[i].x > line + 16 - yoffset) {
                list[i] = Some(oam[i].clone());
            } else {
                list[i] = None;
            }
        }

        list.sort_unstable_by(|a, b| {
            if let (Some(a), Some(b)) = (a, b) {
                b.x.cmp(&a.x)
            } else {
                b.cmp(a)
            }
        });
        let mut out_list = [None; 10];
        for i in 0..10 {
            out_list[i] = list[i + 30];
        }
        SpriteRenderer {
            enabled: true,
            sprite_size,
            yoffset,
            //            line_buffer,
            list: out_list,
            line,
        }
    }
    pub fn render(&mut self, ram: &VideoRam) -> [Pixel; 160] {
        if self.enabled {
            let mut line = [Pixel {
                behind_bg: false,
                palette: false,
                color: 0,
            }; 160];
            for f in self.list.iter().filter_map(|x| *x) {
                let mut tile_line = self.line as i16 - (f.y as i16 - 16);
                let mut tile;
                if self.sprite_size {
                    if tile_line < 8 {
                        //upper tile
                        tile = f.tile & 0xfe;
                    } else {
                        //lower tile
                        tile_line -= 8;
                        tile = f.tile | 0x01;
                    }
                    if f.y_flip {
                        tile ^= 0x01;
                    }
                } else {
                    tile = f.tile;
                }

                let tile_line = if f.y_flip { 7 - tile_line } else { tile_line } as u8;

                //            println!("16 tile {} {} {:02x} {}",f.x,f.y,tile,tile_line);
                let tile_line_data = ram.get_tile(true, tile, tile_line);
                let start = f.x.saturating_sub(8);
                let mut tile_bit = start.abs_diff(f.x);
                for i in start..core::cmp::min(f.x, 159) {
                    let color = get_tile_pixel(
                        tile_line_data,
                        if f.x_flip { 7 - tile_bit } else { tile_bit },
                    );
                    tile_bit += 1;
                    //ram.get_tile_1(tile, (tile_line * 8 + tile_column as i16) as u16);
                    //                println!("pixel {} {} {}",self.line,i,color);
                    if color != 0 {
                        line[i as usize] = Pixel {
                            behind_bg: f.behind_bg,
                            palette: f.palette,
                            color,
                        };
                    }
                }
            }
            line
        } else {
            [Pixel {
                behind_bg: false,
                palette: false,
                color: 0,
            }; 160]
        }
    }
}

struct WindowLineRenderer {
    enabled: bool,
    tile_set: bool,
    tile_map_line_offset: u16,
    screen_offset: u8,
    window_offset: u8,
    //tile_line: u8,
    tile_sub_line: u8,

    //tile_column: u8,
    //tile_sub_column: u8,
    //map_offset: u16,
    //tile: u8,
    tile_bit: u8,
    tile_data: (u8, u8),
}

impl PixelPipeline for WindowLineRenderer {
    fn init(_oam: &[Sprite; 40], video: &Video, line: u8) -> Self {
        video.with_reg(|reg| {
            let tile_line = line as u16 / 8;
            let tile_map = if reg.background_tile_map {
                0x1C00
            } else {
                0x1800
            };
            WindowLineRenderer {
                enabled: reg.enable_window && reg.window_scroll_x <= 167 && reg.scroll_y <= line,
                tile_set: reg.tile_set,
                tile_map_line_offset: tile_map + tile_line * 32,
                screen_offset: reg.window_scroll_x.saturating_sub(7),
                window_offset: 7u8.saturating_sub(reg.window_scroll_x),
                //tile_line,
                tile_sub_line: line % 8,

                //tile_column: 0,
                tile_bit: 8,
                //map_offset: 0,
                //tile: 0,
                tile_data: (0, 0),
            }
        })
    }
    fn refresh(&mut self, vram: &VideoRam, _oam: &[Sprite; 40], x: u8) {
        if self.tile_bit >= 8 {
            let column = x.saturating_sub(self.screen_offset);
            let tile_column = column / 8;
            self.tile_bit = column % 8;
            let map_offset = self.tile_map_line_offset + tile_column as u16;
            let tile = vram.vram[map_offset as usize];
            self.tile_data = vram.get_tile(self.tile_set, tile, self.tile_sub_line)
        }
    }
    fn pixel(&mut self, vram: &VideoRam, oam: &[Sprite; 40], video: &Video, x: u8) -> Option<u8> {
        if self.enabled && self.screen_offset >= x {
            self.refresh(vram, oam, x);
            let color = ((self.tile_data.0 >> self.tile_bit) & 1)
                + (((self.tile_data.1 >> self.tile_bit) & 1) * 2);
            self.tile_bit += 1;
            info!("window color is {}", color);
            Some(color)
        } else {
            None
        }
    }
}

fn tile_line_offset(line: u8, scroll_y: u8, tile_map: u16) -> u16 {
    let (bg_line, _) = line.overflowing_add(scroll_y);
    tile_map + (bg_line / 8) as u16 * 32
}

pub fn embedded_loop(
    ms: u32,
    fifo: &mut SioFifo,
    pio0: rp_pico::pac::PIO0,
    resets: &mut rp_pico::pac::RESETS,
    video: &RefCell<Video>,
    wait_sync_display: fn(),
    start_display: fn(u8, [u8; 4], &[u8; 240]),
    display_four_pixels: fn(u8, u8, [u8; 6]),
    push_display: fn(u8),
    end_display: fn(),
) {
    info!("START OF DISPLAY");
    let base = [0xf, 0x8, 0x4, 0];
    let base_up = [0xf0, 0x80, 0x40, 0];
    //    let cp = unsafe{cortex_m::Peripherals::steal()};
    let video = video.borrow();
    let mut display_started = false;
    let program = pio_proc::pio_asm!(
        "        ;pull -> osr ->out         in -> isr -> push

    public clean:
        pull noblock
        jmp clean
    public background:
        pull block ;number of column to skip -1
        mov x,osr
        pull block
        mov isr,::isr
        out null,16
        set y,7
    before_display:
        out null,1
        jmp y-- pixel_loop
        jmp x-- before_display
        jmp pixel_loop
     
    ;window entry point
    public window:
        pull block ; pull number of column to wait-1 for, if skipping, start as backgroud
        mov x,osr
        set y,0
    no_window:
        mov isr,y
        push block
        jmp x-- no_window
    ;jmp loop
    ;loop
    
    ;entry point if aligned
.wrap_target
    public tile_loop:
        pull block
        mov osr,::osr
        out null,16

        set y,7
    pixel_loop:
        in osr,1
        mov x,osr
        out null,8

        in osr,1
        mov osr,x
        out null,1
        push block
        jmp y-- pixel_loop
.wrap

    ;    in x,0
    ;    push block

    ",
        options(max_program_size = 32) // Optional, defaults to 32
    );
    let (mut pio, sm0, sm1, _, _) = pio0.split(resets);
    let installed = pio.install(&program.program).unwrap();
    let program_offset = installed.offset();

    // Sharing the program between two state machines wat? why is that unsafe ?
    let (mut window_sm, mut from_window, mut to_window) =
        rp_pico::hal::pio::PIOBuilder::from_program(unsafe { installed.share() })
            .in_shift_direction(ShiftDirection::Left)
            .out_shift_direction(ShiftDirection::Right)
            .autopull(false)
            .autopush(false)
            .build(sm1);
    let (mut background_sm, mut from_background, mut to_background) =
        rp_pico::hal::pio::PIOBuilder::from_program(installed)
            .in_shift_direction(ShiftDirection::Left)
            .out_shift_direction(ShiftDirection::Right)
            .autopull(false)
            .autopush(false)
            .build(sm0);

    loop {
        'screen: loop {
            //let mut pixel_buffer = [0u8; 6];
            //TODE send mode change interrupt
            //TODO send line interrupt
            //TODO send vblank
            //TODO set line registers
            let enabled = video.with_reg(|mut reg| {
                reg.video_mode = 1;
                reg.enable_lcd
            });

            if !enabled {
                if display_started {
                    //debug!("DISPLAY OFF");
                    Ipc::DisplayOff.send(fifo);
                    display_started = false;
                }
                cortex_m::asm::delay(ms / 64);
                continue;
            } else {
                if !display_started {
                    //debug!("DISPLAY ON");
                    Ipc::DisplayOn.send(fifo);
                    display_started = true;
                }
            }
            /*
            //start_display();
            background_sm.exec_instruction(
                pio::InstructionOperands::JMP {
                    condition: pio::JmpCondition::Always,
                    address: program.public_defines.clean as u8,
                }
                .encode(),
            );
            window_sm.exec_instruction(
                pio::InstructionOperands::JMP {
                    condition: pio::JmpCondition::Always,
                    address: program.public_defines.clean as u8,
                }
                .encode(),
            );*/
            let mut window_sm_started = window_sm.start();
            let mut background_sm_started = background_sm.start();

            for x in 0..32 {
                info!(
                    "{} {}",
                    background_sm_started.instruction_address(),
                    window_sm_started.instruction_address(),
                );
            }
            info!(
                "defines {} {} {} {}",
                program.public_defines.clean,
                program.public_defines.background,
                program.public_defines.window,
                program.public_defines.tile_loop
            );

            display_wait_sync();
            let mut line_buff = [[0u8; 240]; 2];
            'line: for l in 0..144u8 {
                let line_buff = &mut line_buff[l as usize & 1];
                //info!("linebuffer at {:x}", (&line_buff) as *const u8);

                //info!("line : {}", l);
                //            display_start_line(l, [l, 0, 0, 0]);
                {
                    let (
                        background_palette_bits,
                        sprite_palette_0_bits,
                        sprite_palette_1_bits,
                        mode2_interrupt,
                    ) = video.with_reg(|mut reg| {
                        reg.video_mode = 2;
                        reg.line = l;
                        (
                            reg.background_palette_bits,
                            reg.sprite_palette_0_bits,
                            reg.sprite_palette_1_bits,
                            reg.enable_mode_2_oam_check,
                        )
                    });

                    Ipc::Oam(mode2_interrupt).send(fifo);
                    let (
                        sprites,
                        (
                            background_x,
                            window_x,
                            background_y,
                            window_y,
                            tile_set,
                            background_tile_map,
                            window_tile_map,
                            background_enable,
                            window_enable,
                        ),
                    ) = video.with_oam(|oam| {
                        let mut sprites = SpriteRenderer::init(&oam, &video, l);
                        let ret = video.with_reg(|mut reg| {
                            reg.video_mode = 3;
                            (
                                reg.scroll_x,
                                reg.window_scroll_x,
                                reg.scroll_y,
                                reg.window_scroll_y,
                                reg.tile_set,
                                if reg.background_tile_map {
                                    0x1C00
                                } else {
                                    0x1800
                                },
                                if reg.window_tile_map { 0x1C00 } else { 0x1800 },
                                reg.enable_background,
                                reg.enable_window,
                            )
                        });
                        let sprites = video.with_ram(|vram| sprites.render(&vram));
                        (sprites, ret)
                    });
                    let mut bg_tile_column = background_x / 8;
                    let bg_tile_line_offset =
                        tile_line_offset(l, background_y, background_tile_map);

                    let mut window_tile_column = 0u8;
                    let window_tile_line_offset = tile_line_offset(l, window_y, window_tile_map);

                    video.with_ram(|vram| {
                        //let (vram, _spin2) = video.get_ram();
                        // mode 3
                        //let mut bg = BgLineRenderer::init(&oam, &video, l);
                        //let mut window = WindowLineRenderer::init(&oam, &video, l);
                        let mut even = 0;
                        to_background.drain_fifo();
                        to_window.drain_fifo();
                        let prev_background = background_sm_started.instruction_address();
                        let prev_window = window_sm_started.instruction_address();

                        let (background_offset, background_to_write) = match background_x % 8 {
                            0 => (program.public_defines.tile_loop, None),
                            x => (program.public_defines.background, Some(x - 1)),
                        };
                        cortex_m::asm::delay(ms / 10);
                        background_sm_started.exec_instruction(
                            pio::InstructionOperands::JMP {
                                condition: pio::JmpCondition::Always,
                                address: background_offset as u8 + program_offset,
                            }
                            .encode(),
                        );

                        let (window_offset, window_to_write) = match window_x {
                            _ if window_enable == false => {
                                (program.public_defines.window, Some(200))
                            }
                            7 => (program.public_defines.tile_loop, None),
                            x if x < 7 => (program.public_defines.background, Some(x)),
                            x => (program.public_defines.window, Some(x - 8)),
                        };
                        window_sm_started.exec_instruction(
                            pio::InstructionOperands::JMP {
                                condition: pio::JmpCondition::Always,
                                address: window_offset as u8 + program_offset,
                            }
                            .encode(),
                        );

                        info!(
                            "offsets {}->{}=={} {:?} {}->{}=={} {:?}",
                            prev_background,
                            background_offset,
                            background_sm_started.instruction_address(),
                            background_to_write,
                            prev_window,
                            window_offset,
                            window_sm_started.instruction_address(),
                            window_to_write
                        );

                        info!("cleaning");

                        while let Some(_) = from_background.read() {
                            //    info!("bgline {}", background_sm_started.instruction_address())
                        }
                        while let Some(_) = from_window.read() {
                            //    info!("winline {}", background_sm_started.instruction_address())
                        }
                        info!(
                            "point {} {}",
                            background_sm_started.instruction_address(),
                            window_sm_started.instruction_address()
                        );
                        info!("done cleaning");

                        if let Some(x) = background_to_write {
                            to_background.write(x);
                        }
                        if let Some(x) = window_to_write {
                            to_window.write(x);
                        }

                        let mut send_one_bg_tile = |vram: &VideoRam| {
                            let tile = vram.vram[bg_tile_column as usize];

                            let tile_data = vram.get_u16_tile(tile_set, tile, l);
                            info!("push bg {:04X}", tile_data);
                            to_background.write(tile_data);
                            bg_tile_column = (bg_tile_column + 1) % 32;
                        };
                        let mut send_one_window_tile = |vram: &VideoRam| {
                            let tile = vram.vram[window_tile_column as usize];

                            let tile_data = vram.get_u16_tile(tile_set, tile, l);
                            to_window.write(tile_data);
                            window_tile_column = (window_tile_column + 1) % 32;
                        };
                        send_one_bg_tile(&vram);
                        send_one_window_tile(&vram);
                        'pixel: for x in 0..160 {
                            info!("pixel {}", x);
                            if x % 8 == 0 {
                                if background_enable {
                                    send_one_bg_tile(&vram);
                                }
                                if window_enable && x >= window_x {
                                    send_one_window_tile(&vram);
                                }
                            }
                            while from_background.is_empty() {
                                panic!("bg empty")
                            }
                            while from_window.is_empty() {
                                panic!(
                                    "window empty {} {:?}",
                                    window_sm_started.instruction_address(),
                                    window_sm_started.stalled()
                                )
                            }

                            let bg = from_background.read().unwrap();
                            let win = from_window.read().unwrap();
                            info!("pixel {} {}/{}", x, bg, win);

                            let bw = if background_enable {
                                if window_enable {
                                    if win == 0 {
                                        bg
                                    } else {
                                        win
                                    }
                                } else {
                                    bg
                                }
                            } else {
                                0
                            } as u8;

                            let bw_val = apply_palette(bw, background_palette_bits);
                            let sprite = sprites[x as usize];
                            let sprite_val = apply_palette(
                                sprite.color,
                                if sprite.palette {
                                    sprite_palette_1_bits
                                } else {
                                    sprite_palette_0_bits
                                },
                            );
                            let pixel = if sprite.behind_bg {
                                if bw == 0 {
                                    sprite_val
                                } else {
                                    bw_val
                                }
                            } else {
                                if sprite.color == 0 {
                                    bw_val
                                } else {
                                    sprite_val
                                }
                            };
                            //info!("bg : {} pixel : {}", bw_val, pixel);
                            //line_buff[x as usize] = (pixel ^ 0b11) << 2;
                            /*
                                                        if true {
                                info!("pio?? {:?}", from_background.read());

                                info!("offset {} {}", background_offset, background_x);
                                to_background.write(0xffffFFFFu32);
                                to_background.write(0x00FFFF00u32);
                                background_sm.start();
                                for i in 0..16 {
                                    cortex_m::asm::delay(8 * 8 * 2 * 2);
                                    info!("{:X}", from_background.read().unwrap());
                                }
                                panic!("end of test");
                            }
                                match x & 0b11 {
                                    0 => {
                                        pixel_buffer[0] =
                                            base_up[pixel as usize] | base[pixel as usize];
                                        even = pixel;
                                    }
                                    1 => {
                                        pixel_buffer[1] =
                                            base_up[even as usize] | base[pixel as usize];
                                        pixel_buffer[2] =
                                            base_up[pixel as usize] | base[pixel as usize];
                                    }
                                    2 => {
                                        pixel_buffer[3] =
                                            base_up[pixel as usize] | base[pixel as usize];
                                        even = pixel;
                                    }
                                    3 => {
                                        pixel_buffer[4] =
                                            base_up[even as usize] | base[pixel as usize];
                                        pixel_buffer[5] =
                                            base_up[pixel as usize] | base[pixel as usize];
                                        display_four_pixels(l, x ^ 0x11, pixel_buffer);
                                    }
                                    _ => panic!("IMpossible it should be"),
                                }*/
                            if x & 1 == 0 {
                                //even line, save data
                                even = pixel; //save 1/3 pixel
                            } else {
                                //odd line, send data
                                line_buff[(x as usize >> 1) * 3] =
                                    base_up[even as usize] | base[even as usize];
                                line_buff[(x as usize >> 1) * 3 + 1] =
                                    base_up[even as usize] | base[pixel as usize];
                                line_buff[(x as usize >> 1) * 3 + 2] =
                                    base_up[pixel as usize] | base[pixel as usize];
                                //send saved 1/3 pixelg
                            }
                            //push_display(base_up[pixel as usize] | base[pixel as usize]);
                        }
                    });

                    cortex_m::asm::delay(8); //8 level buffer, 8 bits, 2 cpu clocks per bit, 2 to be sure;

                    //});
                    //let (oam, _spin1) = video.get_oam();
                    // mode 2
                }
                let interrupt_hblank = video.with_reg(|mut reg| {
                    reg.video_mode = 0;
                    reg.enable_mode_0_hblank_check
                });
                Ipc::Hblank(interrupt_hblank).send(fifo);
                //info!("{}", line_buff);
                display_line(l as u8, [l as u8, 0, 1, 0], line_buff);

                //display_end();
                //cortex_m::asm::delay(20 * ms / 1000);
                // mode 0
            }
            background_sm = background_sm_started.stop();
            window_sm = window_sm_started.stop();

            for l in 144..153 {
                let (interrupt_vblank, line) = video.with_reg(|mut reg| {
                    reg.video_mode = 1;
                    reg.line = l;
                    (reg.enable_mode_1_vblank_check, (l == 144))
                });
                if line {
                    Ipc::VBlank(interrupt_vblank).send(fifo);
                }
                cortex_m::asm::delay(ms / 10);
            }
        }
    }
}
