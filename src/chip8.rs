use anyhow::Result;
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[allow(dead_code)]
const MEMORY_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
const STACK_SIZE: usize = 16;
const KEY_COUNT: usize = 16;
const VIDEO_WIDTH: usize = 64;
const VIDEO_HEIGHT: usize = 32;
const VIDEO_SIZE: usize = VIDEO_WIDTH * VIDEO_HEIGHT;

const START_ADDRESS: u16 = 0x200;
const FONTSET_SIZE: usize = 80;
const FONTSET_START_ADDRESS: u16 = 0x50;

const FONTSET: [u8; FONTSET_SIZE] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

#[allow(dead_code)]
pub struct Chip8 {
    registers: [u8; REGISTER_COUNT],
    memory: [u8; MEMORY_SIZE],
    index: u16,
    pc: u16,
    stack: [u16; STACK_SIZE],
    sp: u8,
    delay_timer: u8,
    sound_timer: u8,
    keypad: [bool; KEY_COUNT],
    video: [u32; VIDEO_SIZE],
    opcode: u16,
    rng: StdRng,
    debug: bool,
}

#[allow(dead_code)]
impl Chip8 {
    pub fn new() -> Self {
        let mut chip8 = Self {
            registers: [0; REGISTER_COUNT],
            memory: [0; MEMORY_SIZE],
            index: 0,
            pc: START_ADDRESS,
            stack: [0; STACK_SIZE],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [false; KEY_COUNT],
            video: [0; VIDEO_SIZE],
            opcode: 0,
            rng: StdRng::seed_from_u64(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64,
            ),
            debug: true, // Enable debug output initially
        };

        chip8.load_fontset();
        chip8
    }

    pub fn enable_debug(&mut self, enabled: bool) {
        self.debug = enabled;
    }

    fn debug_print(&self, message: &str) {
        if self.debug {
            println!("DEBUG: {}", message);
        }
    }

    fn load_fontset(&mut self) {
        let start = FONTSET_START_ADDRESS as usize;
        for (i, &byte) in FONTSET.iter().enumerate() {
            self.memory[start + i] = byte;
        }
        self.debug_print(&format!("Loaded fontset at 0x{:03X}", start));
    }

    pub fn load_rom(&mut self, filename: &str) -> Result<()> {
        let rom_data = fs::read(filename)?;

        let start = START_ADDRESS as usize;
        if rom_data.len() > (MEMORY_SIZE - start) {
            return Err(anyhow::anyhow!("ROM too large to fit in memory"));
        }

        for (i, &byte) in rom_data.iter().enumerate() {
            self.memory[start + i] = byte;
        }

        self.debug_print(&format!("Loaded ROM: {} bytes at 0x{:03X}", rom_data.len(), start));
        Ok(())
    }

    pub fn random_byte(&mut self) -> u8 {
        self.rng.random::<u8>()
    }

    pub fn get_display(&self) -> &[u32] {
        &self.video
    }

    pub fn set_keys(&mut self, keys: &[bool; KEY_COUNT]) {
        self.keypad = *keys;
    }

