#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::dma::{DMAExt, double_buffer, SingleChannel};
use hal::gpio::{FunctionPio0, Pin};
use hal::pio::PIOExt;
use hal::Sio;
use hal::pac::interrupt;

use core::sync::atomic::{AtomicBool, Ordering};
use cortex_m::peripheral::NVIC;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// ===== DMA complete flag =====
static DMA_DONE: AtomicBool = AtomicBool::new(false);

// ===== Doubl buffer =====
static BUF0: [u32; 16] = [
    0b0000, 0b0001, 0b0010, 0b0011, 
    0b0100, 0b0101, 0b0110, 0b0111, 
    0b1000, 0b1001, 0b1010, 0b1011, 
    0b1100, 0b1101, 0b1110, 0b1110,
];
static BUF1: [u32; 16] = [
    0b1010, 0b0101, 0b1010, 0b0101, 
    0b1010, 0b0101, 0b1010, 0b0101, 
    0b1010, 0b0101, 0b1010, 0b0101, 
    0b1010, 0b0101, 0b1010, 0b0101, 
];

#[interrupt]
fn DMA_IRQ_0() {
    let dma = unsafe { &*hal::pac::DMA::ptr() };
    dma.ints0().write(|w| unsafe { w.bits((1 << 0) | (1 << 1)) });

    DMA_DONE.store(true, Ordering::Release);
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

    let _led6: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    let _led7: Pin<_, FunctionPio0, _> = pins.gpio7.into_function();
    let _led8: Pin<_, FunctionPio0, _> = pins.gpio8.into_function();
    let _led9: Pin<_, FunctionPio0, _> = pins.gpio9.into_function();
    
    let program = pio::pio_asm!(
        ".wrap_target",
        "   pull block",
        "   out pins, 4",
        "   set x, 31",
        "delay:",
        "   nop [31]",
        "   jmp x--, delay",
        ".wrap"
    );

    let (mut pio0, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio0.install(&program.program).unwrap();

    let (mut sm0, _, tx0) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(6, 4)
        .clock_divisor_fixed_point(60000, 0) // 2500Hz
        .build(sm0);
    sm0.set_pindirs((6..10).map(|pin| (pin, hal::pio::PinDir::Output)));
    sm0.start();

    // DMA
    let dma = pac.DMA.split(&mut pac.RESETS);
    let mut ch0 = dma.ch0;
    let mut ch1 = dma.ch1;

    ch0.enable_irq0();
    ch1.enable_irq0();

    unsafe { NVIC::unmask(hal::pac::Interrupt::DMA_IRQ_0) };

    let mut use_buf0 = true;

    let mut transfer = double_buffer::Config::new(
        (ch0, ch1), 
        &BUF0, 
        tx0
    )
    .start()
    .read_next(&BUF1);

    loop {
        if DMA_DONE.swap(false, Ordering::AcqRel) {
            let (_finished, next) = transfer.wait();

            transfer = if use_buf0 {
                use_buf0 = false;
                next.read_next(&BUF0)
            } else {
                use_buf0 = true;
                next.read_next(&BUF1)
            }
        }
    }
}