#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionSioOutput, Pin};
use hal::pac::interrupt;
use hal::pio::PIOExt;
use hal::Sio;

use embedded_hal::digital::StatefulOutputPin;

use core::sync::atomic::{AtomicBool, Ordering};
use cortex_m::peripheral::NVIC;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// IRQ Flag
static PIO0_IRQ0_FLAG: AtomicBool = AtomicBool::new(false);

#[interrupt]
fn PIO0_IRQ_0() {
    let pio0 = unsafe { &*hal::pac::PIO0::ptr() };
    pio0.irq().write(|w| unsafe { w.bits(1 << 0) });

    PIO0_IRQ0_FLAG.store(true, Ordering::Release);
}

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

    // LED (CPU control)
    let mut led: Pin<_, FunctionSioOutput, _> = pins.gpio25.into_push_pull_output();

    let program = pio::pio_asm!(
        ".wrap_target",
        "   irq 0",
        "   set x, 23",
        "loop:",
        "   nop [31]",
        "   nop [18]",
        "   jmp x--, loop",
        ".wrap"
    );

    let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio0.install(&program.program).unwrap();

    let (sm0, _, _) = hal::pio::PIOBuilder::from_installed_program(installed)
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm0);

    pio0.irq0().enable_sm_interrupt(0);

    unsafe { NVIC::unmask(hal::pac::Interrupt::PIO0_IRQ_0); }

    sm0.start();

    loop {
        if PIO0_IRQ0_FLAG.swap(false, Ordering::AcqRel) {
            led.toggle().unwrap();
        }
    }
}