    // Fetch -> Decode -> Execute
    pub fn cycle(&mut self) {
        // Check if PC is in valid range
        if (self.pc as usize) >= MEMORY_SIZE - 1 {
            self.debug_print(&format!("PC out of bounds: 0x{:03X}", self.pc));
            return;
        }

        let high_byte = self.memory[self.pc as usize] as u16;
        let low_byte = self.memory[(self.pc + 1) as usize] as u16;
        self.opcode = (high_byte << 8) | low_byte;

        self.debug_print(&format!("PC: 0x{:03X}, Opcode: 0x{:04X}", self.pc, self.opcode));

        self.pc += 2;

        match (self.opcode & 0xF000) >> 12 {
            0x0 => self.execute_0xxx(),
            0x1 => self.op_1nnn(), // JP addr
            0x2 => self.op_2nnn(), // CALL addr
            0x3 => self.op_3xkk(), // SE Vx, byte
            0x4 => self.op_4xkk(), // SNE Vx, byte
            0x5 => self.op_5xy0(), // SE Vx, Vy
            0x6 => self.op_6xkk(), // LD Vx, byte
            0x7 => self.op_7xkk(), // ADD Vx, byte
            0x8 => self.execute_8xxx(),
            0x9 => self.op_9xy0(), // SNE Vx, Vy
            0xA => self.op_annn(), // LD I, addr
            0xB => self.op_bnnn(), // JP V0, addr
            0xC => self.op_cxkk(), // RND Vx, byte
            0xD => self.op_dxyn(), // DRW Vx, Vy, nibble
            0xE => self.execute_exxx(),
            0xF => self.execute_fxxx(),
            _ => {
                println!("Unknown opcode: 0x{:04X}", self.opcode);
            }
        }

        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    fn execute_0xxx(&mut self) {
        match self.opcode & 0x00FF {
            0xE0 => self.op_00e0(), // CLS
            0xEE => self.op_00ee(), // RET
            _ => {
                println!("Unknown 0xxx opcode: 0x{:04X}", self.opcode);
            }
        }
    }

    fn execute_8xxx(&mut self) {
        match self.opcode & 0x000F { // Fixed: should check last nibble, not last byte
            0x0 => self.op_8xy0(), // LD Vx, Vy
            0x1 => self.op_8xy1(), // OR Vx, Vy
            0x2 => self.op_8xy2(), // AND Vx, Vy
            0x3 => self.op_8xy3(), // XOR Vx, Vy
            0x4 => self.op_8xy4(), // ADD Vx, Vy
            0x5 => self.op_8xy5(), // SUB Vx, Vy
            0x6 => self.op_8xy6(), // SHR Vx
            0x7 => self.op_8xy7(), // SUBN Vx, Vy
            0xE => self.op_8xye(), // SHL Vx
            _ => {
                println!("Unknown 8xxx opcode: 0x{:04X}", self.opcode);
            }
        }
    }

    fn execute_exxx(&mut self) {
        match self.opcode & 0x00FF {
            0x9E => self.op_ex9e(), // SKP Vx
            0xA1 => self.op_exa1(), // SKNP Vx
            _ => {
                println!("Unknown Exxx opcode: 0x{:04X}", self.opcode);
            }
        }
    }

    fn execute_fxxx(&mut self) {
        match self.opcode & 0x00FF {
            0x07 => self.op_fx07(), // LD Vx, DT
            0x0A => self.op_fx0a(), // LD Vx, K
            0x15 => self.op_fx15(), // LD DT, Vx
            0x18 => self.op_fx18(), // LD ST, Vx
            0x1E => self.op_fx1e(), // ADD I, Vx
            0x29 => self.op_fx29(), // LD F, Vx
            0x33 => self.op_fx33(), // LD B, Vx
            0x55 => self.op_fx55(), // LD [I], Vx
            0x65 => self.op_fx65(), // LD Vx, [I]
            _ => {
                println!("Unknown Fxxx opcode: 0x{:04X}", self.opcode);
            }
        }
    }

    // ===== INSTRUCTIONS =====

    // 00E0: CLS Clear the display.
    fn op_00e0(&mut self) {
        self.video = [0; VIDEO_SIZE];
        self.debug_print("Cleared display");
    }

    // 00EE: RET Return from a subroutine.
    fn op_00ee(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
        self.debug_print(&format!("RET to 0x{:03X}", self.pc));
    }

    // 1nnn: JP addr Jump to location nnn.
    fn op_1nnn(&mut self) {
        let address = self.opcode & 0x0FFF;
        self.debug_print(&format!("JP to 0x{:03X}", address));
        self.pc = address;
    }

    // 2nnn: CALL addr Call subroutine at nnn.
    fn op_2nnn(&mut self) {
        let address = self.opcode & 0x0FFF;
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = address;
        self.debug_print(&format!("CALL 0x{:03X}", address));
    }

    // 3xkk - SE Vx, byte Skip next instruction if Vx = kk.
    fn op_3xkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        if self.registers[vx] == byte {
            self.pc += 2;
            self.debug_print(&format!("SE V{:X}, 0x{:02X} - SKIP", vx, byte));
        } else {
            self.debug_print(&format!("SE V{:X}, 0x{:02X} - NO SKIP", vx, byte));
        }
    }

