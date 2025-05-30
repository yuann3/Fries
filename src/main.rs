use anyhow::Result;

mod chip8;
mod platform;

use chip8::Chip8;
use platform::Platform;

const SCALE: u32 = 10;
const WINDOW_WIDTH: u32 = 64 * SCALE;
const WINDOW_HEIGHT: u32 = 32 * SCALE;

fn main() -> Result<()>{
    println!("starting emulator...");

    let mut chip8 = Chip8::new();

    println!("yayyy, emulator initialized successfully!");

    Ok(())
}
