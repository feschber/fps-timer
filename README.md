## FPS Timer

Fps timer implementation with very accurate timings.

### Example

```rust
use std::{env, io::Write, time::Duration};

use fps_timer::Timer;

fn main() {
    let args: Vec<String> = env::args().collect();
    let fps = args
        .get(1)
        .map(|arg| arg.parse().ok())
        .flatten()
        .unwrap_or(420.69);

    let mut timer = Timer::default()
        .log_interval(Duration::from_millis(10))
        .fps(fps);

    loop {
        let _dt = timer.frame();
        if let Some(log) = timer.log() {
            print!(
                "{:>15.6}ms ({:>10.3}fps)      \r",
                log.delta_time_avg_ms(),
                log.fps_average()
            );
            let _ = std::io::stdout().flush();
        }
    }
}
```
