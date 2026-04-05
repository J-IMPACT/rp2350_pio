#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionPio0, Pin};
use hal::pio::PIOExt;
use hal::Sio;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let led: Pin<_, FunctionPio0, _> = pins.gpio25.into_function();
    let led_pin_id = led.id().num;
    
    let program = pio::pio_asm!(
        ".wrap_target",
        "   set pins, 0",
        "   set x, 6",
        "low:"
        "   nop [12]",
        "   jmp x--, low",
        "   set pins, 1",
        "   set x, 6",
        "high:",
        "   nop [12]",
        "   jmp x--, high",
        ".wrap"
    );

    let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio0.install(&program.program).unwrap();
    
    let (int, frac) = (60000, 0); // 200Hz
    let (mut sm0, _, _) = hal::pio::PIOBuilder::from_installed_program(installed)
        .set_pins(led_pin_id, 1)
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);
    sm0.set_pindirs([(led_pin_id, hal::pio::PinDir::Output)]);
    sm0.start();

    loop {}
}