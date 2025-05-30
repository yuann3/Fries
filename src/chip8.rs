use std::{fs, time::{SystemTime, UNIX_EPOCH}};
use anyhow::Result;
use pixels::wgpu::core::registry;
use rand::{rngs::StdRng, Rng, SeedableRng};

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
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
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

    /// ===== INSTRUCTIONS =====

    // 00E0: CLS Clear the display.
    pub fn op_00e0(&mut self) {
        self.video = [0; VIDEO_SIZE]
    }

    // 00EE: RET Return from a subroutine.
    pub fn op_00ee(&mut self) {
        self.sp -= 1;
        self.pc = self.stack[self.sp as usize];
    }

    // 1nnn: JP addr Jump to location nnn.
    pub fn op_1nnn(&mut self) {
        let address = self.opcode & 0x0FFF;
        self.pc = address;
    }

    // 2nnn: JP addr Jump to location nnn.
    pub fn op_2nnn(&mut self) {
        let address = self.opcode & 0x0FFF;
        self.stack[self.sp as usize] = self.pc;
        self.sp += 1;
        self.pc = address;
    }

    // 3xkk - SE Vx, byte Skip next instruction if Vx = kk.
    pub fn op_3xkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        if self.registers[vx] == byte {
            self.pc += 2;
        }
    }

    // 4xkk - SE Vx, byte Skip next instruction if Vx != kk.
    pub fn op_4xkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        if self.registers[vx] != byte {
            self.pc += 2;
        }
    }

    // 5xy0 - SE Vx, Vy Skip next instruction if Vx = Vy.
    pub fn op_5xy0(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x0F00) >> 4) as usize;

        if self.registers[vx] == self.registers[vy] {
            self.pc += 2;
        }
    }

    // 6xkk - LD Vx, byte, Set Vx = kk.
    pub fn op_6xkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        self.registers[vx] = byte;
    }

    // 7xkk - LD Vx, byte, Set Vx + kk.
    pub fn op_7xkk(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let byte = (self.opcode & 0x00FF) as u8;

        self.registers[vx] = self.registers[vx].wrapping_add(byte);
    }

    // 8xy0 - LD Vx, Vy, Set Vx = Vy.
    fn op_8xy0(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[vx] = self.registers[vy]
    }

    // 8xy1 - LD Vx, Vy, Set Vx OR Vy.
    fn op_8xy1(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[vx] |= self.registers[vy]
    }

    // 8xy2 - LD Vx, Vy, Set Vx OR Vy.
    fn op_8xy2(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[vx] &= self.registers[vy]
    }

    // 8xy3 - LD Vx, Vy, Set Vx OR Vy.
    fn op_8xy3(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[vx] ^= self.registers[vy]
    }

    // 8xy4 - ADD Vx, Vy, Set Vx = Vx + Vy, set VF = carry.
    fn op_8xy4(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        let sum = self.registers[vx] as u16 + self.registers[vy] as u16;

        self.registers[0xF] = if sum > 255 { 1 } else { 0 };

        self.registers[vx] = (sum & 0xFF) as u8;

    }

    // 8xy5 - SUB Vx, Vy, Set Vx = Vx - Vy, set VF = NOT borrow.
    fn op_8xy5(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[0xF] = if self.registers[vx] > self.registers[vy] { 1 } else { 0 };

        self.registers[vx] = self.registers[vx].wrapping_sub(self.registers[vy]);
    }

    // 8xy6 - SHR Vx, Set Vx = Vx SHR 1.
    fn op_8xy6(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;

        self.registers[0xF] = self.registers[vx] & 0x1;

        self.registers[vx] >>= 1;
    }

    // 8xy7 - SUBN Vx, Vy, Set Vx = Vy - Vx, set VF = NOT borrow.
    fn op_8xy7(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;
        let vy = ((self.opcode & 0x00F0) >> 4) as usize;

        self.registers[0xF] = if self.registers[vy] > self.registers[vx] { 1 } else { 0 };

        self.registers[vx] = self.registers[vy].wrapping_sub(self.registers[vx]);
    }

    // 8xyE - SHL Vx {, Vy}, Set Vx = Vx SHL 1.
    fn op_8xye(&mut self) {
        let vx = ((self.opcode & 0x0F00) >> 8) as usize;

        self.registers[0xF] = ((self.registers[vx] & 0x80) >> 7) as u8;

        self.registers[vx] <<= 1;
    }

    // Getter methods for testing
    pub fn get_pc(&self) -> u16 { self.pc }
    pub fn get_register(&self, index: usize) -> u8 { self.registers[index] }
    pub fn get_index(&self) -> u16 { self.index }
    pub fn get_sp(&self) -> u8 { self.sp }
    pub fn get_stack(&self, index: usize) -> u16 { self.stack[index] }
    pub fn get_delay_timer(&self) -> u8 { self.delay_timer }
    pub fn get_sound_timer(&self) -> u8 { self.sound_timer }
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

    // #[test]
    // fn test_op_annn_load_index() {
    //     let mut chip8 = Chip8::new();
    //     chip8.opcode = 0xA123; // LD I, 0x123

    //     chip8.op_annn();

    //     assert_eq!(chip8.index, 0x123);
    // }

    // #[test]
    // fn test_fetch_decode_execute() {
    //     let mut chip8 = Chip8::new();

    //     // Place a simple instruction in memory: 6A55 (LD VA, 0x55)
    //     chip8.memory[0x200] = 0x6A;
    //     chip8.memory[0x201] = 0x55;

    //     chip8.cycle();

    //     assert_eq!(chip8.registers[0xA], 0x55);
    //     assert_eq!(chip8.pc, 0x202); // PC should advance
    // }
}
