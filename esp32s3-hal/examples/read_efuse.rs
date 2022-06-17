#![no_std]
#![no_main]

use core::fmt::Write;

use esp32s3_hal::{efuse::Efuse, pac::Peripherals, prelude::*, RtcCntl, Serial, Timer};
use panic_halt as _;
use xtensa_lx_rt::entry;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take().unwrap();

    let mut timer0 = Timer::new(peripherals.TIMG0);
    let mut serial0 = Serial::new(peripherals.UART0).unwrap();
    let mut rtc_cntl = RtcCntl::new(peripherals.RTC_CNTL);

    // Disable MWDT and RWDT (Watchdog) flash boot protection
    timer0.disable();
    rtc_cntl.set_wdt_global_enable(false);
    writeln!(serial0, "MAC address {:02x?}", Efuse::get_mac_address()).unwrap();
    writeln!(
        serial0,
        "Flash Encryption {:?}",
        Efuse::get_flash_encryption()
    )
    .unwrap();

    loop {}
}
