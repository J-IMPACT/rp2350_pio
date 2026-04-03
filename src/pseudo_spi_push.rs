#![no_std]
#![no_main]

use embedded_hal::digital::OutputPin;
use panic_halt as _;

use rp235x_hal as hal;

use hal::gpio::{FunctionPio0, FunctionSioOutput, Pin};
use hal::pio::{PIOExt, ShiftDirection};
use hal::Sio;

use embedded_hal::delay::DelayNs;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

/*
Master:
    GPIO0 -> MOSI_OUT
    GPIO1 -> CLK_OUT

Slave:
    GPIO2 <- MOSI_IN (connect to GPIO0)
    GPIO3 <- CLK_IN (connect to GPIO1)
    GPIO6-9 -> LED (4 bit output)
*/

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

    // Master
    let _mosi_out: Pin<_, FunctionPio0, _> = pins.gpio0.into_function();
    let _clk_out: Pin<_, FunctionPio0, _> = pins.gpio1.into_function();

    // Slave
    let _mosi_in: Pin<_, FunctionPio0, _> = pins.gpio2.into_function();
    let _clk_in: Pin<_, FunctionPio0, _> = pins.gpio3.into_function();

    let mut led0: Pin<_, FunctionSioOutput, _> = pins.gpio6.into_push_pull_output();
    let mut led1: Pin<_, FunctionSioOutput, _> = pins.gpio7.into_push_pull_output();
    let mut led2: Pin<_, FunctionSioOutput, _> = pins.gpio8.into_push_pull_output();
    let mut led3: Pin<_, FunctionSioOutput, _> = pins.gpio9.into_push_pull_output();

    let master_prog = pio::pio_asm!(
        ".side_set 1 opt",
        "set pindirs, 0b11",
        ".wrap_target",
        "   pull block",
        "   set x, 3",
        "bitloop:",
        "   out pins, 1 side 0 [7]",
        "   nop side 1 [7]",
        "   jmp x-- bitloop",
        ".wrap"
    );

    let slave_prog = pio::pio_asm!(
        ".wrap_target",
        "   set x, 3",
        "   mov isr, null",
        "bitloop:",
        "   wait 1 pin 1",
        "   in pins, 1",
        "   wait 0 pin 1",
        "   jmp x-- bitloop",
        "   push block",
        ".wrap"
    );

    let (mut pio, sm0, sm1, _, _) = pac.PIO0.split(&mut pac.RESETS);

    let master_inst = pio.install(&master_prog.program).unwrap();
    let slave_inst = pio.install(&slave_prog.program).unwrap();

    let (sm_master, _, mut tx_master) = hal::pio::PIOBuilder::from_installed_program(master_inst)
        .out_pins(0, 1)
        .side_set_pin_base(1)
        .clock_divisor_fixed_point(1, 0)
        .build(sm0);
    
    let (sm_slave, mut rx_slave, _) = hal::pio::PIOBuilder::from_installed_program(slave_inst)
        .in_pin_base(2)
        .in_shift_direction(ShiftDirection::Left) // default
        .clock_divisor_fixed_point(1, 0)
        .build(sm1);

    sm_master.start();
    sm_slave.start();

    let patterns: [u32; 16] = [
        0b0000,
        0b0001,
        0b0010,
        0b0011,
        0b0100,
        0b0101,
        0b0110,
        0b0111,
        0b1000,
        0b1001,
        0b1010,
        0b1011,
        0b1100,
        0b1101,
        0b1110,
        0b1111,
    ];

    loop {
        for &value in patterns.iter() {
            tx_master.write(value);
            timer.delay_ms(500);

            match rx_slave.read() {
                Some(v) => {
                    let data = v & 0xF;

                    if data & 0b0001 != 0 { led0.set_high().unwrap(); } else { led0.set_low().unwrap(); }
                    if data & 0b0010 != 0 { led1.set_high().unwrap(); } else { led1.set_low().unwrap(); }
                    if data & 0b0100 != 0 { led2.set_high().unwrap(); } else { led2.set_low().unwrap(); }
                    if data & 0b1000 != 0 { led3.set_high().unwrap(); } else { led3.set_low().unwrap(); }
                }
                None => {}
            }
            timer.delay_ms(500);
        }
    }
}