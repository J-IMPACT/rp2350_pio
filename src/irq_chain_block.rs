#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionPio0, FunctionPio1, Pin};
use hal::pio::{MovStatusConfig, PIOExt};
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

    // LED (PIO0 control)
    let _led_pio0: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    // LED (PIO1 control)
    let _led_pio1: Pin<_, FunctionPio1, _> = pins.gpio7.into_function();

    // "wait 1 irq 0 next" is not implemented yet.
    let program_pio0 = pio::pio_asm!(
        ".wrap_target",
        "   set pins, 1",
        "   set x, 31",
        "delay0:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, delay0",
        "   irq wait 0",
        "   set pins, 0",
        "   set x, 31",
        "delay1:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, delay1",
        "check_status:",
        "   mov x, status",
        "   jmp !x, check_status",
        "   irq clear 0 next",
        ".wrap"
    );

    let program_pio1 = pio::pio_asm!(
        ".wrap_target",
        "check_status:",
        "   mov x, status",
        "   jmp !x, check_status",
        "   irq clear 0 prev",
        "   set pins, 1",
        "   set x, 31",
        "delay0:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, delay0",
        "   irq wait 0",
        "   set pins, 0",
        "   set x, 31",
        "delay1:",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, delay1",
        ".wrap"
    );

    let (mut pio0, pio0_sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed_pio0 = pio0.install(&program_pio0.program).unwrap();

    let (mut pio0_sm0, _, _) = hal::pio::PIOBuilder::from_installed_program(installed_pio0)
        .set_pins(6, 1)
        .set_mov_status_config(MovStatusConfig::Irq(0x10)) // PIO1_IRQ0
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(pio0_sm0);
    pio0_sm0.set_pindirs([(6, hal::pio::PinDir::Output)]);

    let (mut pio1, pio1_sm0, _, _, _) = pac.PIO1.split(&mut pac.RESETS);
    let installed_pio1 = pio1.install(&program_pio1.program).unwrap();

    let (mut pio1_sm0, _, _) = hal::pio::PIOBuilder::from_installed_program(installed_pio1)
        .set_pins(7, 1)
        .set_mov_status_config(MovStatusConfig::Irq(0x08)) // PIO0_IRQ0
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(pio1_sm0);
    pio1_sm0.set_pindirs([(7, hal::pio::PinDir::Output)]);

    pio0_sm0.start();
    pio1_sm0.start();

    loop {
        
    }
}