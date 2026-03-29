#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{Pin, FunctionSioOutput};
use hal::Sio;
use hal::timer::Timer;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

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

    let mut timer = Timer::new_timer0(
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

    let mut led: Pin<_, FunctionSioOutput, _> = pins.gpio25.into_push_pull_output();
    
    loop {
        led.set_high().unwrap();
        timer.delay_ms(500);
        led.set_low().unwrap();
        timer.delay_ms(500);
    }
}