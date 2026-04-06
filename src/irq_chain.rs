#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionPio0, FunctionSioOutput, Pin};
use hal::pac::interrupt;
use hal::pio::PIOExt;
use hal::Sio;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

use core::sync::atomic::{AtomicBool, Ordering};
use cortex_m::peripheral::NVIC;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// IRQ Flag
static PIO0_IRQ1_FLAG: AtomicBool = AtomicBool::new(false);

#[interrupt]
fn PIO0_IRQ_0() {
    let pio = unsafe { &*hal::pac::PIO0::ptr() };
    pio.irq().write(|w| unsafe { w.bits(1 << 1) });

    PIO0_IRQ1_FLAG.store(true, Ordering::Release);
}

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ, 
        pac.XOSC, 
        pac.CLOCKS, 
        pac.PLL_SYS, 
        pac.PLL_USB, 
        &mut pac.RESETS, 
        &mut watchdog
    ).unwrap();

    let mut timer = hal::timer::Timer::new_timer0(
        pac.TIMER0, 
        &mut pac.RESETS, 
        &clocks
    );

    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // LED (CPU control)
    let mut led_cpu: Pin<_, FunctionSioOutput, _> = pins.gpio25.into_push_pull_output();

    // LED (SM0 control)
    let _led_sm0: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    // LED (SM1 control)
    let _led_sm1: Pin<_, FunctionPio0, _> = pins.gpio7.into_function();

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
        ".wrap"
    );

    let program_sm1 = pio::pio_asm!(
        ".wrap_target",
        "   wait 1 irq 0",
        "   set pins, 1",
        "   set x, 31",
        "sm1_delay0:",
        "   nop [31]",
        "   nop [31]",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, sm1_delay0",
        "   irq wait 1",
        "   set pins, 0",
        "   set x, 31",
        "sm1_delay1:",
        "   nop [31]",
        "   nop [31]",
        "   nop [31]",
        "   nop [31]",
        "   jmp x--, sm1_delay1",
        ".wrap"
    );

    let (mut pio0, sm0, sm1, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed_sm0 = pio0.install(&program_sm0.program).unwrap();
    let installed_sm1 = pio0.install(&program_sm1.program).unwrap();

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

    pio0.irq0().enable_sm_interrupt(1);

    unsafe { NVIC::unmask(hal::pac::Interrupt::PIO0_IRQ_0); }

    sm0.start();
    sm1.start();

    loop {
        if PIO0_IRQ1_FLAG.swap(false, Ordering::AcqRel) {
            led_cpu.set_high().unwrap();
            timer.delay_ms(500);
            led_cpu.set_low().unwrap();
            timer.delay_ms(500);
        }
    }
}