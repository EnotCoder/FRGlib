//! Frame pacing: how long a frame may run for, and how long to wait after.
//!
//! Sleeping for a fixed slice after every frame -- `sleep(16ms)` being the
//! usual choice -- does not give 60 fps, it gives "however long the frame took,
//! plus 16ms". At one millisecond of work that is 58.8 fps; at five, 47.6; at
//! sixteen, 31.2. The heavier the drawing, the worse it looks, which is the
//! opposite of what a fixed sleep should do.
//!
//! [`FrameClock`] sleeps the *remainder* instead, so the target is actually
//! held until the work no longer fits in the budget.

use std::time::{Duration, Instant};

/// Default ceiling on `dt`, in seconds.
///
/// Long enough for a genuinely slow frame, short enough that a stall does not
/// teleport everything across the screen.
pub const DEFAULT_MAX_DT: Duration = Duration::from_millis(100);

/// Keeps a loop at a target rate, and reports how long the last frame ran.
///
/// Call [`dt`](Self::dt) at the top of the loop for the value to move things
/// with, and [`tick`](Self::tick) at the bottom to wait out the frame:
///
/// ```no_run
/// # fn main() -> Result<(), frglib::Error> {
/// # let mut window = frglib::Window::builder("demo").build()?;
/// let square = frglib::Object::from_xywh(200.0, 200.0, 200.0, 200.0);
/// let mut clock = frglib::FrameClock::new(60.0);
///
/// loop {
///     let dt = clock.dt();
///     for event in window.poll()? {
///         let _ = event;
///     }
///
///     window.frame_with(|d| {
///         d.fill_object(&square, frglib::Color::WHITE)?;
///         Ok(())
///     })?;
///
///     clock.tick();
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct FrameClock {
    period: Duration,
    last: Instant,
    next: Instant,
    max_dt: Duration,
}

impl FrameClock {
    /// A clock aiming for `hz` frames per second.
    ///
    /// A frequency of zero or less has no sensible period, so anything that is
    /// not positive falls back to 60.
    pub fn new(hz: f64) -> Self {
        FrameClock {
            period: period_for(hz),
            last: Instant::now(),
            next: Instant::now(),
            max_dt: DEFAULT_MAX_DT,
        }
    }

    /// Retargets to `hz` frames per second, counting from now.
    pub fn hz(mut self, hz: f64) -> Self {
        self.period = period_for(hz);
        self.next = Instant::now() + self.period;
        self
    }

    /// Retargets to `period` per frame.
    pub fn period(mut self, period: Duration) -> Self {
        self.period = period;
        self.next = Instant::now() + period;
        self
    }

    /// Caps what [`dt`](Self::dt) may report.
    ///
    /// Without this, a stall -- alt-tab, a breakpoint, a slow first frame --
    /// hands `update` a `dt` of several seconds and everything it is moving
    /// jumps clean off the screen.
    pub fn max_dt(mut self, max_dt: Duration) -> Self {
        self.max_dt = max_dt;
        self
    }

    /// The target period, as configured.
    pub fn period_len(&self) -> Duration {
        self.period
    }

    /// Seconds since the last [`tick`](Self::tick), capped by `max_dt`.
    ///
    /// Measure this at the top of the loop and it covers the previous frame's
    /// work but not the wait that followed it.
    pub fn dt(&self) -> f32 {
        self.last.elapsed().min(self.max_dt).as_secs_f32()
    }

    /// Waits out whatever is left of the frame, then starts the next one.
    ///
    /// Returns immediately when the frame already ran past its budget. Falling
    /// behind does not queue up a burst of catch-up frames: the next deadline
    /// moves to now, not further into the future.
    pub fn tick(&mut self) {
        let now = Instant::now();
        if let Some(left) = remaining(now, self.next) {
            std::thread::sleep(left);
        }

        // Stamped after the sleep on purpose. Before it, the next `dt` would
        // measure the wait along with the work and come out as a full period
        // every time.
        self.last = Instant::now();

        self.next = (self.next + self.period).max(self.last);
    }
}

/// The period a frame rate of `hz` allows, or a fallback if `hz` is not a
/// positive frequency.
fn period_for(hz: f64) -> Duration {
    if hz > 0.0 && hz.is_finite() {
        Duration::from_secs_f64(1.0 / hz)
    } else {
        Duration::from_secs_f64(1.0 / 60.0)
    }
}

