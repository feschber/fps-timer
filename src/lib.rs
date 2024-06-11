use std::{
    hint, thread,
    time::{Duration, Instant},
};

pub struct Timer {
    previous: Instant,
    previous_log: Instant,
    target: Instant,
    log_target: Instant,
    delta_time: Duration,
    log_interval: Duration,
    prev_framecount: u64,
    framecount: u64,
}

/// since thread::sleep usually is not accurate down to the millisecond, we
/// only suspend the thread for max(delay - 1ms, 0)
/// and spin in a loop for the rest of the time
///
/// returns the last measured timestamp
fn sleep_until(target: Instant) -> Instant {
    // calculate approximate duration until target time
    let now = Instant::now();

    // early out to avoid additional measurement
    if now >= target {
        return now;
    }

    // calculate the required wait duration
    let approx_duration = target.duration_since(now);

    // sleep for a maximum of 1ms less than the approximate required delay
    const MILLISECOND: Duration = Duration::from_millis(1);
    if approx_duration > MILLISECOND {
        let suspend_duration = approx_duration - MILLISECOND;
        thread::sleep(suspend_duration);
    }

    // spin until target time is reached and return it
    loop {
        let time = Instant::now();
        if time >= target {
            break time;
        }
        hint::spin_loop();
    }
}

pub struct Log {
    delta_avg: Duration,
}

impl Log {
    pub fn delta_time_avg(&self) -> Duration {
        self.delta_avg
    }

    pub fn delta_time_avg_ms(&self) -> f64 {
        self.delta_avg.as_secs_f64() * 1000.
    }

    pub fn fps_average(&self) -> f64 {
        1. / self.delta_avg.as_secs_f64()
    }
}

impl Default for Timer {
    fn default() -> Self {
        let now = Instant::now();
        let delta_time = Duration::from_secs_f64(1.0 / 60.);
        let log_interval = Duration::from_millis(10);
        Self {
            framecount: 0,
            log_interval,
            previous: now,
            target: now + delta_time,
            previous_log: now,
            prev_framecount: 0,
            log_target: now + log_interval,
            delta_time,
        }
    }
}

impl Timer {
    pub fn log_interval(mut self, log_interval: Duration) -> Self {
        self.log_interval = log_interval;
        self.log_target = self.previous + log_interval;
        self
    }

    pub fn frame_time(mut self, delta: Duration) -> Self {
        self.delta_time = delta;
        self.target = self.previous + delta;
        self
    }

    pub fn fps(self, fps: f64) -> Self {
        self.frame_time(Duration::from_secs_f64(1. / fps))
    }

    pub fn frame(&mut self) -> Duration {
        // increment framecount
        self.framecount += 1;

        // get current time
        let mut current = Instant::now();

        // wait until target instant if needed
        if current < self.target {
            current = sleep_until(self.target);
        }

        // duration since last frame
        let frame_time = current.duration_since(self.previous);

        // update target time and actual time
        self.target += self.delta_time;
        self.previous = current;
        frame_time
    }

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
        self.log_target += self.log_interval;
        self.previous_log = current;
        self.prev_framecount = self.framecount;

        Some(Log { delta_avg })
    }
}
