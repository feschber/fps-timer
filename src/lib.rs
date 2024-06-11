use std::{
    hint, thread,
    time::{Duration, Instant},
};

/// Timer instance
pub struct Timer {
    /// instant of the previous call to frame()
    previous: Instant,
    /// instant of the previous call to log()
    previous_log: Instant,
    /// target time for the next frame
    target: Instant,
    /// target time for the next log
    log_target: Instant,
    /// time between two frames
    delta_time: Duration,
    /// time interval between two logs
    log_interval: Duration,
    /// count of frames the last time log() was called
    prev_framecount: u64,
    /// current frame count
    framecount: u64,
    /// maximum amount of frames to lag behind
    max_delay_frames: u32,
    /// improved_accuracy
    high_precision: bool,
}

/// since thread::sleep usually is not accurate down to the millisecond, we
/// only suspend the thread for max(delay - 1ms, 0)
/// and spin in a loop for the rest of the time
///
/// returns the last measured timestamp
fn sleep_until_high_precision(target: Instant) -> Instant {
    // calculate approximate duration until target time
    let now = Instant::now();

    // early out to avoid additional measurement
    if now >= target {
        return now;
    }

    // calculate the required wait duration
    let approx_duration = target.duration_since(now);

    // sleep for a maximum of 1ms less than the approximate required delay
    // (0.250ms on unix)
    #[cfg(unix)]
    const MAX_BUSY_WAIT: Duration = Duration::from_micros(250);
    #[cfg(not(unix))]
    const MAX_BUSY_WAIT: Duration = Duration::from_millis(1);
    if approx_duration > MAX_BUSY_WAIT {
        thread::sleep(approx_duration - MAX_BUSY_WAIT);
    }

    busy_wait_until(target)
}

fn sleep_until(target: Instant) -> Instant {
    // calculate approximate duration until target time
    let now = Instant::now();

    // early out to avoid additional measurement
    if now >= target {
        return now;
    }

    let suspend_duration = target - now;
    thread::sleep(suspend_duration);
    busy_wait_until(target)
}

fn busy_wait_until(target: Instant) -> Instant {
    // spin until target time is reached and return it
    loop {
        let time = Instant::now();
        if time >= target {
            break time;
        }
        hint::spin_loop();
    }
}

/// A struct holding information about the previous logging interval
#[derive(Debug)]
pub struct Log {
    /// average delta time between frames since the last call to [`Timer::log`]
    delta_avg: Duration,
}

impl Log {
    /// frame time averaged over the interval since the last call to [`Timer::log`]
    pub fn delta_time_avg(&self) -> Duration {
        self.delta_avg
    }

    /// frame time averaged over the interval since the last call to [`Timer::log`]
    /// in milliseconds
    pub fn delta_time_avg_ms(&self) -> f64 {
        self.delta_avg.as_secs_f64() * 1000.
    }

    /// fps averaged over the interval since the last call to [`Timer::log`]
    pub fn fps_average(&self) -> f64 {
        1. / self.delta_avg.as_secs_f64()
    }
}

impl Default for Timer {
    fn default() -> Self {
        let now = Instant::now();
        let delta_time = Duration::from_secs_f64(1.0 / 60.);
        let log_interval = Duration::from_millis(100);
        Self {
            framecount: 0,
            log_interval,
            previous: now,
            target: now + delta_time,
            previous_log: now,
            prev_framecount: 0,
            log_target: now + log_interval,
            delta_time,
            max_delay_frames: 2,
            high_precision: true,
        }
    }
}

impl Timer {
    /// Sets the logging interval of this timer to `log_interval`.
    ///
    /// # Arguments
    /// * `log_interval` - logging interval as used by [`Self::log`]
    ///
    /// # Returns
    /// [`Self`] the (modified) timer
    ///
    /// # Example
    /// ```rust
    /// use std::time::Duration;
    /// use fps_timer::Timer;
    /// let mut timer = Timer::default()
    ///     .log_interval(Duration::from_millis(100))
    ///     .fps(240.);
    /// ```
    pub fn log_interval(mut self, log_interval: Duration) -> Self {
        self.log_interval = log_interval;
        self.log_target = self.previous + log_interval;
        self
    }