/// How long is left until `next`, if any.
///
/// `None` means the deadline has reached or passed, so there is nothing worth
/// sleeping for. Zero counts as nothing: sleeping for it would only cost a
/// call.
fn remaining(now: Instant, next: Instant) -> Option<Duration> {
    match next.checked_duration_since(now) {
        Some(left) if left > Duration::ZERO => Some(left),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame rate maps to the period it allows.
    #[test]
    fn frequency_becomes_its_period() {
        assert_eq!(period_for(60.0), Duration::from_secs_f64(1.0 / 60.0));
        assert_eq!(period_for(30.0), Duration::from_secs_f64(1.0 / 30.0));
        assert_eq!(period_for(144.0), Duration::from_secs_f64(1.0 / 144.0));
    }

    /// Zero, negative and NaN have no period, and must not produce one.
    #[test]
    fn nonsense_frequencies_fall_back_to_sixty() {
        let sixty = period_for(60.0);

        assert_eq!(period_for(0.0), sixty);
        assert_eq!(period_for(-10.0), sixty);
        assert_eq!(period_for(f64::NAN), sixty);
        assert_eq!(period_for(f64::INFINITY), sixty);
    }

    #[test]
    fn a_clock_reports_the_period_it_was_given() {
        assert_eq!(FrameClock::new(30.0).period_len(), period_for(30.0));
        assert_eq!(
            FrameClock::new(60.0)
                .period(Duration::from_millis(5))
                .period_len(),
            Duration::from_millis(5)
        );
    }

    /// A deadline in the future leaves the gap to wait out.
    #[test]
    fn time_left_before_a_deadline() {
        let now = Instant::now();
        let later = remaining(now, now + Duration::from_millis(10));

        assert!(later.is_some());
        assert!(later.unwrap() <= Duration::from_millis(10));
    }

    /// A deadline that has passed means there is nothing left to wait for.
    #[test]
    fn no_time_left_past_the_deadline() {
        let now = Instant::now();
        assert!(remaining(now, now - Duration::from_millis(10)).is_none());
        assert!(remaining(now, now).is_none());
    }

    /// dt starts near zero, since the clock was just made.
    #[test]
    fn dt_is_near_zero_right_after_construction() {
        assert!(FrameClock::new(60.0).dt() < 0.05);
    }

    /// `tick` must not sleep when the frame already blew its budget, so a
    /// caller can hand it a period of zero and get an instant return.
    #[test]
    fn tick_is_immediate_when_nothing_is_owed() {
        let mut clock = FrameClock::new(60.0).period(Duration::from_millis(0));

        let start = Instant::now();
        clock.tick();
        clock.tick();

        assert!(start.elapsed() < Duration::from_millis(50));
    }

    /// Behind schedule must not turn into a burst of catch-up frames: after a
    /// tick that found nothing to wait for, the next one owes nothing either.
    #[test]
    fn falling_behind_does_not_accumulate_a_debt() {
        let mut clock = FrameClock::new(1000.0).period(Duration::from_millis(1));

        // Owes a wait, then does not: the first call consumed the deadline.
        std::thread::sleep(Duration::from_millis(3));
        clock.tick();

        // A second immediate tick must not owe a further millisecond either,
        // which it would if `next` had been left behind.
        let start = Instant::now();
        clock.tick();
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    /// A cap keeps a stall from turning into one enormous step.
    #[test]
    fn dt_stops_at_max_dt() {
        let clock = FrameClock::new(60.0).max_dt(Duration::from_millis(10));
        std::thread::sleep(Duration::from_millis(40));

        let dt = clock.dt();
        assert!(dt <= 0.0101, "dt {dt} should be capped at 10ms");
        assert!(dt > 0.009, "dt {dt} should be near the cap, not zero");
    }

    /// A generous cap must not get in the way.
    #[test]
    fn dt_passes_through_under_the_cap() {
        let clock = FrameClock::new(60.0).max_dt(Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(20));

        let dt = clock.dt();
        assert!(dt > 0.015, "dt {dt} should reflect the real wait");
    }
}
