#![crate_type = "lib"]
#![crate_name = "chip_8"]

extern crate rand;
extern crate time;
use rand::Rng;
use std::fs::File;
use std::io::Read;
use std::num::Wrapping;

use time::*;

fn hex_nibble(opcode: u16, nibble: u8) -> u16 {
    (opcode & (0xf << (4 * nibble))) >> (4 * nibble)
}

fn lsb(x: u16) -> u16 {
    hex_nibble(x, 0)
}

fn gsb(x: u16) -> u16 {
    hex_nibble(x, 3)
}

fn left_truncate(opcode: u16, nibbles_to_drop: u16) -> u16 {
    opcode >> (4 * nibbles_to_drop)
}

pub struct Chip8 {
    memory: [u8; 4096],
    v: [u8; 16],
    i:  u16,
    pc: u16,
    stack: Vec<u16>,
    screen: [bool; 64*32],
    sound: u8,
    delay: u8,
    keys: [bool; 16],
    locked: bool,
    locked_register: u8,
    last_tick: time::Tm,
}

impl Chip8 {
    
    pub fn new() -> Chip8 {
		let mut chip = Chip8 {
			memory: [0; 4096],
			v: [0; 16],
			i: 0,
			pc: 0x200,
			stack: Vec::new(),
			screen: [false; 64*32],
			sound: 0,
			delay: 0,
			keys: [false; 16],
			locked: false,
			locked_register: 0,
			last_tick: time::now_utc(),
		};
		chip.load_font_system();
		chip
    }
    
    pub fn get_current_state(self) -> Self {
        self
    }
    
    pub fn eval_opcode(&mut self, opcode: u16) -> () {
        if !self.locked {
            let gsb = gsb(opcode);
            match gsb {
                0x0 => {
                    if hex_nibble(opcode, 0) == 0x0 {
                        for x in 0..self.screen.len() {
                            self.screen[x] = false;
                        }
                    } else{
                        if self.stack.len() > 0 {
                            self.pc = self.stack.pop().unwrap()-2;
                        }
                    }
                }
                0x1 => {
                    self.pc = left_truncate(opcode, 1) - 2;
                }
                0x2 => {
                    self.stack.push(self.pc);
                    self.pc = left_truncate(opcode, 1) - 2;
                }
                0x3 => {
                    let register = hex_nibble(opcode, 2);
                    let num = left_truncate(opcode, 2) as u8;
                    if self.v[register as usize] == num {
                        self.pc += 2;
                    }
                }
                0x4 => {
                    let register = hex_nibble(opcode, 2);
                    let num = left_truncate(opcode, 2) as u8;
                    if self.v[register as usize] != num {
                        self.pc += 2;
                    }
                }
                0x5 => {
                    let vx = hex_nibble(opcode, 2);
                    let vy = hex_nibble(opcode, 1);
                    if vx == vy {
                        self.pc += 2;
                    }            
                }
                0x6 => {
                    let register = hex_nibble(opcode, 2);
                    let num = left_truncate(opcode, 2) as u8;
                    self.v[register as usize] = num;
                }
                0x7 => {
                    let register = hex_nibble(opcode, 2);
                    let num = left_truncate(opcode, 2) as u8;
                    self.v[register as usize] += num;
                }
                0x8 => {
                    let id = lsb(opcode);
                    let vx = hex_nibble(opcode, 2) as usize;
                    let vy = hex_nibble(opcode, 1) as usize;
                    match id {
                        0   => self.v[vx] =  self.v[vy],
                        1   => self.v[vx] |= self.v[vy],
                        2   => self.v[vx] &= self.v[vy],
                        3   => self.v[vx] ^= self.v[vy],
                        //unimpelented for the rest of this
                        4   => {
                            let vx_wrapping = Wrapping(self.v[vx]);
                            let vy_wrapping = Wrapping(self.v[vy]);
                            
                            let result = vx_wrapping + vy_wrapping;
                            let val = result.0;
                            self.v[0xf] = if val < self.v[vx] || val < self.v[vy] {
                                    1
                                } else {
                                    0
                                };
                            self.v[vx] = val;
                        },
                        5   => {
                            let vx_wrapping = Wrapping(self.v[vx]);
                            let vy_wrapping = Wrapping(self.v[vy]);
                            let result = vx_wrapping - vy_wrapping;
                            let val = result.0;
                            self.v[0xf] = if val < self.v[vx] {
                                    1
                                } else {
                                    0
                                };
                            self.v[vx] = val;
                        },//borrow
                        6   => {
                            let vx_wrapping = Wrapping(self.v[vx]);
                            self.v[0xf] = self.v[vx] & 0x1;
                            let result = vx_wrapping >> 1;
                            let val = result.0;
                            self.v[vx] = val;                        
                        },//LSB
                        7   => {
                            let vx_wrapping = Wrapping(self.v[vx]);
                            let vy_wrapping = Wrapping(self.v[vy]);
                            let result = vy_wrapping - vx_wrapping;
                            let val = result.0;
                            self.v[0xf] = if self.v[vy] > self.v[vx] {
                                    1
                                } else {
                                    0
                                };
                            self.v[vx] = val;                        
                        },//Borrow
                        0xE => {
                            let vx_wrapping = Wrapping(self.v[vx]);
                            self.v[0xf] = self.v[vx] >> 7;
                            let result = vx_wrapping >> 1;
                            let val = result.0;
                            self.v[vx] = val;                        
                        },//MSB
                        _ => println!("Unknown Opcode: {}", opcode), 
                    }
                }
                0x9 => {
                    if self.v[hex_nibble(opcode, 2) as usize] != self.v[hex_nibble(opcode, 1) as usize] {
                        self.pc += 2;
                    }
                }
                0xa => {
                    self.i = left_truncate(opcode, 1);
                }
                0xb => {
                    self.pc = self.v[0 as usize] as u16 + left_truncate(opcode, 1);
                }
                0xc => {
                    let mut rng = rand::thread_rng();
                    self.v[hex_nibble(opcode, 2) as usize] = left_truncate(opcode, 2) as u8 & rng.gen::<u8>(); 
                }
                0xd => {
                    let x = hex_nibble(opcode, 2);
                    let y = hex_nibble(opcode, 1);
                    let height = hex_nibble(opcode, 0);
                    let mut turned_off = false;
                    for y_loc in y..y+height {
                        let val = format!("{:08b}", self.memory[(self.i + y_loc * 64 + x) as usize]);
                        let mut x_pos: u16 = x;
                        for c in val.chars() {
                            if c == '1'{
                                self.screen[((y_loc % 32) * 64 + ((x + x_pos) % 64)) as usize] ^= true;
                                turned_off |= !self.screen[((y_loc % 32) * 64 + ((x + x_pos) % 64)) as usize];
                            }
                            x_pos += 1;
                        }
                    }
                    self.v[0xf] = 0;
                    if turned_off {
                        self.v[0xf] = 1;
                    }
                }
                0xe => {
                    match hex_nibble(opcode, 0){
                        0x1 => {
                            if self.keys[self.v[hex_nibble(opcode, 2) as usize] as usize]{
                                self.pc += 2;
                            }
                        }
                        0xe => {
                            if !self.keys[self.v[hex_nibble(opcode, 2) as usize] as usize]{
                                self.pc += 2;
                            }
                        }
                        _ => println!("Unknown Opcode: {}", opcode),
                    }
                }
                0xf => {
                    match hex_nibble(opcode, 0){
                        0x3 => {
                            let digit = format!("{:03}", self.v[(hex_nibble(opcode, 2) & 0xf) as usize]);
                            let mut count = 0;
                            for c in digit.chars(){
                                self.memory[(self.i + count) as usize] = c.to_digit(10).unwrap() as u8;
                                count += 1;
                            }
                        },
                        0x5 => {
                            match hex_nibble(opcode, 1){
                                1 => {
                                    self.delay = self.v[hex_nibble(opcode, 2) as usize];
                                },
                                5 => {
                                    let loc = self.i;
                                    let reg = hex_nibble(opcode, 2);
                                    for a in 0..reg{
                                        self.memory[(a+loc) as usize] = self.v[a as usize];
                                    }
                                },
                                6 => {
                                    let loc = self.i;
                                    let reg = hex_nibble(opcode, 2);
                                    for a in 0..reg{
                                        self.v[a as usize] = self.memory[(a+loc) as usize];
                                    }
                                },
                                _ => println!("Unknown Opcode: {}", opcode),
                            }
                        },
                        0x7 => self.v[hex_nibble(opcode, 2) as usize] = self.delay,
                        0x8 => self.sound = self.v[hex_nibble(opcode, 2) as usize],
                        0x9 => self.i = self.v[hex_nibble(opcode, 2) as usize] as u16 * 5,
                        0xa => {
                            self.locked = true;
                            self.locked_register = hex_nibble(opcode, 2) as u8;
                        },
                        0xe => self.i += self.v[hex_nibble(opcode, 2) as usize] as u16,
                        _ => println!("Unknown Opcode: {}", opcode),
                    }
                }
                _ => println!("Unknown Opcode: {}", opcode),
            }
            self.pc += 2;
        }
    }
    
