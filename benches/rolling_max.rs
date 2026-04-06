use std::{collections::VecDeque, num::NonZeroUsize, ops::Range};

use criterion::{criterion_group, criterion_main, Criterion};
use high_roller::rolling_max::RollingMax;
use rand::{
    distr::{uniform::SampleUniform, Uniform},
    rngs::SmallRng,
    RngExt, SeedableRng,
};
use std::hint::black_box;

/*

Compare:
- Std
- Naive
- RollingMax
- RollingSum

*/

const QLEN: usize = 600;
const STREAM_LEN: usize = 100_000;

fn sampler<T>(range: Range<T>) -> impl Iterator<Item = T>
where
    T: SampleUniform,
{
    // Unwrap is fine since this is test code, and the range's
    // acceptability will never change.
    #[allow(clippy::unwrap_used)]
    SmallRng::seed_from_u64(119)
        .sample_iter(Uniform::new(range.start, range.end).unwrap())
        .take(STREAM_LEN)
}

fn bench_rolling_max<const WINDOW: usize>(c: &mut Criterion) {
    c.bench_function("rng_baseline_stream", |b| {
        b.iter(|| {
            for num in self::sampler(0f32..100f32) {
                std::hint::black_box(num);
            }
        });
    });

    c.bench_function(&format!("rolling_max_{WINDOW}"), |b| {
        b.iter(|| {
            let mut roller: RollingMax<i32, WINDOW> = RollingMax::new();
            for num in self::sampler(i32::MIN..i32::MAX) {
                roller.push(num);
                std::hint::black_box(roller.max());
            }
        });
    });
}

fn rolling_max(c: &mut Criterion) {
    for qlen in [6, 60, 600, 6000, 60_000] {}
}

criterion_group!(benches, rolling_max);
criterion_main!(benches);

#[derive(Debug)]
pub struct RollingMaxV1<T> {
    // Invariant: These are 1:1. They are conceptually a `(usize, T)` tuple,
    // but split into two deques to avoid alignment padding when T is narrower
    // than usize (e.g. u8/u16 on 64-bit targets).
    deq: VecDeque<T>,
    expires: VecDeque<usize>,

    ct: usize,
    cap: usize,
}

impl<T> RollingMaxV1<T>
where
    T: PartialOrd,
{
    #[must_use]
    pub fn new(capacity: NonZeroUsize) -> Self {
        let cap: usize = capacity.into();
        Self {
            deq: VecDeque::with_capacity(cap),
            expires: VecDeque::with_capacity(cap),
            cap,
            ct: 0,
        }
    }

    pub fn push(&mut self, entry: T) {
        self.ct = self.ct.wrapping_add(1);

        while self
            .expires
            .front()
            .is_some_and(|&exp| self.ct.wrapping_sub(exp) <= self.cap)
        {
            self.deq.pop_front();
            self.expires.pop_front();
        }

        while self.deq.back().is_some_and(|tail| tail <= &entry) {
            self.deq.pop_back();
            self.expires.pop_back();
        }

        self.deq.push_back(entry);
        self.expires.push_back(self.ct.wrapping_add(self.cap));
    }

    #[must_use]
    pub fn max(&self) -> Option<&T> {
        self.deq.front()
    }
}
