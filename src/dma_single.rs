#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::dma::{DMAExt, single_buffer};
use hal::gpio::{FunctionPio0, Pin};
use hal::pio::PIOExt;
use hal::Sio;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

// LED Pattern (4bit -> GPIO6-9)
static BUF: [u32; 16] = [
    0b0000, 0b0001, 0b0010, 0b0011, 
    0b0100, 0b0101, 0b0110, 0b0111, 
    0b1000, 0b1001, 0b1010, 0b1011, 
    0b1100, 0b1101, 0b1110, 0b1110,
];

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
    let ch0 = dma.ch0;

    // single buffer DMA (PATTERN -> PIO TX-FIFO)
    let mut transfer = single_buffer::Config::new(ch0, &BUF, tx0).start();

    loop {
        let (ch0, _buf, tx0) = transfer.wait();
        
        // Restart
        transfer = single_buffer::Config::new(ch0, &BUF, tx0).start();
    }
}