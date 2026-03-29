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

    const MAX_DELAY: u8 = 31;
    let mut a = pio::Assembler::<32>::new();
    let mut wrap_target = a.label();
    let mut wrap_source = a.label();
    
    a.set(pio::SetDestination::PINDIRS, 1);
    a.bind(&mut wrap_target);
    a.set_with_delay(pio::SetDestination::PINS, 0, MAX_DELAY);
    a.set_with_delay(pio::SetDestination::PINS, 1, MAX_DELAY);
    a.bind(&mut wrap_source);

    let program = a.assemble_with_wrap(wrap_source, wrap_target);

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program).unwrap();
    let (int, frac) = (0, 0); // as slow as possible (0 is interpreted as 65536)
    let (sm, _, _) = hal::pio::PIOBuilder::from_installed_program(installed)
        .set_pins(led_pin_id, 1)
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);
    sm.start();

    loop {}
}