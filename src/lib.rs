// #![no_std]

pub mod decimal;
pub mod rolling_max;
pub mod rolling_sum;

/*

TODO
- Better tests
- RollingMax:
    - Save RollingMax current impl for performance testing
    - Use RingBuffer instead
    - Size optimize expirations

*/
