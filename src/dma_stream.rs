#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionPio0, Pin};
use hal::pio::PIOExt;
use hal::Sio;
use hal::dma::{DMAExt, single_buffer};

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let _led0: Pin<_, FunctionPio0, _> = pins.gpio6.into_function();
    let _led1: Pin<_, FunctionPio0, _> = pins.gpio7.into_function();
    let _led2: Pin<_, FunctionPio0, _> = pins.gpio8.into_function();
    let _led3: Pin<_, FunctionPio0, _> = pins.gpio9.into_function();
    
    let program = pio::pio_asm!(
        "set pindirs, 0b1111",

        ".wrap_target",
        "   pull block",
        "   out pins, 4",

        "   set x, 5",

        "delay_loop:",
        "   nop [31]",
        "   jmp x-- delay_loop",

        ".wrap"
    );

    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let installed = pio.install(&program.program).unwrap();
    let (sm, _, tx) = hal::pio::PIOBuilder::from_installed_program(installed)
        .out_pins(6, 4)
        .autopull(true)
        .pull_threshold(32)
        .clock_divisor_fixed_point(0, 0) // as slow as possible
        .build(sm0);
    sm.start();

    let dma = pac.DMA.split(&mut pac.RESETS);
    let ch = dma.ch0;

    static DATA: [u32; 4] = [
        0x11111111,
        0x22222222,
        0x44444444,
        0x88888888,
    ];

    let mut transfer = single_buffer::Config::new(ch, &DATA, tx).start();

    loop {
        let (ch, _buf, tx) = transfer.wait();
        
        // Restart
        transfer = single_buffer::Config::new(ch, &DATA, tx).start();
    }
}