    /// Sets the target frametime to the specified amount.
    ///
    /// # Arguments
    /// * `delta` - target frametime
    ///
    /// # Returns
    /// [`Self`] the (modified) timer
    ///
    /// # Example
    /// ```rust
    /// use std::time::Duration;
    /// use fps_timer::Timer;
    /// let mut timer = Timer::default()
    ///     .frame_time(Duration::from_secs_f64(1. / 60.));
    /// ```
    pub fn frame_time(mut self, delta: Duration) -> Self {
        self.delta_time = delta;
        self.target = self.previous + delta;
        self
    }

    /// Sets the framerate target to the specified amount.
    ///
    /// # Arguments
    /// * `fps` - target framerate
    ///
    /// # Returns
    /// [`Self`] the (modified) timer
    ///
    /// # Example
    /// ```rust
    /// use fps_timer::Timer;
    /// let mut timer = Timer::default()
    ///     .fps(60.);
    /// ```
    pub fn fps(self, fps: f64) -> Self {
        let duration = match fps {
            0. => Duration::ZERO,
            fps => Duration::from_secs_f64(1. / fps),
        };
        self.frame_time(duration)
    }

    /// Enable or disable improved accuracy for this timer.
    ///
    /// Enabling high precision makes the timer more precise
    /// at the cost of higher power consumption because
    /// part of the duration is awaited in a busy spinloop.
    ///
    /// Defaults to `true`
    ///
    /// # Arguments
    /// * `enable` - whether or not to enable higher precision
    ///
    /// # Returns
    /// [`Self`] the (modified) timer
    ///
    /// # Example
    /// ```rust
    /// use fps_timer::Timer;
    /// let mut timer = Timer::default()
    ///     .fps(60.);
    /// ```
    pub fn high_precision(mut self, enabled: bool) -> Self {
        self.high_precision = enabled;
        self
    }

    /// Waits until the specified frametime target is reached
    /// and returns the [`Duration`] since the last call
    /// to [`Self::frame()`] of this [`Timer`] (= frametime).
    ///
    /// # Example
    /// ```no_run
    /// use std::time::Duration;
    /// use fps_timer::Timer;
    ///
    /// fn update(dt: Duration) {
    ///     // game logic
    /// }
    ///
    /// fn main()  {
    ///     let mut timer = Timer::default();
    ///     loop {
    ///         let delta_time = timer.frame();
    ///         update(delta_time);
    ///     }
    /// }
    /// ```
    pub fn frame(&mut self) -> Duration {
        // increment framecount
        self.framecount += 1;

        // get current time
        let mut current = Instant::now();

        if self.delta_time > Duration::ZERO {
            // calculate if frame was too late
            let behind = if current > self.target {
                current - self.target
            } else {
                Duration::ZERO
            };

            // If the frame is more than `slack` behind,
            // we update the target to the current time,
            // scheduling the next frame for `current + delta_time`.
            //
            // Otherwise, the next frame is scheduled for
            // `prev_target + delta_time` to allow the timer to catch up.
            if behind > self.slack() {
                self.target = current;
            }

            // wait until target instant if needed
            if current < self.target {
                current = if self.high_precision {
                    sleep_until_high_precision(self.target)
                } else {
                    sleep_until(self.target)
                };
            }

            // update target time
            self.target += self.delta_time;
        }

        // calculate frame_time and update previous time
        let frame_time = current.duration_since(self.previous);
        self.previous = current;
        frame_time
    }

    /// returns [`Some<Log>`], holding information
    /// about the previous logging interval, every time
    /// the interval specified by [`Timer::log_interval`] has passed
    /// and [`None`] otherwise
    pub fn log(&mut self) -> Option<Log> {
        // check if it's time to log fps
        let current = self.previous;
        if current < self.log_target {
            return None;
        }

        // frames since last log (guaranteed to be at least 1)
        let frames = (self.framecount - self.prev_framecount) as u32;

        // avg frametime = duration / (frames in this duration)
        let delta_avg = current.duration_since(self.previous_log) / frames;

        // set time of current and next log (current time + log interval)
        self.log_target = current + self.log_interval;
        self.previous_log = current;
        self.prev_framecount = self.framecount;

        Some(Log { delta_avg })
    }

    /// The slack of the timer, i.e. the amount of time in which a game
    /// is allowed to lag behind while allowing it to catch up.
    /// If the game lags behind more than this slack, the target frame
    /// time is relaxed to not fall behind completely.
    fn slack(&self) -> Duration {
        self.max_delay_frames * self.delta_time
    }
}
