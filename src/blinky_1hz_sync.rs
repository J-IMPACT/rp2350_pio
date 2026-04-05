#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionPio0, FunctionPio1, FunctionPio2, FunctionSioInput, Pin};
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

    let _pin0: Pin<_, FunctionSioInput, _> = pins.gpio0.into_pull_down_input();
    let led0: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    let led1: Pin<_, FunctionPio1, _> = pins.gpio7.into_function();
    let led2: Pin<_, FunctionPio2, _> = pins.gpio8.into_function();

    let program0 = pio::pio_asm!(
        "wait 1 gpio 0",
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
    
    let program1 = pio::pio_asm!(
        "wait 1 gpio 0",
        ".wrap_target",
        "   set pins, 0",
        "   set y, 25",
        "low_outer:",
        "   set x, 22",
        "low_inner:"
        "   nop",
        "   jmp x--, low_inner",
        "   jmp y--, low_outer",
        "   set pins, 1",
        "   set y, 25",
        "high_outer:",
        "   set x, 22",
        "high_inner:"
        "   nop",
        "   jmp x--, high_inner",
        "   jmp y--, high_outer",
        ".wrap"
    );
    
    let program2 = pio::pio_asm!(
        "set y, 0",
        "wait 1 gpio 0",
        ".wrap_target",
        "   mov pins, y",
        "   mov y, ~y",
        "   set x, 28",
        "loop:",
        "   nop [31]",
        "   nop [9]",
        "   jmp x--, loop",
        ".wrap"
    );

    let (int, frac) = (60000, 0); // 2500Hz

    let (mut pio0, sm0_pio0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed0 = pio0.install(&program0.program).unwrap();
    let (mut sm0_pio0, _, _) = hal::pio::PIOBuilder::from_installed_program(installed0)
        .set_pins(led0.id().num, 1)
        .clock_divisor_fixed_point(int, frac)
        .build(sm0_pio0);
    sm0_pio0.set_pindirs([(led0.id().num, hal::pio::PinDir::Output)]);
    sm0_pio0.start();

    let (mut pio1, sm0_pio1, _, _, _) = pac.PIO1.split(&mut pac.RESETS);
    let installed1 = pio1.install(&program1.program).unwrap();
    let (mut sm0_pio1, _, _) = hal::pio::PIOBuilder::from_installed_program(installed1)
        .set_pins(led1.id().num, 1)
        .clock_divisor_fixed_point(int, frac)
        .build(sm0_pio1);
    sm0_pio1.set_pindirs([(led1.id().num, hal::pio::PinDir::Output)]);
    sm0_pio1.start();

    let (mut pio2, sm0_pio2, _, _, _) = pac.PIO2.split(&mut pac.RESETS);
    let installed2 = pio2.install(&program2.program).unwrap();
    let (mut sm0_pio2, _, _) = hal::pio::PIOBuilder::from_installed_program(installed2)
        .out_pins(led2.id().num, 1) // set -> out
        .clock_divisor_fixed_point(int, frac)
        .build(sm0_pio2);
    sm0_pio2.set_pindirs([(led2.id().num, hal::pio::PinDir::Output)]);
    sm0_pio2.start();

    loop {}
}