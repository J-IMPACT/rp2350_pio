#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionPio0, Pin};
use hal::pio::{PIOExt, ShiftDirection};
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
        "   out exec, 16", // nop [30] + 1
        "   mov exec, osr", // nop [17] + 1
        "   jmp x--, low",
        "   set pins, 1",
        "   set x, 23",
        "high:",
        "   out exec, 16", // nop [30] + 1
        "   mov exec, osr", // nop [17] + 1
        "   jmp x--, high",
        ".wrap"
    );

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program.program).unwrap();
    let (int, frac) = (60000, 0); // 2500Hz
    let (mut sm, _, mut tx) = hal::pio::PIOBuilder::from_installed_program(installed)
        .set_pins(led_pin_id, 1)
        .out_shift_direction(ShiftDirection::Right) // default
        .autopull(true)
        .pull_threshold(16)
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);
    sm.set_pindirs([(led_pin_id, hal::pio::PinDir::Output)]);
    sm.start();

    let patterns: [u32; 2] = [
        0b101_11110_010_00_010, // mov y, y [30]
        0b101_10000_010_00_010, // mov y, y [17]
    ];

    loop {
        for &value in patterns.iter() {
            while tx.is_full() {}
            tx.write(value);
        }
    }
}