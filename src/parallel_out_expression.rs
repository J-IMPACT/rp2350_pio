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

// Use GPIO6-9
const BUS_BASE_PIN: u8 = 6;

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
        &mut watchdog
    ).unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let _led0: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    let _led1: Pin<_, FunctionPio0, _> = pins.gpio7.into_function();
    let _led2: Pin<_, FunctionPio0, _> = pins.gpio8.into_function();
    let _led3: Pin<_, FunctionPio0, _> = pins.gpio9.into_function();

    let program = pio::pio_asm!(
        ".define public bus_width 4",
        ".define pattern_bit (bus_width * 8)", // 32
        ".define iter (32 - 1)", // 31
        ".define nop_delay 0b11111", // 31
        ".wrap_target",
        "   pull block",
        "   set y, (pattern_bit / bus_width - 1)",
        "out_pins:",
        "   out pins, bus_width",
        "   set x, iter",
        "delay:",
        "   nop [nop_delay]",
        "   nop [0x1f]", // 31
        "   jmp x--, delay",
        "   jmp y--, out_pins",
        ".wrap"
    );

    let bus_width = program.public_defines.bus_width as u8;

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program.program).unwrap();

    let (mut sm, _, mut tx) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(BUS_BASE_PIN, bus_width)
        .out_shift_direction(ShiftDirection::Right) // default
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm0);
    sm.set_pindirs((0..bus_width as u8).map(|pin| (pin + BUS_BASE_PIN, hal::pio::PinDir::Output)));
    sm.start();

    let patterns: [u32; 2] = [
        0b0111_0110_0101_0100_0011_0010_0001_0000,
        0b1111_1110_1101_1100_1011_1010_1001_1000,
    ];

    loop {
        for &value in patterns.iter() {
            while tx.is_full() {}
            tx.write(value);
        }
    }
}