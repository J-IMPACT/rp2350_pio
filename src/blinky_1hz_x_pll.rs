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

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let _clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ, 
        pac.XOSC, 
        pac.CLOCKS,
        pac.PLL_SYS, 
        pac.PLL_USB, 
        &mut pac.RESETS, 
        &mut watchdog,
    ).unwrap();

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
        "   set x, 23",
        "low:",
        "   nop [31]",
        "   nop [18]",
        "   jmp x--, low",
        "   set pins, 1",
        "   set x, 23",
        "high:",
        "   nop [31]",
        "   nop [18]",
        "   jmp x--, high",
        ".wrap"
    );

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program.program).unwrap();
    let (int, frac) = (60000, 0); // 2500Hz
    let (mut sm, _, _) = hal::pio::PIOBuilder::from_installed_program(installed)
        .set_pins(led_pin_id, 1)
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);
    sm.set_pindirs([(led_pin_id, hal::pio::PinDir::Output)]);
    sm.start();

    loop {}
}