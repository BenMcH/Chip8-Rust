extern crate rand;
use rand::Rng;
use std::num::Wrapping;

macro_rules! hex_nibble{
	($x:expr, $y:expr) => (($x & (0xf << (4 * $y))) >> (4 * $y));
}

macro_rules! lsb{
	($x:expr) => (hex_nibble!($x, 0));
}

macro_rules! gsb{
	($x:expr) => (hex_nibble!($x, 3));
}

macro_rules! left_truncate {
	($x:expr, $y:expr) => ($x >> (4 * $y));
}

struct Chip8{
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
}

impl Chip8{
	
	fn get_current_state(self) -> Self {
		self
	}
	
	fn eval_opcode(&mut self, opcode: u16) -> () {
		if !self.locked {
			let gsb = gsb!(opcode);
			match gsb {
				0x0 => {
					if hex_nibble!(opcode, 0) == 0x0 {
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
					self.pc = left_truncate!(opcode, 1) - 2;
				}
				0x2 => {
					self.stack.push(self.pc);
					self.pc = left_truncate!(opcode, 1) - 2;
				}
				0x3 => {
					let register = hex_nibble!(opcode, 2);
					let num = left_truncate!(opcode, 2) as u8;
					if self.v[register as usize] == num {
						self.pc += 2;
					}
				}
				0x4 => {
					let register = hex_nibble!(opcode, 2);
					let num = left_truncate!(opcode, 2) as u8;
					if self.v[register as usize] != num {
						self.pc += 2;
					}
				}
				0x5 => {
					let vx = hex_nibble!(opcode, 2);
					let vy = hex_nibble!(opcode, 1);
					if vx == vy {
						self.pc += 2;
					}			
				}
				0x6 => {
					let register = hex_nibble!(opcode, 2);
					let num = left_truncate!(opcode, 2) as u8;
					self.v[register as usize] = num;
				}
				0x7 => {
					let register = hex_nibble!(opcode, 2);
					let num = left_truncate!(opcode, 2) as u8;
					self.v[register as usize] += num;
				}
				0x8 => {
					let id = lsb!(opcode);
					let vx = hex_nibble!(opcode, 2) as usize;
					let vy = hex_nibble!(opcode, 1) as usize;
					match id {
						0   => self.v[vx] =  self.v[vy],
						1   => self.v[vx] |= self.v[vy],
						2   => self.v[vx] &= self.v[vy],
						3   => self.v[vx] ^= self.v[vy],
						//unimpelented for the rest of this
						4   => {
							let mut vx_wrapping = Wrapping(self.v[vx]);
							let mut vy_wrapping = Wrapping(self.v[vy]);
							
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
							let mut vx_wrapping = Wrapping(self.v[vx]);
							let mut vy_wrapping = Wrapping(self.v[vy]);
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
							let mut vx_wrapping = Wrapping(self.v[vx]);
							self.v[0xf] = self.v[vx] & 0x1;
							let result = vx_wrapping >> 1;
							let val = result.0;
							self.v[vx] = val;						
						},//LSB
						7   => {
							let mut vx_wrapping = Wrapping(self.v[vx]);
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
							let mut vx_wrapping = Wrapping(self.v[vx]);
							self.v[0xf] = self.v[vx] >> 15;
							let result = vx_wrapping >> 1;
							let val = result.0;
							self.v[vx] = val;						
						},//MSB
						_ => println!("Unknown Opcode: {}", opcode), 
					}
				}
				0x9 => {
					if self.v[hex_nibble!(opcode, 2) as usize] != self.v[hex_nibble!(opcode, 1) as usize] {
						self.pc += 2;
					}
				}
				0xa => {
					self.i = left_truncate!(opcode, 1);
				}
				0xb => {
					self.pc = self.v[0 as usize] as u16 + left_truncate!(opcode, 1);
				}
				0xc => {
					let mut rng = rand::thread_rng();
					self.v[hex_nibble!(opcode, 2) as usize] = left_truncate!(opcode, 2) as u8 & rng.gen::<u8>(); 
				}
				0xd => {
					let x = hex_nibble!(opcode, 2);
					let y = hex_nibble!(opcode, 1);
					let height = hex_nibble!(opcode, 0);
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
					match hex_nibble!(opcode, 0){
						0x1 => {
							if self.keys[self.v[hex_nibble!(opcode, 2) as usize] as usize]{
								self.pc += 2;
							}
						}
						0xe => {
							if !self.keys[self.v[hex_nibble!(opcode, 2) as usize] as usize]{
								self.pc += 2;
							}
						}
						_ => println!("Unknown Opcode: {}", opcode),
					}
				}
				0xf => {
					match hex_nibble!(opcode, 0){
						0x3 => {
							let digit = format!("{:03}", self.v[(hex_nibble!(opcode, 2) & 0xf) as usize]);
							let mut count = 0;
							for c in digit.chars(){
								self.memory[(self.i + count) as usize] = c.to_digit(10).unwrap() as u8;
								count += 1;
							}
						},
						0x5 => {
							match hex_nibble!(opcode, 1){
								1 => {
									self.delay = self.v[hex_nibble!(opcode, 2) as usize];
								},
								5 => {
									let loc = self.i;
									let reg = hex_nibble!(opcode, 2);
									for a in 0..reg{
										self.memory[(a+loc) as usize] = self.v[a as usize];
									}
								},
								6 => {
									let loc = self.i;
									let reg = hex_nibble!(opcode, 2);
									for a in 0..reg{
										self.v[a as usize] = self.memory[(a+loc) as usize];
									}
								},
								_ => println!("Unknown Opcode: {}", opcode),
							}
						},
						0x7 => self.v[hex_nibble!(opcode, 2) as usize] = self.delay,
						0x8 => self.sound = self.v[hex_nibble!(opcode, 2) as usize],
						0x9 => self.i = self.v[hex_nibble!(opcode, 2) as usize] as u16 * 5,
						0xa => {
							self.locked = true;
							self.locked_register = hex_nibble!(opcode, 2) as u8;
						},
						0xe => self.i += self.v[hex_nibble!(opcode, 2) as usize] as u16,
						_ => println!("Unknown Opcode: {}", opcode),
					}
				}
				_ => println!("Unknown Opcode: {}", opcode),
			}
			self.pc += 2;
		}
	}
	
	fn press_button(&mut self, x : u8){
		self.keys[x as usize] = true;
		if self.locked {
			self.v[self.locked_register as usize] = x;
			self.locked = false;
		}
	}
	
	fn release_button(&mut self, x : u8){
		self.keys[x as usize] = false;
	}
	
	fn get_screen(&mut self) -> [bool; 64*32] {
		self.screen
	}
	
}