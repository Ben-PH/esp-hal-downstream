//! `SysUptime` implements proposed embedded-hal `TimeCount` trait to get system uptime
//!
//! ### Example
//! ```no_run
//! let time = time::current_time();
//! ```
#![warn(missing_docs)]
use counters::TimeCount;

/// Provides an interface by which to access the platforms clock. Can be used where
/// (proposed) embedded-hal `TimeCount` implementor is expected.
pub struct SysUptime;

/// Provides time since system start
///
/// The counter won’t measure time in sleep-mode.
///
/// The timer will wrap after
#[cfg_attr(esp32, doc = "36_558 years")]
#[cfg_attr(esp32s2, doc = "7_311 years")]
#[cfg_attr(not(any(esp32, esp32s2)), doc = "more than 7 years")]
impl counters::TimeCount for SysUptime {
    type RawData = u64;



    /// Defines granularity of the timer. Combines with the raw tick count to derive an instant
    #[cfg(not(esp32))]
    type TickMeasure = fugit::Instant<Self::RawData, 1, {crate::timer::systimer::TICKS_PER_SECOND}>;
    #[cfg(esp32)]
    type TickMeasure = fugit::Duration<Self::RawData, 1, 16_000_000>;

    type Error = ();

    fn try_now_raw(&self) -> Result<Self::RawData, Self::Error> {
        #[cfg(esp32)]
        let ticks = {
            // on ESP32 use LACT
            let tg0 = unsafe { crate::peripherals::TIMG0::steal() };
            tg0.lactupdate().write(|w| unsafe { w.update().bits(1) });

            // The peripheral doesn't have a bit to indicate that the update is done, so we
            // poll the lower 32 bit part of the counter until it changes, or a timeout
            // expires.
            let lo_initial = tg0.lactlo().read().bits();
            let mut div = tg0.lactconfig().read().divider().bits();
            let lo = loop {
                let lo = tg0.lactlo().read().bits();
                if lo != lo_initial || div == 0 {
                    break lo;
                }
                div -= 1;
            };
            let hi = tg0.lacthi().read().bits();

            let ticks = (hi as u64) << 32u64 | lo as u64;
            ticks
        };

        #[cfg(not(esp32))]
        let ticks = {
            // otherwise use SYSTIMER
            crate::timer::systimer::SystemTimer::now()
        };

        Ok(ticks)
    }

    fn try_now(&self) -> Result<Self::TickMeasure, Self::Error> {
        let ticks = self.try_now_raw()?;
        Ok(Self::TickMeasure::from_ticks(ticks))

    }
}

impl SysUptime {
    #[cfg(esp32)]
    pub(crate) fn time_init() -> Self {
        // we assume 80MHz APB clock source - there is no way to configure it in a
        // different way currently
        const APB_FREQUENCY: u32 = 80_000_000u32;

        let tg0 = unsafe { crate::peripherals::TIMG0::steal() };

        tg0.lactconfig().write(|w| unsafe { w.bits(0) });
        tg0.lactalarmhi().write(|w| unsafe { w.bits(u32::MAX) });
        tg0.lactalarmlo().write(|w| unsafe { w.bits(u32::MAX) });
        tg0.lactload().write(|w| unsafe { w.load().bits(1) });

        // 16 MHz counter
        tg0.lactconfig()
            .modify(|_, w| unsafe { w.divider().bits((APB_FREQUENCY / 16_000_000u32) as u16) });
        tg0.lactconfig().modify(|_, w| {
            w.increase().bit(true);
            w.autoreload().bit(true);
            w.en().bit(true)
        });
        Self
    }
    #[deprecated(note = "Use SysUptime::try_now() instead")]
    #[allow(missing_docs)]
    pub fn current_time(&self) -> <Self as counters::TimeCount>::TickMeasure  {
        self.try_now().unwrap()
    }
}
