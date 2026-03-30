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

// Use GPIO0-3
const BUS_BASE_PIN_IN: u8 = 0;
// Use GPIO6-9
const BUS_BASE_PIN_OUT: u8 = 6;
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

    let _in0: Pin<_, FunctionPio0, _> = pins.gpio0.into_function();
    let _in1: Pin<_, FunctionPio0, _> = pins.gpio1.into_function();
    let _in2: Pin<_, FunctionPio0, _> = pins.gpio2.into_function();
    let _in3: Pin<_, FunctionPio0, _> = pins.gpio3.into_function();

    let _led0: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    let _led1: Pin<_, FunctionPio0, _> = pins.gpio7.into_function();
    let _led2: Pin<_, FunctionPio0, _> = pins.gpio8.into_function();
    let _led3: Pin<_, FunctionPio0, _> = pins.gpio9.into_function();

    let program = pio::pio_asm!(
        ".wrap_target",
        "   in pins, 4",
        "   mov osr, isr",
        "   out pins, 4",
        "   set x, 31",
        "delay:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, delay",
        ".wrap"
    );

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program.program).unwrap();

    let (mut sm, _, _) = hal::pio::PIOBuilder::from_installed_program(installed)
        .in_pin_base(BUS_BASE_PIN_IN)
        .out_pins(BUS_BASE_PIN_OUT, BUS_WIDTH)
        .in_shift_direction(ShiftDirection::Left) // default
        .out_shift_direction(ShiftDirection::Right) // default
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm0);
    sm.set_pindirs((0..BUS_WIDTH as u8).map(|pin| (pin + BUS_BASE_PIN_IN, hal::pio::PinDir::Input)));
    sm.set_pindirs((0..BUS_WIDTH as u8).map(|pin| (pin + BUS_BASE_PIN_OUT, hal::pio::PinDir::Output)));
    sm.start();

    loop {}
}