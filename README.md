# High Roller

This `no_std` library includes tools for tracking rolling-window
statistics in latency-sensitive systems. The motivating case was
reporting downsampled performance telemetry in embedded applications,
but it's hopefully useful for other domains as well.

This crate contains three members:
- Rolling Max: tracks the greatest value in a fixed-size window.
- Rolling Sum: tracks the sum of entries in a fixed-size window.
- Decimal32: a 32-bit fixed-precision decimal type. Pairs with `RollingSum`
  when float precision is needed. Native floating point types are incompatible
  with `RollingSum`'s overflow recovery mechanism.

This crate has the following design motivations:
- Algorithmic optimality: Max and overflow-resilient Sum expose asymptotically
  optimal operations.
- Performance orientation: no_std, no heap allocs, and a performance-aware
  approach to implementation. Demonstrated performance improvement contributions
  are also very welcome.
- Code simplicity: this crate has more lines of docs than actual code. When feature
  availability and simplicity collide, the latter is chosen.

# Example

The example below shows how `high_roller` could be used to track
and publish request latency telemetry in an application with
scheduled ticks and structured I/O patterns.

The example expects a request every 1/3 ticks, publishes telemetry
every 100 ticks, and tracks a window of 1000 ticks.

```rust
use core::cmp::Reverse;

use high_roller::decimal::D5;
use high_roller::rolling_max::RollingMax;
use high_roller::rolling_sum::RollingSum;

/// Track a rolling window of this many ticks.
const WINDOW: usize = 1000;

/// Expect a request every 1 / 3 ticks.
const EXPECTED_INTERVAL: u32 = 3;

/// Emits telemetry on the rolling window of 1000 ticks.
/// Emitted every 100 ticks.
#[allow(unused)]
struct TelemetryMsg {
    /// The maximum number of ticks between requests.
    max_latency: Option<u32>,

    /// The minimum number of ticks between requests.
    min_latency: Option<u32>,

    /// Root Mean Square Error of request latency from what is expected.
    rmse: Option<D5>,
}

let mut io = IoLayer::new();
let mut telemetry = Telemetry::default();

while io.tick() {
    let req = io.next_request();
    if let Some(req) = &req {
        process_request(req);
    }
    telemetry.log_tick(req.is_some());

    if io.count % 100 == 0 {
        let max_latency = telemetry.max_latency_ticks.max().copied();
        let min_latency = telemetry.min_latency_ticks.max().map(|m| m.0);
        let rmse = {
            let sum_sq = telemetry.rmse_acc.total().copied().map(D5::get);
            let sample_ct = telemetry.rmse_samples.total().copied().unwrap_or(0);
            sum_sq.and_then(|sum_sq| {
                (sample_ct != 0)
                    .then(|| (sum_sq / sample_ct as f64).sqrt())
                    .map(D5::cast)
            })
        };

        io.log_telemetry(TelemetryMsg {
            max_latency,
            min_latency,
            rmse,
        });
    }
}

/// An accumulator for dynamic system telemetry.
#[derive(Default)]
struct Telemetry {
    tick: u32,
    last_req_tick: u32,
    rmse_acc: RollingSum<D5, WINDOW>,
    rmse_samples: RollingSum<u32, WINDOW>,
    max_latency_ticks: RollingMax<u32, WINDOW>,
    min_latency_ticks: RollingMax<Reverse<u32>, WINDOW>,
}

impl Telemetry {
    /// Call this once every tick to log statistics based on
    /// whether a request was received.
    fn log_tick(&mut self, received_req: bool) {
        self.tick = self.tick.wrapping_add(1);

        if !received_req {
            self.max_latency_ticks.push(0);
            self.min_latency_ticks.push(Reverse(u32::MAX));
            self.rmse_acc.add(D5::ZERO);
            self.rmse_samples.add(0);
            return;
        }

        let interval = self
            .tick
            .checked_sub(self.last_req_tick)
            .expect("irrational last_req");
        self.last_req_tick = self.tick;

        // RMSE = sqrt(mean(sq_err)).
        // Saturate worst-case error at `D5::MAX`.
        let sq_err = D5::checked((interval as f64 - EXPECTED_INTERVAL as f64).powf(2.))
            .unwrap_or(D5::MAX);

        self.max_latency_ticks.push(interval);
        self.min_latency_ticks.push(Reverse(interval));
        self.rmse_acc.add(sq_err);
        self.rmse_samples.add(1);
    }
}

/// A dummy I/O layer.
struct IoLayer {
    rng: rand::rngs::ThreadRng,
    dist: rand::distr::Bernoulli,
    // How many ticks this contrived IO stack will sustain. Otherwise the
    // example would run forever.
    count: usize,
}

impl IoLayer {
    /// Creates a new IoLayer instance. A real app presumably loops forever,
    /// but this dummy stack self-destructs after a certain number of ticks.
    fn new() -> Self {
        Self {
            rng: rand::rng(),
            dist: rand::distr::Bernoulli::from_ratio(1, 3).expect("good range"),
            count: 10_000,
        }
    }

    /// Returns Some(Request) if one was received and None if not.
    fn next_request(&mut self) -> Option<Request> {
        use rand::distr::Distribution;
        self.dist.sample(&mut self.rng).then_some(Request)
    }

    /// In a real system, this would implement some timed tick mechanism.
    /// Here it's just a pass through. Returns false if the example should exit.
    fn tick(&mut self) -> bool {
        let prev = self.count;
        self.count = prev.saturating_sub(1);
        prev != 0
    }

    /// Pushes a telemetry message into some structured logging pipeline.
    fn log_telemetry(&self, msg: TelemetryMsg) {
        core::hint::black_box((&self, msg));
    }
}

struct Request;

fn process_request(req: &Request) {
    core::hint::black_box(req);
}

```


### Roadmap

High Roller is usable as-is, but here's the plan for
upcoming improvements.

- Performance profiling
    - Compare naive and optimized rolling sum.
    - Compare naive and optimized rolling max.
    - Compare ringbuffer and VecDeque rolling max.
    - Performance profile of Decimal32 operations. How do instructions compare to f32?
- Experiment with size optimizations in RollingMax.
- BitVec rolling sum for accumulating a window of boolean counters.
- DoubleBitVec and DBV rolling sum for accumulating the range 0..4.
