#![no_std]

pub mod decimal;
pub mod rolling_max;
pub mod rolling_sum;

/*

TODO
- Decimal
    - Within u16
    - Within u32
    - Implement traits on it and use within rolling sum
- Better tests
    - Both modules should have a
- RollingMax:
    - Save RollingMax current impl for performance testing
    - Use RingBuffer instead
    - Size optimize expirations

*/
