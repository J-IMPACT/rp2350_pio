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
        ".define public iter 24",
        ".wrap_target",
        "   set pins, 1",       //  0
        "   set x, (iter - 1)", //  1
        "low:",
        "   pull block",        //  2
        "   out y, 5",          //  3
        "   out pc, 5",         //  4
        "   jmp x--, low",      //  5
        "   set pins, 0",       //  6
        "   set x, (iter - 1)", //  7
        "high:",
        "   pull block",        //  8
        "   out y, 5",          //  9
        "   out pc, 5",         // 10
        "   jmp x--, high",     // 11
        ".wrap",
        "   nop [31]",          // 12
        // 18 - 4 (pull, out, out, mov)
        "   nop [14]",          // 13
        "   mov pc, y",         // 14
    );

    let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio0.install(&program.program).unwrap();
    let offset = installed.offset() as u32;
    let (int, frac) = (60000, 0); // 2500Hz
    let (mut sm0, _, mut tx0) = hal::pio::PIOBuilder::from_installed_program(installed)
        .set_pins(led_pin_id, 1)
        .out_shift_direction(ShiftDirection::Right) // default
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);
    sm0.set_pindirs([(led_pin_id, hal::pio::PinDir::Output)]);
    sm0.start();

    let iter = program.public_defines.iter;
    let pattern_low = ((12 + offset) << 5) + 5 + offset;
    let pattern_high = ((12 + offset) << 5) + 11 + offset;

    loop {
        for _ in 0..iter {
            while tx0.is_full() {}
            tx0.write(pattern_low);
        }
        for _ in 0..iter {
            while tx0.is_full() {}
            tx0.write(pattern_high);
        }
    }
}