#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionPio0, Pin};
use hal::pio::{Buffers, PIOExt, ShiftDirection};
use hal::Sio;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// Use GPIO6-9
const BUS_BASE_PIN: u8 = 6;
const BUS_WIDTH: u8 = 4;

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
        "set x, 0b0010",
        "mov isr, x",
        "mov rxfifo[1], isr",
        "set x, 0b1000",
        "mov isr, x",
        "mov rxfifo[3], isr",
        "set x, 0b0001",
        "mov isr, x",
        "mov rxfifo[0], isr",
        "set x, 0b0100",
        "mov isr, x",
        "mov rxfifo[2], isr",
        "set y, 0b10",
        ".wrap_target",
        "   mov osr, rxfifo[y]",
        "   out pins, 4",
        "   set x, 31",
        "delay:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, delay",
        "   mov y, !y",
        ".wrap"
    );

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program.program).unwrap();

    let (mut sm, _, _) = hal::pio::PIOBuilder::from_installed_program(installed)
        .buffers(Buffers::RxPutGet)
        .out_pins(BUS_BASE_PIN, BUS_WIDTH)
        .out_shift_direction(ShiftDirection::Right) // default
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm0);
    sm.set_pindirs((0..BUS_WIDTH as u8).map(|pin| (pin + BUS_BASE_PIN, hal::pio::PinDir::Output)));
    sm.start();

    loop {}
}