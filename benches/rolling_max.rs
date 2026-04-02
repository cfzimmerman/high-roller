use std::{collections::VecDeque, num::NonZeroUsize};

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
