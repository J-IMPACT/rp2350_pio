#![no_std]
#![no_main]

use panic_halt as _;

use rp235x_hal as hal;

use hal::dma::{DMAExt, double_buffer, SingleChannel};
use hal::gpio::{FunctionPio0, Pin};
use hal::pio::PIOExt;
use hal::Sio;
use hal::pac::interrupt;

use core::cell::{RefCell, UnsafeCell};
use core::sync::atomic::{AtomicBool, Ordering};
use cortex_m::peripheral::NVIC;
use critical_section::Mutex;
use heapless::spsc::Queue;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

const N_BUFS: usize = 4;

// ===== DMA complete flag =====
static DMA_DONE: AtomicBool = AtomicBool::new(false);

// ===== Buffer pool =====
struct Pool(UnsafeCell<[[u32; 16]; N_BUFS]>);
unsafe impl Sync for Pool {}

static POOL: Pool = Pool(UnsafeCell::new([[0; 16]; N_BUFS]));

// ===== Queue =====
static FREE_Q: Mutex<RefCell<Queue<usize, N_BUFS>>> = Mutex::new(RefCell::new(Queue::new()));
static READY_Q: Mutex<RefCell<Queue<usize, N_BUFS>>> = Mutex::new(RefCell::new(Queue::new()));

fn buf_mut(idx: usize) -> &'static mut [u32; 16] {
    unsafe { &mut (*POOL.0.get())[idx] }
}

fn set_data(idx: usize, count: u32) {
    for i in 0..8 {
        buf_mut(idx)[2*i] = 0;
        buf_mut(idx)[2*i+1] = count;
    }
}

fn set_error(idx: usize) {
    for i in 0..8 {
        buf_mut(idx)[2*i] = 0b1010;
        buf_mut(idx)[2*i+1] = 0b0101;
    }
}

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

    // ===== Initialize =====
    for i in 0..N_BUFS { set_data(i, i as u32); }
    let mut count = N_BUFS as u32;

    critical_section::with(|cs| {
        let mut rq = READY_Q.borrow_ref_mut(cs);
        for i in 0..N_BUFS {
            let _ = rq.enqueue(i);
        }
    });

    let (idx0, idx1) = critical_section::with(|cs| {
        let mut rq = READY_Q.borrow_ref_mut(cs);
        (rq.dequeue().unwrap(), rq.dequeue().unwrap())
    });

    let mut dma_cur_idx = idx0;
    let mut dma_next_idx = idx1;

    let mut transfer = double_buffer::Config::new(
        (ch0, ch1), 
        buf_mut(idx0), 
        tx0
    )
    .start()
    .read_next(buf_mut(idx1));

    loop {
        if let Some(free_idx) = critical_section::with(|cs| {
            let mut fq = FREE_Q.borrow_ref_mut(cs);
            fq.dequeue()
        }) {
            set_data(free_idx, count);
            count = count.wrapping_add(1) & 0x0F;
            critical_section::with(|cs| {
                let mut rq = READY_Q.borrow_ref_mut(cs);
                let _ = rq.enqueue(free_idx);
            });
        }

        if DMA_DONE.swap(false, Ordering::AcqRel) {
            let (_finished, next) = transfer.wait();

            let finished_idx = dma_cur_idx;
            dma_cur_idx = dma_next_idx;

            if let Some(next_idx) = critical_section::with(|cs| {
                let mut rq = READY_Q.borrow_ref_mut(cs);
                rq.dequeue()
            }) {
                dma_next_idx = next_idx;

                // To FREE
                critical_section::with(|cs| {
                    let mut fq = FREE_Q.borrow_ref_mut(cs);
                    let _ = fq.enqueue(finished_idx);
                });
            } else {
                set_error(finished_idx);
                dma_next_idx = finished_idx;
            }

            transfer = next.read_next(buf_mut(dma_next_idx));
        }
    }
}