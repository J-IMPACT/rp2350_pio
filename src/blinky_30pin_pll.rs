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

    let _gpio0: Pin<_, FunctionPio0, _> = pins.gpio0.into_function();
    let _gpio1: Pin<_, FunctionPio0, _> = pins.gpio1.into_function();
    let _gpio2: Pin<_, FunctionPio0, _> = pins.gpio2.into_function();
    let _gpio3: Pin<_, FunctionPio0, _> = pins.gpio3.into_function();
    let _gpio4: Pin<_, FunctionPio0, _> = pins.gpio4.into_function();
    let _gpio5: Pin<_, FunctionPio0, _> = pins.gpio5.into_function();
    let _gpio6: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    let _gpio7: Pin<_, FunctionPio0, _> = pins.gpio7.into_function();
    let _gpio8: Pin<_, FunctionPio0, _> = pins.gpio8.into_function();
    let _gpio9: Pin<_, FunctionPio0, _> = pins.gpio9.into_function();
    let _gpio10: Pin<_, FunctionPio0, _> = pins.gpio10.into_function();
    let _gpio11: Pin<_, FunctionPio0, _> = pins.gpio11.into_function();
    let _gpio12: Pin<_, FunctionPio0, _> = pins.gpio12.into_function();
    let _gpio13: Pin<_, FunctionPio0, _> = pins.gpio13.into_function();
    let _gpio14: Pin<_, FunctionPio0, _> = pins.gpio14.into_function();
    let _gpio15: Pin<_, FunctionPio0, _> = pins.gpio15.into_function();
    let _gpio16: Pin<_, FunctionPio0, _> = pins.gpio16.into_function();
    let _gpio17: Pin<_, FunctionPio0, _> = pins.gpio17.into_function();
    let _gpio18: Pin<_, FunctionPio0, _> = pins.gpio18.into_function();
    let _gpio19: Pin<_, FunctionPio0, _> = pins.gpio19.into_function();
    let _gpio20: Pin<_, FunctionPio0, _> = pins.gpio20.into_function();
    let _gpio21: Pin<_, FunctionPio0, _> = pins.gpio21.into_function();
    let _gpio22: Pin<_, FunctionPio0, _> = pins.gpio22.into_function();
    let _gpio23: Pin<_, FunctionPio0, _> = pins.gpio23.into_function();
    let _gpio24: Pin<_, FunctionPio0, _> = pins.gpio24.into_function();
    let _gpio25: Pin<_, FunctionPio0, _> = pins.gpio25.into_function();
    let _gpio26: Pin<_, FunctionPio0, _> = pins.gpio26.into_function();
    let _gpio27: Pin<_, FunctionPio0, _> = pins.gpio27.into_function();
    let _gpio28: Pin<_, FunctionPio0, _> = pins.gpio28.into_function();
    let _gpio29: Pin<_, FunctionPio0, _> = pins.gpio29.into_function();
    
    let program = pio::pio_asm!(
        "set y, 0b11111",
        ".wrap_target",
        "   mov y, ~y",
        "   mov isr, null",
        "   set x, 5",
        "bit_loop:",
        "   in y, 5",
        "   jmp x--, bit_loop",
        "   mov osr, isr",
        "   out pins, 30",
        "   set x, 27",
        "loop:",
        "   nop [31]",
        "   nop [10]",
        "   jmp x--, loop",
        ".wrap"
    );

    let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio0.install(&program.program).unwrap();
    
    let (int, frac) = (60000, 0); // 2500Hz
    let (mut sm0, _, _) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(0, 30)
        .in_shift_direction(hal::pio::ShiftDirection::Left)
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);
    sm0.set_pindirs((0..30 as u8).map(|pin| (pin, hal::pio::PinDir::Output)));
    sm0.start();

    loop {}
}