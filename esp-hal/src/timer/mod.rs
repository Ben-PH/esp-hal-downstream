//! General-purpose timers.

use fugit::{ExtU64, Instant, MicrosDurationU64};

#[cfg(systimer)]
pub mod systimer;
#[cfg(any(timg0, timg1))]
pub mod timg;

/// Timer errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    /// The timer is already active.
    TimerActive,
    /// The timer is not currently active.
    TimerInactive,
    /// The alarm is not currently active.
    AlarmInactive,
}

/// Functionality provided by any timer peripheral.
pub trait Timer: crate::private::Sealed {
    /// Start the timer.
    fn start(&self);

    /// Stop the timer.
    fn stop(&self);

    /// Reset the timer value to 0.
    fn reset(&self);

    /// Is the timer running?
    fn is_running(&self) -> bool;

    /// The current timer value.
    fn now(&self) -> Instant<u64, 1, 1_000_000>;

    /// Load a target value into the timer.
    fn load_value(&self, value: MicrosDurationU64);

    /// Enable auto reload of the loaded value.
    fn enable_auto_reload(&self, auto_reload: bool);

    /// Enable or disable the timer's interrupt.
    fn enable_interrupt(&self, state: bool);

    /// Clear the timer's interrupt.
    fn clear_interrupt(&self);

    /// Has the timer triggered?
    fn is_interrupt_set(&self) -> bool;

    /// FIXME: This is (hopefully?) temporary...
    fn set_alarm_active(&self, state: bool);
}

/// A one-shot timer.
pub struct OneShotTimer<T> {
    inner: T,
}

impl<T> OneShotTimer<T>
where
    T: Timer,
{
    /// Construct a new instance of [`OneShotTimer`].
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Pauses execution for *at least* `ms` milliseconds.
    pub fn delay_millis(&self, ms: u32) {
        self.delay((ms as u64).millis());
    }

    /// Pauses execution for *at least* `us` microseconds.
    pub fn delay_micros(&self, us: u32) {
        self.delay((us as u64).micros());
    }

    /// Pauses execution for *at least* `ns` nanoseconds.
    pub fn delay_nanos(&self, ns: u32) {
        self.delay((ns as u64 / 1000).micros())
    }

    fn delay(&self, us: MicrosDurationU64) {
        if self.inner.is_running() {
            self.inner.stop();
        }

        self.inner.clear_interrupt();
        self.inner.reset();

        self.inner.enable_auto_reload(false);
        self.inner.load_value(us);
        self.inner.start();

        while !self.inner.is_interrupt_set() {
            // Wait
        }

        self.inner.stop();
        self.inner.clear_interrupt();
    }
}

#[cfg(feature = "embedded-hal-02")]
impl<T, UXX> embedded_hal_02::blocking::delay::DelayMs<UXX> for OneShotTimer<T>
where
    T: Timer,
    UXX: Into<u32>,
{
    fn delay_ms(&mut self, ms: UXX) {
        self.delay_millis(ms.into());
    }
}

#[cfg(feature = "embedded-hal-02")]
impl<T, UXX> embedded_hal_02::blocking::delay::DelayUs<UXX> for OneShotTimer<T>
where
    T: Timer,
    UXX: Into<u32>,
{
    fn delay_us(&mut self, us: UXX) {
        self.delay_micros(us.into());
    }
}

#[cfg(feature = "embedded-hal")]
impl<T> embedded_hal::delay::DelayNs for OneShotTimer<T>
where
    T: Timer,
{
    #[allow(clippy::useless_conversion)]
    fn delay_ns(&mut self, ns: u32) {
        self.delay_nanos(ns.into());
    }
}

/// A periodic timer.
pub struct PeriodicTimer<T> {
    inner: T,
}

impl<T> PeriodicTimer<T>
where
    T: Timer,
{
    /// Construct a new instance of [`PeriodicTimer`].
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Start a new count down.
    pub fn start(&mut self, timeout: MicrosDurationU64) {
        if self.inner.is_running() {
            self.inner.stop();
        }

        self.inner.clear_interrupt();
        self.inner.reset();

        self.inner.enable_auto_reload(true);
        self.inner.load_value(timeout);
        self.inner.start();
    }

    /// "Wait" until the count down finishes without blocking.
    pub fn wait(&mut self) -> nb::Result<(), void::Void> {
        if self.inner.is_interrupt_set() {
            self.inner.clear_interrupt();
            self.inner.set_alarm_active(true); // FIXME: Remove if/when able

            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    /// Tries to cancel the active count down.
    pub fn cancel(&mut self) -> Result<(), Error> {
        if !self.inner.is_running() {
            return Err(Error::TimerInactive);
        }

        self.inner.stop();

        Ok(())
    }
}

#[cfg(feature = "embedded-hal-02")]
impl<T> embedded_hal_02::timer::CountDown for PeriodicTimer<T>
where
    T: Timer,
{
    type Time = MicrosDurationU64;

    fn start<Time>(&mut self, timeout: Time)
    where
        Time: Into<Self::Time>,
    {
        self.start(timeout.into());
    }

    fn wait(&mut self) -> nb::Result<(), void::Void> {
        self.wait()
    }
}

#[cfg(feature = "embedded-hal-02")]
impl<T> embedded_hal_02::timer::Cancel for PeriodicTimer<T>
where
    T: Timer,
{
    type Error = Error;

    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.cancel()
    }
}

#[cfg(feature = "embedded-hal-02")]
impl<T> embedded_hal_02::timer::Periodic for PeriodicTimer<T> where T: Timer {}
