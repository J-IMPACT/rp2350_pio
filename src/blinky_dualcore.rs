#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{bank0, FunctionSioOutput, Pin, PullDown};
use hal::multicore::{Multicore, Stack};
use hal::Sio;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// Stack for Core1
static CORE1_STACK: Stack<4096> = Stack::new();

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

    let mut timer_core0 = hal::timer::Timer::new_timer0(
        pac.TIMER0, 
        &mut pac.RESETS, 
        &clocks
    );
    let timer_core1 = hal::timer::Timer::new_timer1(
        pac.TIMER1, 
        &mut pac.RESETS, 
        &clocks
    );

    let mut sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_core0: Pin<_, FunctionSioOutput, _> = pins.gpio25.into_function();
    let led_core1: Pin<_, FunctionSioOutput, _> = pins.gpio6.into_function();
    
    let mut multicore = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = multicore.cores();
    let core1 = &mut cores[1];

    core1
        .spawn(CORE1_STACK.take().unwrap(), move || {
            core1_task(led_core1, timer_core1)
        })
        .unwrap();

    loop {
        led_core0.set_high().unwrap();
        timer_core0.delay_ms(500);
        led_core0.set_low().unwrap();
        timer_core0.delay_ms(500);
    }
}

fn core1_task(
    mut led_core1: Pin<bank0::Gpio6, FunctionSioOutput, PullDown>,
    mut timer_core1: hal::timer::Timer<hal::timer::CopyableTimer1>
) {
    loop {
        led_core1.set_high().unwrap();
        timer_core1.delay_ms(250);
        led_core1.set_low().unwrap();
        timer_core1.delay_ms(250);
    }
}