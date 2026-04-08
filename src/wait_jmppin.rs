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

    let _pin1 = pins.gpio1.into_pull_down_input();
    let _pin2 = pins.gpio2.into_pull_down_input();
    let _pin3 = pins.gpio3.into_pull_down_input();
    let _pin4 = pins.gpio4.into_pull_down_input();
    let _pin5 = pins.gpio5.into_pull_down_input();
    let led: Pin<_, FunctionPio0, _> = pins.gpio25.into_function();
    let led_pin_id = led.id().num;

    let mut a = pio::Assembler::<32>::new();
    let mut wrap_target = a.label();
    let mut wrap_source = a.label();
    let mut label = a.label();
    
    a.set(pio::SetDestination::Y, 0);
    // relative = false
    // a.wait(1, pio::WaitSource::JMPPIN, 0, false); // GPIO1
    // a.wait(1, pio::WaitSource::JMPPIN, 1, false); // GPIO2
    // a.wait(1, pio::WaitSource::JMPPIN, 2, false); // GPIO3
    // a.wait(1, pio::WaitSource::JMPPIN, 3, false); // GPIO4
    a.wait(1, pio::WaitSource::JMPPIN, 4, false); // GPIO1
    a.bind(&mut wrap_target);
    a.mov(pio::MovDestination::PINS, pio::MovOperation::None, pio::MovSource::Y);
    a.mov(pio::MovDestination::Y, pio::MovOperation::Invert, pio::MovSource::Y);
    a.set(pio::SetDestination::X, 28);
    a.bind(&mut label);
    a.nop_with_delay(31);
    a.nop_with_delay(9);
    a.jmp(pio::JmpCondition::XDecNonZero, &mut label);
    a.bind(&mut wrap_source);

    let program = a.assemble_with_wrap(wrap_source, wrap_target);

    let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio0.install(&program).unwrap();
    
    let (int, frac) = (60000, 0); // 2500Hz
    let (mut sm0, _, _) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(led_pin_id, 1) // set -> out
        .jmp_pin(1) // GPIO1
        .clock_divisor_fixed_point(int, frac)
        .build(sm0);
    sm0.set_pindirs([(led_pin_id, hal::pio::PinDir::Output)]);
    sm0.start();

    loop {}
}