    pub fn press_button(&mut self, x : u8) {
        self.keys[x as usize] = true;
        if self.locked {
            self.v[self.locked_register as usize] = x;
            self.locked = false;
        }
    }
    
    pub fn release_button(&mut self, x : u8) {
        self.keys[x as usize] = false;
    }
    
    pub fn get_screen(self) -> [bool; 64*32] {
        self.screen
    }
    
    fn load_font_system(&mut self) {
    	let fonts = include_bytes!("chip8.rom");
        for x in 0..fonts.len() {
            self.memory[x as usize] = fonts[x as usize];
        }
    }
    
    pub fn load_rom(&mut self, file_name: &str) {
        self.reset_system();
        let mut file = File::open(file_name).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        for (offset, value) in data.into_iter().enumerate() {
            self.memory[offset + 0x200 as usize] = value as u8;
        }
    }
    
    fn reset_system(&mut self) {
        self.memory = [0; 4096];
        self.v = [0; 16];
		self.i = 0;
		self.pc = 0x200;
        self.stack.clear();
        self.screen = [false; 64*32];
        self.sound = 0;
        self.delay = 0;
        self.keys = [false; 16];
        self.locked = false;
        self.locked_register = 0;
        self.load_font_system(); 
    }
    
    pub fn step(&mut self) {
        let opcode = (self.memory[self.pc as usize] as u16) << 8 + self.memory[(self.pc + 2) as usize] as u16;
        self.eval_opcode(opcode);
        let dur = time::now_utc() - self.last_tick;
        if dur.num_seconds() >= 1 {
            let num = dur.num_seconds();
            if num <= 0 || num > (!0 as u8) as i64 {
                self.delay = 0;
                self.sound = 0;
            } else {
                self.delay = if num > self.delay as i64 {
                    0
                } else {
                    self.delay - num as u8
                };
                self.sound = if num > self.sound as i64 { 
                    0
                } else {
                    self.sound - num as u8
                };
            }
            self.last_tick = time::now_utc();
        }
    }

}