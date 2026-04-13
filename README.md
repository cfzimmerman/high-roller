# High Roller

### TODO

The code in this crate is solid, but the profiling and
documentation is lacking. The motivating case for this
crate is time pressured, but I will be iteratively returning
to improve and prettify as time allows.

### Roadmap

High Roller is usable as-is, but here's the plan for
upcoming improvements.

- Finish documentation on RollingMax. Proper documentation on RollingSum.
- Write a proper README.
- Performance profiling
    - Compare naive and optimized rolling sum.
    - Compare naive and optimized rolling max.
    - Compare ringbuffer and VecDeque rolling max.
    - Performance profile of Decimal32 operations. How do instructions compare to f32?
- BitVec rolling sum for accumulating a window of boolean counters.
- DoubleBitVec and DBV rolling sum for accumulating the range 0..4.
- Experiment with size optimizations in RollingMax.

