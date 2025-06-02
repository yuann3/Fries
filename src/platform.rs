use anyhow::Result;
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent, ElementState},
    event_loop::EventLoop,
    keyboard::{PhysicalKey, KeyCode},
    window::WindowBuilder,
};

// CHIP-8 display constants
const DISPLAY_WIDTH: u32 = 64;
const DISPLAY_HEIGHT: u32 = 32;

pub struct Platform;

impl Platform {
    pub fn new(_title: &str, _window_width: u32, _window_height: u32) -> Result<Self> {
        Ok(Self)
    }

    pub fn run<F>(self, mut update_fn: F) -> Result<()>
    where
        F: FnMut(&mut [bool; 16]) -> (Vec<u32>, bool) + 'static,
    {
        let event_loop = EventLoop::new()?;

        let window = {
            let size = LogicalSize::new(640.0, 320.0);
            Arc::new(
                WindowBuilder::new()
                    .with_title("FRIES-8")
                    .with_inner_size(size)
                    .with_min_inner_size(size)
                    .build(&event_loop)?
            )
        };

        let mut pixels = {
            let surface_texture = SurfaceTexture::new(
                DISPLAY_WIDTH,
                DISPLAY_HEIGHT,
                window.clone()
            );
            Pixels::new(DISPLAY_WIDTH, DISPLAY_HEIGHT, surface_texture)?
        };

        let mut keys = [false; 16];

        event_loop.run(move |event, control_flow| {
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    control_flow.exit();
                }
                Event::WindowEvent {
                    event: WindowEvent::KeyboardInput {
                        event: key_event,
                        ..
                    },
                    ..
                } => {
                    if let PhysicalKey::Code(key_code) = key_event.physical_key {
                        let pressed = key_event.state == ElementState::Pressed;
                        handle_key_input(&mut keys, key_code, pressed);
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    if let Err(err) = pixels.resize_surface(size.width, size.height) {
                        eprintln!("Failed to resize surface: {}", err);
                        control_flow.exit();
                    }
                }
                Event::WindowEvent {
                    event: WindowEvent::RedrawRequested,
                    ..
                } => {
                    // Get updated display buffer from emulator
                    let (display_buffer, should_quit) = update_fn(&mut keys);

                    if should_quit {
                        control_flow.exit();
                        return;
                    }

                    // Update the pixel buffer
                    update_pixels(&mut pixels, &display_buffer);

                    // Render to screen
                    if let Err(err) = pixels.render() {
                        eprintln!("Failed to render: {}", err);
                        control_flow.exit();
                    }
                }
                Event::AboutToWait => {
                    // Request a redraw
                    window.request_redraw();
                }
                _ => {}
            }
        })?;

        Ok(())
    }
}

fn update_pixels(pixels: &mut Pixels, chip8_display: &[u32]) {
    let frame = pixels.frame_mut();

    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        let chip8_pixel = chip8_display[i];

        // Convert CHIP-8 pixel (0x00000000 or 0xFFFFFFFF) to RGBA
        let rgba = if chip8_pixel == 0xFFFFFFFF {
            [0xFF, 0xFF, 0xFF, 0xFF] // White
        } else {
            [0x00, 0x00, 0x00, 0xFF] // Black
        };

        pixel.copy_from_slice(&rgba);
    }
}

fn handle_key_input(keys: &mut [bool; 16], key_code: KeyCode, pressed: bool) {
    // Map keyboard keys to CHIP-8 keys following the tutorial's layout:
    // Keypad       Keyboard
    // +-+-+-+-+    +-+-+-+-+
    // |1|2|3|C|    |1|2|3|4|
    // +-+-+-+-+    +-+-+-+-+
    // |4|5|6|D| => |Q|W|E|R|
    // +-+-+-+-+    +-+-+-+-+
    // |7|8|9|E|    |A|S|D|F|
    // +-+-+-+-+    +-+-+-+-+
    // |A|0|B|F|    |Z|X|C|V|
    // +-+-+-+-+    +-+-+-+-+

    let chip8_key = match key_code {
        KeyCode::Digit1 => Some(0x1),
        KeyCode::Digit2 => Some(0x2),
        KeyCode::Digit3 => Some(0x3),
        KeyCode::Digit4 => Some(0xC),

        KeyCode::KeyQ => Some(0x4),
        KeyCode::KeyW => Some(0x5),
        KeyCode::KeyE => Some(0x6),
        KeyCode::KeyR => Some(0xD),

        KeyCode::KeyA => Some(0x7),
        KeyCode::KeyS => Some(0x8),
        KeyCode::KeyD => Some(0x9),
        KeyCode::KeyF => Some(0xE),

        KeyCode::KeyZ => Some(0xA),
        KeyCode::KeyX => Some(0x0),
        KeyCode::KeyC => Some(0xB),
        KeyCode::KeyV => Some(0xF),

        _ => None,
    };

    if let Some(key) = chip8_key {
        keys[key] = pressed;
    }
}
