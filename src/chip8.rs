use std::{fs, time::{SystemTime, UNIX_EPOCH}};
use anyhow::Result;
use rand::{rngs::StdRng, Rng, SeedableRng};

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
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

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
}

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
                    .as_nanos() as u64
            ),
        };

        chip8.load_fontset();

        chip8
    }

    fn load_fontset(&mut self) {
        let start = FONTSET_START_ADDRESS as usize;
        for (i, &byte) in FONTSET.iter().enumerate() {
            self.memory[start + i] = byte;
        }
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

    pub fn cycle(&mut self) {
        // TODO: IMPLEMENT FETCH-DECODE-EXECUTE CYCLE
        todo!()
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
}