    // 4xkk - SNE Vx, byte Skip next instruction if Vx != kk.
    fn op_4xkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        if self.registers[vx] != byte {
            self.pc += 2;
            self.debug_print(&format!("SNE V{:X}, 0x{:02X} - SKIP", vx, byte));
        } else {
            self.debug_print(&format!("SNE V{:X}, 0x{:02X} - NO SKIP", vx, byte));
        }
    }

    // 5xy0 - SE Vx, Vy Skip next instruction if Vx = Vy.
    fn op_5xy0(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        if self.registers[vx] == self.registers[vy] {
            self.pc += 2;
        }
        self.debug_print(&format!("SE V{:X}, V{:X}", vx, vy));
    }

    // 6xkk - LD Vx, byte, Set Vx = kk.
    fn op_6xkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        self.registers[vx] = byte;
        self.debug_print(&format!("LD V{:X}, 0x{:02X}", vx, byte));
    }

    // 7xkk - ADD Vx, byte, Set Vx = Vx + kk.
    fn op_7xkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        self.registers[vx] = self.registers[vx].wrapping_add(byte);
        self.debug_print(&format!("ADD V{:X}, 0x{:02X}", vx, byte));
    }

    // 8xy0 - LD Vx, Vy, Set Vx = Vy.
    fn op_8xy0(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[vx] = self.registers[vy];
        self.debug_print(&format!("LD V{:X}, V{:X}", vx, vy));
    }

    // 8xy1 - OR Vx, Vy, Set Vx = Vx OR Vy.
    fn op_8xy1(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[vx] |= self.registers[vy];
        self.debug_print(&format!("OR V{:X}, V{:X}", vx, vy));
    }

    // 8xy2 - AND Vx, Vy, Set Vx = Vx AND Vy.
    fn op_8xy2(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[vx] &= self.registers[vy];
        self.debug_print(&format!("AND V{:X}, V{:X}", vx, vy));
    }

    // 8xy3 - XOR Vx, Vy, Set Vx = Vx XOR Vy.
    fn op_8xy3(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[vx] ^= self.registers[vy];
        self.debug_print(&format!("XOR V{:X}, V{:X}", vx, vy));
    }

    // 8xy4 - ADD Vx, Vy, Set Vx = Vx + Vy, set VF = carry.
    fn op_8xy4(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        let sum = self.registers[vx] as u16 + self.registers[vy] as u16;

        self.registers[0xF] = if sum > 255 { 1 } else { 0 };
        self.registers[vx] = (sum & 0xFF) as u8;
        self.debug_print(&format!("ADD V{:X}, V{:X}", vx, vy));
    }

    // 8xy5 - SUB Vx, Vy, Set Vx = Vx - Vy, set VF = NOT borrow.
    fn op_8xy5(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[0xF] = if self.registers[vx] > self.registers[vy] {
            1
        } else {
            0
        };

        self.registers[vx] = self.registers[vx].wrapping_sub(self.registers[vy]);
        self.debug_print(&format!("SUB V{:X}, V{:X}", vx, vy));
    }

    // 8xy6 - SHR Vx, Set Vx = Vx SHR 1.
    fn op_8xy6(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;

        self.registers[0xF] = self.registers[vx] & 0x1;
        self.registers[vx] >>= 1;
        self.debug_print(&format!("SHR V{:X}", vx));
    }

    // 8xy7 - SUBN Vx, Vy, Set Vx = Vy - Vx, set VF = NOT borrow.
    fn op_8xy7(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[0xF] = if self.registers[vy] > self.registers[vx] {
            1
        } else {
            0
        };

        self.registers[vx] = self.registers[vy].wrapping_sub(self.registers[vx]);
        self.debug_print(&format!("SUBN V{:X}, V{:X}", vx, vy));
    }

    // 8xyE - SHL Vx {, Vy}, Set Vx = Vx SHL 1.
    fn op_8xye(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;

        self.registers[0xF] = (self.registers[vx] & 0x80) >> 7;
        self.registers[vx] <<= 1;
        self.debug_print(&format!("SHL V{:X}", vx));
    }

    // 9xy0 - SNE Vx, Vy, Skip next instruction if Vx != Vy.
    fn op_9xy0(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        if self.registers[vx] != self.registers[vy] {
            self.pc += 2;
        }
        self.debug_print(&format!("SNE V{:X}, V{:X}", vx, vy));
    }

    // Annn - LD I, addr, Set I = nnn.
    fn op_annn(&mut self) {
        let address = self.opcode & 0x0FFF;
        self.index = address;
        self.debug_print(&format!("LD I, 0x{:03X}", address));
    }

    // Bnnn - JP V0, addr, Jump to location nnn + V0.
    fn op_bnnn(&mut self) {
        let address = self.opcode & 0x0FFF;
        self.pc = address + self.registers[0] as u16;
        self.debug_print(&format!("JP V0, 0x{:03X}", address));
    }

    // Cxkk - RND Vx, byte, Set Vx = random byte AND kk.
    fn op_cxkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        self.registers[vx] = self.random_byte() & byte;
        self.debug_print(&format!("RND V{:X}, 0x{:02X}", vx, byte));
    }

    // Dxyn - DRW Vx, Vy, nibble
    // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
    fn op_dxyn(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;
        let height = (self.opcode & 0x000F) as usize;

        let x_pos = self.registers[vx] as usize % VIDEO_WIDTH;
        let y_pos = self.registers[vy] as usize % VIDEO_HEIGHT;

        self.debug_print(&format!("DRW V{:X}, V{:X}, {} at ({}, {})", vx, vy, height, x_pos, y_pos));

        self.registers[0xF] = 0; // Clear collision flag

        for row in 0..height {
            let sprite_byte = self.memory[(self.index + row as u16) as usize];
            self.debug_print(&format!("  Row {}: 0b{:08b} (0x{:02X})", row, sprite_byte, sprite_byte));

            for col in 0..8 {
                let sprite_pixel = sprite_byte & (0x80 >> col);

                if sprite_pixel == 0 { continue; }
                if (x_pos + col) >= VIDEO_WIDTH { continue; }
                if (y_pos + row) >= VIDEO_HEIGHT { continue; }

                let screen_pixel_index = (y_pos + row) * VIDEO_WIDTH + (x_pos + col);
                if self.video[screen_pixel_index] == 0xFFFFFFFF {
                    self.registers[0xF] = 1;
                }
                self.video[screen_pixel_index] ^= 0xFFFFFFFF;
            }
        }

        // Count pixels that are on for debugging
        let pixels_on = self.video.iter().filter(|&&p| p == 0xFFFFFFFF).count();
        self.debug_print(&format!("  Pixels on after draw: {}", pixels_on));
    }

    // Ex9E - SKP Vx, Skip next instruction if key with the value of Vx is pressed.
    fn op_ex9e(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let key = self.registers[vx] as usize;

        if key < KEY_COUNT && self.keypad[key] {
            self.pc += 2;
        }
        self.debug_print(&format!("SKP V{:X}", vx));
    }

    // ExA1 - SKNP Vx, Skip next instruction if key with the value of Vx is not pressed.
    fn op_exa1(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let key = self.registers[vx] as usize;

        if key >= KEY_COUNT || !self.keypad[key] {
            self.pc += 2;
        }
        self.debug_print(&format!("SKNP V{:X}", vx));
    }

    // Fx07 - LD Vx, DT, Set Vx = delay timer value.
    fn op_fx07(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        self.registers[vx] = self.delay_timer;
        self.debug_print(&format!("LD V{:X}, DT", vx));
    }

    // Fx0A - LD Vx, K, Wait for a key press, store the value of the key in Vx.
    fn op_fx0a(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;

        for (i, &key_pressed) in self.keypad.iter().enumerate() {
            if key_pressed {
                self.registers[vx] = i as u8;
                self.debug_print(&format!("LD V{:X}, K (key {})", vx, i));
                return;
            }
        }

        self.pc -= 2;
        self.debug_print(&format!("LD V{:X}, K (waiting)", vx));
    }

    // Fx15 - LD DT, Vx, Set delay timer = Vx.
    fn op_fx15(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        self.delay_timer = self.registers[vx];
        self.debug_print(&format!("LD DT, V{:X}", vx));
    }

    // Fx18 - LD ST, Vx, Set sound timer = Vx.
    fn op_fx18(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        self.sound_timer = self.registers[vx];
        self.debug_print(&format!("LD ST, V{:X}", vx));
    }

    // Fx1E - ADD I, Vx, Set I = I + Vx.
    fn op_fx1e(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        self.index += self.registers[vx] as u16;
        self.debug_print(&format!("ADD I, V{:X}", vx));
    }

    // Fx29 - LD F, Vx, Set I = location of sprite for digit Vx.
    fn op_fx29(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let digit = self.registers[vx] as u16;

        self.index = FONTSET_START_ADDRESS + (5 * digit);
        self.debug_print(&format!("LD F, V{:X} (digit {}, addr 0x{:03X})", vx, digit, self.index));
    }

    // Fx33 - LD B, Vx, Store BCD representation of Vx in memory locations I, I+1, and I+2.
    fn op_fx33(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let mut value = self.registers[vx];

        self.memory[(self.index + 2) as usize] = value % 10;
        value /= 10;

        self.memory[(self.index + 1) as usize] = value % 10;
        value /= 10;

        self.memory[self.index as usize] = value % 10;
        self.debug_print(&format!("LD B, V{:X}", vx));
    }

    // Fx55 - LD [I], Vx: Store registers V0 through Vx in memory starting at location I
    fn op_fx55(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;

        for i in 0..=vx {
            self.memory[(self.index + i as u16) as usize] = self.registers[i];
        }
        self.debug_print(&format!("LD [I], V{:X}", vx));
    }

    // Fx65 - LD Vx, [I]: Read registers V0 through Vx from memory starting at location I
    fn op_fx65(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;

        for i in 0..=vx {
            self.registers[i] = self.memory[(self.index + i as u16) as usize];
        }
        self.debug_print(&format!("LD V{:X}, [I]", vx));
    }

    // Getter methods for testing
    pub fn get_pc(&self) -> u16 {
        self.pc
    }
    pub fn get_register(&self, index: usize) -> u8 {
        self.registers[index]
    }
    pub fn get_index(&self) -> u16 {
        self.index
    }
    pub fn get_sp(&self) -> u8 {
        self.sp
    }
    pub fn get_stack(&self, index: usize) -> u16 {
        self.stack[index]
    }
    pub fn get_delay_timer(&self) -> u8 {
        self.delay_timer
    }
    pub fn get_sound_timer(&self) -> u8 {
        self.sound_timer
    }
    pub fn load_test_program(&mut self, program: &[u8]) {
        let start = START_ADDRESS as usize;
        for (i, &byte) in program.iter().enumerate() {
            if start + i < MEMORY_SIZE {
                self.memory[start + i] = byte;
            }
        }
        self.debug_print(&format!("Loaded test program: {} bytes", program.len()));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_chip8_initialization() {
        let chip8 = Chip8::new();

        assert_eq!(chip8.pc, START_ADDRESS);
        assert_eq!(chip8.sp, 0);
        assert_eq!(chip8.index, 0);

        for &reg in chip8.registers.iter() {
            assert_eq!(reg, 0);
        }

        for &pixel in chip8.video.iter() {
            assert_eq!(pixel, 0);
        }
    }

    #[test]
    fn test_fontset_loaded() {
        let chip8 = Chip8::new();
        let start = FONTSET_START_ADDRESS as usize;

        for (i, &expected) in FONTSET.iter().enumerate() {
            assert_eq!(chip8.memory[start + i], expected)
        }
    }

    #[test]
    fn test_rom_loading() {
        let dummy_rom = vec![0xA2, 0x2A, 0x60, 0x0C, 0x61, 0x08];

        use std::io::Write;
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(&dummy_rom).unwrap();

        let mut chip8 = Chip8::new();
        chip8.load_rom(temp_file.path().to_str().unwrap()).unwrap();

        let start = START_ADDRESS as usize;
        for (i, &expected) in dummy_rom.iter().enumerate() {
            assert_eq!(chip8.memory[start + i], expected);
        }
    }

    #[test]
    fn test_random_byte_generation() {
        let mut chip8 = Chip8::new();

        let mut values = Vec::new();
        for _ in 0..10 {
            values.push(chip8.random_byte());
        }

        let first = values[0];
        let all_same = values.iter().all(|&x| x == first);
        assert!(!all_same, "Random generator produced all identical values");
    }

    // OPCODE TESTS

    #[test]
    fn test_op_00e0_cls() {
        let mut chip8 = Chip8::new();
        chip8.video[0] = 0xFFFFFFFF;
        chip8.video[100] = 0xFFFFFFFF;

        chip8.op_00e0();

        for &pixel in chip8.video.iter() {
            assert_eq!(pixel, 0);
        }
    }

    #[test]
    fn test_op_00ee_ret() {
        let mut chip8 = Chip8::new();
        chip8.stack[0] = 0x300;
        chip8.sp = 1;

        chip8.op_00ee();

        assert_eq!(chip8.pc, 0x300);
        assert_eq!(chip8.sp, 0);
    }

    #[test]
    fn test_op_1nnn_jump() {
        let mut chip8 = Chip8::new();
        chip8.opcode = 0x1234;

        chip8.op_1nnn();

        assert_eq!(chip8.pc, 0x234);
    }

    #[test]
    fn test_op_2nnn_call() {
        let mut chip8 = Chip8::new();
        chip8.pc = 0x300;
        chip8.opcode = 0x2456;

        chip8.op_2nnn();

        assert_eq!(chip8.stack[0], 0x300);
        assert_eq!(chip8.sp, 1);
        assert_eq!(chip8.pc, 0x456);
    }

    #[test]
    fn test_op_3xkk_skip_equal() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0x33;
        chip8.opcode = 0x3533; // SE V5, 0x33
        chip8.pc = 0x200;

        chip8.op_3xkk();

        assert_eq!(chip8.pc, 0x202); // Should skip
    }

    #[test]
    fn test_op_3xkk_no_skip() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0x22;
        chip8.opcode = 0x3533; // SE V5, 0x33
        chip8.pc = 0x200;

        chip8.op_3xkk();

        assert_eq!(chip8.pc, 0x200); // Should not skip
    }

    #[test]
    fn test_op_6xkk_load() {
        let mut chip8 = Chip8::new();
        chip8.opcode = 0x6A55; // LD VA, 0x55

        chip8.op_6xkk();

        assert_eq!(chip8.registers[0xA], 0x55);
    }

    #[test]
    fn test_op_7xkk_add() {
        let mut chip8 = Chip8::new();
        chip8.registers[3] = 0x10;
        chip8.opcode = 0x7315; // ADD V3, 0x15

        chip8.op_7xkk();

        assert_eq!(chip8.registers[3], 0x25);
    }

    #[test]
    fn test_op_7xkk_add_overflow() {
        let mut chip8 = Chip8::new();
        chip8.registers[3] = 0xFF;
        chip8.opcode = 0x7301; // ADD V3, 0x01

        chip8.op_7xkk();

        assert_eq!(chip8.registers[3], 0x00); // Should wrap around
    }

    #[test]
    fn test_op_annn_load_index() {
        let mut chip8 = Chip8::new();
        chip8.opcode = 0xA123; // LD I, 0x123

        chip8.op_annn();

        assert_eq!(chip8.index, 0x123);
    }

    #[test]
    fn test_fetch_decode_execute() {
        let mut chip8 = Chip8::new();

        // Place a simple instruction in memory: 6A55 (LD VA, 0x55)
        chip8.memory[0x200] = 0x6A;
        chip8.memory[0x201] = 0x55;

        chip8.cycle();

        assert_eq!(chip8.registers[0xA], 0x55);
        assert_eq!(chip8.pc, 0x202); // PC should advance
    }

    #[test]
    fn test_op_8xy1_or() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 0b11110000;
        chip8.registers[3] = 0b00001111;
        chip8.opcode = 0x8231; // OR V2, V3

        chip8.op_8xy1();

        assert_eq!(chip8.registers[2], 0b11111111);
    }

    #[test]
    fn test_op_8xy2_and() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 0b11110000;
        chip8.registers[3] = 0b11001100;
        chip8.opcode = 0x8232; // AND V2, V3

        chip8.op_8xy2();

        assert_eq!(chip8.registers[2], 0b11000000);
    }

    #[test]
    fn test_op_8xy3_xor() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 0b11110000;
        chip8.registers[3] = 0b11001100;
        chip8.opcode = 0x8233; // XOR V2, V3

        chip8.op_8xy3();

        assert_eq!(chip8.registers[2], 0b00111100);
    }

    #[test]
    fn test_op_8xy4_add_no_carry() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 100;
        chip8.registers[3] = 50;
        chip8.opcode = 0x8234; // ADD V2, V3

        chip8.op_8xy4();

        assert_eq!(chip8.registers[2], 150);
        assert_eq!(chip8.registers[0xF], 0); // No carry
    }

    #[test]
    fn test_op_8xy4_add_with_carry() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 200;
        chip8.registers[3] = 100;
        chip8.opcode = 0x8234; // ADD V2, V3

        chip8.op_8xy4();

        assert_eq!(chip8.registers[2], 44); // 300 & 0xFF = 44
        assert_eq!(chip8.registers[0xF], 1); // Carry set
    }

    #[test]
    fn test_op_8xy5_sub_no_borrow() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 100;
        chip8.registers[3] = 50;
        chip8.opcode = 0x8235; // SUB V2, V3

        chip8.op_8xy5();

        assert_eq!(chip8.registers[2], 50);
        assert_eq!(chip8.registers[0xF], 1); // No borrow (Vx > Vy)
    }

    #[test]
    fn test_op_8xy5_sub_with_borrow() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 50;
        chip8.registers[3] = 100;
        chip8.opcode = 0x8235; // SUB V2, V3

        chip8.op_8xy5();

        assert_eq!(chip8.registers[2], 206); // 50 - 100 wraps to 206
        assert_eq!(chip8.registers[0xF], 0); // Borrow occurred (Vx < Vy)
    }

    #[test]
    fn test_op_8xy6_shr() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 0b10101011;
        chip8.opcode = 0x8206; // SHR V2

        chip8.op_8xy6();

        assert_eq!(chip8.registers[2], 0b01010101);
        assert_eq!(chip8.registers[0xF], 1); // LSB was 1
    }

    #[test]
    fn test_op_8xy6_shr_lsb_zero() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 0b10101010;
        chip8.opcode = 0x8206; // SHR V2

        chip8.op_8xy6();

        assert_eq!(chip8.registers[2], 0b01010101);
        assert_eq!(chip8.registers[0xF], 0); // LSB was 0
    }

    #[test]
    fn test_op_8xy7_subn_no_borrow() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 50;
        chip8.registers[3] = 100;
        chip8.opcode = 0x8237; // SUBN V2, V3

        chip8.op_8xy7();

        assert_eq!(chip8.registers[2], 50); // 100 - 50
        assert_eq!(chip8.registers[0xF], 1); // No borrow (Vy > Vx)
    }

    #[test]
    fn test_op_8xy7_subn_with_borrow() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 100;
        chip8.registers[3] = 50;
        chip8.opcode = 0x8237; // SUBN V2, V3

        chip8.op_8xy7();

        assert_eq!(chip8.registers[2], 206); // 50 - 100 wraps to 206
        assert_eq!(chip8.registers[0xF], 0); // Borrow occurred (Vy < Vx)
    }

    #[test]
    fn test_op_8xye_shl() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 0b10101011;
        chip8.opcode = 0x820E; // SHL V2

        chip8.op_8xye();

        assert_eq!(chip8.registers[2], 0b01010110);
        assert_eq!(chip8.registers[0xF], 1); // MSB was 1
    }

    #[test]
    fn test_op_8xye_shl_msb_zero() {
        let mut chip8 = Chip8::new();
        chip8.registers[2] = 0b01010101;
        chip8.opcode = 0x820E; // SHL V2

        chip8.op_8xye();

        assert_eq!(chip8.registers[2], 0b10101010);
        assert_eq!(chip8.registers[0xF], 0); // MSB was 0
    }

    // Tests for Exxx opcodes

    #[test]
    fn test_op_ex9e_key_pressed() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0xA;
        chip8.keypad[0xA] = true;
        chip8.opcode = 0xE59E; // SKP V5
        chip8.pc = 0x200;

        chip8.op_ex9e();

        assert_eq!(chip8.pc, 0x202); // Should skip
    }

    #[test]
    fn test_op_ex9e_key_not_pressed() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0xA;
        chip8.keypad[0xA] = false;
        chip8.opcode = 0xE59E; // SKP V5
        chip8.pc = 0x200;

        chip8.op_ex9e();

        assert_eq!(chip8.pc, 0x200); // Should not skip
    }

    #[test]
    fn test_op_exa1_key_not_pressed() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0xA;
        chip8.keypad[0xA] = false;
        chip8.opcode = 0xE5A1; // SKNP V5
        chip8.pc = 0x200;

        chip8.op_exa1();

        assert_eq!(chip8.pc, 0x202); // Should skip
    }

    #[test]
    fn test_op_exa1_key_pressed() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0xA;
        chip8.keypad[0xA] = true;
        chip8.opcode = 0xE5A1; // SKNP V5
        chip8.pc = 0x200;

        chip8.op_exa1();

        assert_eq!(chip8.pc, 0x200); // Should not skip
    }

    // Tests for Fxxx opcodes

    #[test]
    fn test_op_fx07_load_delay_timer() {
        let mut chip8 = Chip8::new();
        chip8.delay_timer = 0x42;
        chip8.opcode = 0xF507; // LD V5, DT

        chip8.op_fx07();

        assert_eq!(chip8.registers[5], 0x42);
    }

    #[test]
    fn test_op_fx0a_key_pressed() {
        let mut chip8 = Chip8::new();
        chip8.keypad[7] = true;
        chip8.opcode = 0xF50A; // LD V5, K
        chip8.pc = 0x200;

        chip8.op_fx0a();

        assert_eq!(chip8.registers[5], 7);
        assert_eq!(chip8.pc, 0x200); // PC should not change when key found
    }

    #[test]
    fn test_op_fx0a_no_key_pressed() {
        let mut chip8 = Chip8::new();
        // All keys are false by default
        chip8.opcode = 0xF50A; // LD V5, K
        chip8.pc = 0x200;

        chip8.op_fx0a();

        assert_eq!(chip8.pc, 0x1FE); // PC should decrement by 2 (repeat instruction)
    }

    #[test]
    fn test_op_fx15_set_delay_timer() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0x42;
        chip8.opcode = 0xF515; // LD DT, V5

        chip8.op_fx15();

        assert_eq!(chip8.delay_timer, 0x42);
    }

    #[test]
    fn test_op_fx18_set_sound_timer() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0x42;
        chip8.opcode = 0xF518; // LD ST, V5

        chip8.op_fx18();

        assert_eq!(chip8.sound_timer, 0x42);
    }

    #[test]
    fn test_op_fx1e_add_to_index() {
        let mut chip8 = Chip8::new();
        chip8.index = 0x200;
        chip8.registers[5] = 0x10;
        chip8.opcode = 0xF51E; // ADD I, V5

        chip8.op_fx1e();

        assert_eq!(chip8.index, 0x210);
    }

    #[test]
    fn test_op_fx29_load_font_address() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 0xA;
        chip8.opcode = 0xF529; // LD F, V5

        chip8.op_fx29();

        // Font for 'A' (0xA) should be at 0x50 + (5 * 0xA) = 0x50 + 50 = 0x82
        assert_eq!(chip8.index, 0x50 + (5 * 0xA));
    }

    #[test]
    fn test_op_fx33_bcd_conversion() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 234;
        chip8.index = 0x300;
        chip8.opcode = 0xF533; // LD B, V5

        chip8.op_fx33();

        assert_eq!(chip8.memory[0x300], 2); // Hundreds
        assert_eq!(chip8.memory[0x301], 3); // Tens
        assert_eq!(chip8.memory[0x302], 4); // Ones
    }

    #[test]
    fn test_op_fx33_bcd_conversion_small() {
        let mut chip8 = Chip8::new();
        chip8.registers[5] = 7;
        chip8.index = 0x300;
        chip8.opcode = 0xF533; // LD B, V5

        chip8.op_fx33();

        assert_eq!(chip8.memory[0x300], 0); // Hundreds
        assert_eq!(chip8.memory[0x301], 0); // Tens
        assert_eq!(chip8.memory[0x302], 7); // Ones
    }

    #[test]
    fn test_op_fx55_store_registers() {
        let mut chip8 = Chip8::new();
        chip8.registers[0] = 0x10;
        chip8.registers[1] = 0x20;
        chip8.registers[2] = 0x30;
        chip8.index = 0x300;
        chip8.opcode = 0xF255; // LD [I], V2 (store V0-V2)

        chip8.op_fx55();

        assert_eq!(chip8.memory[0x300], 0x10);
        assert_eq!(chip8.memory[0x301], 0x20);
        assert_eq!(chip8.memory[0x302], 0x30);
    }

    #[test]
    fn test_op_fx65_load_registers() {
        let mut chip8 = Chip8::new();
        chip8.memory[0x300] = 0x10;
        chip8.memory[0x301] = 0x20;
        chip8.memory[0x302] = 0x30;
        chip8.index = 0x300;
        chip8.opcode = 0xF265; // LD V2, [I] (load V0-V2)

        chip8.op_fx65();

        assert_eq!(chip8.registers[0], 0x10);
        assert_eq!(chip8.registers[1], 0x20);
        assert_eq!(chip8.registers[2], 0x30);
    }

    #[test]
    fn test_op_dxyn_draw() {
        let mut chip8 = Chip8::new();

        // Set up a simple 1x1 sprite (just one byte with all bits set)
        chip8.index = 0x300;
        chip8.memory[0x300] = 0xFF; // 11111111 in binary

        // Draw at position (0, 0)
        chip8.registers[0] = 0; // x position
        chip8.registers[1] = 0; // y position
        chip8.opcode = 0xD011; // DRW V0, V1, 1

        chip8.op_dxyn();

        // Check that the first 8 pixels in the first row are set
        for i in 0..8 {
            assert_eq!(chip8.video[i], 0xFFFFFFFF);
        }

        // Check that collision flag is not set (nothing was there before)
        assert_eq!(chip8.registers[0xF], 0);
    }
}
