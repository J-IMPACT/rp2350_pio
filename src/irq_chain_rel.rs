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
        &mut watchdog
    ).unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // LED (SM0 control)
    let _led_sm0: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    // LED (SM1 control)
    let _led_sm1: Pin<_, FunctionPio0, _> = pins.gpio7.into_function();
    // LED (SM0 control)
    let _led_sm2: Pin<_, FunctionPio0, _> = pins.gpio8.into_function();
    // LED (SM1 control)
    let _led_sm3: Pin<_, FunctionPio0, _> = pins.gpio9.into_function();

    let program_sm0 = pio::pio_asm!(
        ".wrap_target",
        "   set pins, 1",
        "   set x, 31",
        "sm0_delay0:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, sm0_delay0",
        "   irq wait 0",
        "   set pins, 0",
        "   set x, 31",
        "sm0_delay1:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, sm0_delay1",
        "   wait 1 irq 3",
        ".wrap"
    );

    let program_sm123 = pio::pio_asm!(
        ".wrap_target",
        "   wait 1 irq 3 rel",
        "   set pins, 1",
        "   set x, 31",
        "sm123_delay0:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, sm123_delay0",
        "   irq wait 0 rel",
        "   set pins, 0",
        "   set x, 31",
        "sm123_delay1:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, sm123_delay1",
        ".wrap"
    );
    let (mut pio, sm0, sm1, sm2, sm3) = pac.PIO0.split(&mut pac.RESETS);
    let installed_sm0 = pio.install(&program_sm0.program).unwrap();
    let installed_sm1 = pio.install(&program_sm123.program).unwrap();
    let installed_sm2 = unsafe { installed_sm1.share() };
    let installed_sm3 = unsafe { installed_sm1.share() };

    let (mut sm0, _, _) = hal::pio::PIOBuilder::from_installed_program(installed_sm0)
        .set_pins(6, 1)
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm0);
    sm0.set_pindirs([(6, hal::pio::PinDir::Output)]);

    let (mut sm1, _, _) = hal::pio::PIOBuilder::from_installed_program(installed_sm1)
        .set_pins(7, 1)
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm1);
    sm1.set_pindirs([(7, hal::pio::PinDir::Output)]);

    let (mut sm2, _, _) = hal::pio::PIOBuilder::from_installed_program(installed_sm2)
        .set_pins(8, 1)
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm2);
    sm2.set_pindirs([(8, hal::pio::PinDir::Output)]);

    let (mut sm3, _, _) = hal::pio::PIOBuilder::from_installed_program(installed_sm3)
        .set_pins(9, 1)
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm3);
    sm3.set_pindirs([(9, hal::pio::PinDir::Output)]);

    // let pio_reg = unsafe { &*hal::pac::PIO0::ptr() };
    // pio_reg.irq().write(|w| unsafe { w.bits(1 << 3 | 1 << 2 | 1 << 1 | 1 << 0) });

    sm0.start();
    sm1.start();
    sm2.start();
    sm3.start();

    loop {
        
    }
}