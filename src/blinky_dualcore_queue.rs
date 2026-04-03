#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{bank0, FunctionSioOutput, Pin, PullDown};
use hal::multicore::{Multicore, Stack};
use hal::Sio;

use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

use core::cell::RefCell;
use critical_section::Mutex;
use heapless::spsc::Queue;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// Stack for Core1
static CORE1_STACK: Stack<4096> = Stack::new();

// Queue (Inter-core communication)
static QUEUE: Mutex<RefCell<Queue<u8, 16>>> = Mutex::new(RefCell::new(Queue::new()));

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

    let led6: Pin<_, FunctionSioOutput, _> = pins.gpio6.into_function();
    let led7: Pin<_, FunctionSioOutput, _> = pins.gpio7.into_function();
    let led8: Pin<_, FunctionSioOutput, _> = pins.gpio8.into_function();
    let led9: Pin<_, FunctionSioOutput, _> = pins.gpio9.into_function();
    
    let mut multicore = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = multicore.cores();
    let core1 = &mut cores[1];

    core1
        .spawn(CORE1_STACK.take().unwrap(), move || {
            core1_task(led6, led7, led8, led9, timer_core1)
        })
        .unwrap();

    // Producer (Core 0)
    let mut counter: u8 = 0;

    loop {
        // enqueue (discard when it fails)
        critical_section::with(|cs| {
            let mut q = QUEUE.borrow_ref_mut(cs);
            let _ = q.enqueue(counter);
        });

        counter = counter.wrapping_add(1) & 0x0F;

        led_core0.set_high().unwrap();
        timer_core0.delay_ms(100);
        led_core0.set_low().unwrap();
        timer_core0.delay_ms(100);
    }
}

fn core1_task(
    mut led6: Pin<bank0::Gpio6, FunctionSioOutput, PullDown>,
    mut led7: Pin<bank0::Gpio7, FunctionSioOutput, PullDown>,
    mut led8: Pin<bank0::Gpio8, FunctionSioOutput, PullDown>,
    mut led9: Pin<bank0::Gpio9, FunctionSioOutput, PullDown>,
    mut timer_core1: hal::timer::Timer<hal::timer::CopyableTimer1>
) {
    loop {
        let mut data: Option<u8> = None;

        // dequeue
        critical_section::with(|cs| {
            let mut q = QUEUE.borrow_ref_mut(cs);
            data = q.dequeue();
        });

        if let Some(value) = data {
            if (value & 0b0001) != 0 { led6.set_high().unwrap(); } else { led6.set_low().unwrap(); }
            if (value & 0b0010) != 0 { led7.set_high().unwrap(); } else { led7.set_low().unwrap(); }
            if (value & 0b0100) != 0 { led8.set_high().unwrap(); } else { led8.set_low().unwrap(); }
            if (value & 0b1000) != 0 { led9.set_high().unwrap(); } else { led9.set_low().unwrap(); }
        }

        timer_core1.delay_ms(50);
    }
}