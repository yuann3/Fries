use anyhow::Result;
use std::time::{Duration, Instant};

mod chip8;
mod platform;

use chip8::Chip8;
use platform::Platform;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        println!("Usage: {} <Scale> <Delay> <ROM>", args[0]);
        println!("  Scale: Window scale factor (e.g., 10)");
        println!("  Delay: Cycle delay in milliseconds (e.g., 1)");
        println!("  ROM: Path to CHIP-8 ROM file (e.g., test_opcode.ch8)");
        println!();
        println!("Examples:");
        println!("  {} 10 1 test_opcode.ch8", args[0]);
        println!("  {} 10 3 Tetris.ch8", args[0]);
        return Ok(());
    }

    // Parse command line arguments exactly like the tutorial
    let video_scale: u32 = args[1].parse()
        .map_err(|_| anyhow::anyhow!("Invalid scale factor: {}", args[1]))?;
    let cycle_delay: u64 = args[2].parse()
        .map_err(|_| anyhow::anyhow!("Invalid delay: {}", args[2]))?;
    let rom_filename = &args[3];

    // Calculate window dimensions
    const VIDEO_WIDTH: u32 = 64;
    const VIDEO_HEIGHT: u32 = 32;
    let window_width = VIDEO_WIDTH * video_scale;
    let window_height = VIDEO_HEIGHT * video_scale;

    println!("CHIP-8 Emulator");
    println!("Scale: {}x, Delay: {}ms, ROM: {}", video_scale, cycle_delay, rom_filename);

    let mut chip8 = Chip8::new();
    chip8.enable_debug(false); // Disable debug for clean output like tutorial

    // Load ROM
    println!("Loading ROM: {}", rom_filename);
    chip8.load_rom(rom_filename)?;
    println!("ROM loaded successfully!");

    // Initialize platform
    let platform = Platform::new("CHIP-8 Emulator", window_width, window_height)?;

    println!("Controls: 1234/QWER/ASDF/ZXCV keys map to CHIP-8 keypad");
    println!("Press ESC or close window to exit");

    // Main emulation loop
    let cycle_duration = Duration::from_millis(cycle_delay);
    let mut last_cycle_time = Instant::now();

    platform.run(move |keys: &mut [bool; 16]| {
        chip8.set_keys(keys);

        let now = Instant::now();
        if now.duration_since(last_cycle_time) >= cycle_duration {
            chip8.cycle();
            last_cycle_time = now;
        }

        let display_buffer = chip8.get_display().to_vec();
        (display_buffer, false)
    })?;

    Ok(())
}
