//! Imperfect — Result extended with partial success.
//!
//! Three states: Ok (perfect), Partial (value with loss), Err (failure).
//! Derived from partial successes in PbtA game design.
//!
//! `Loss` is a trait. `ShannonLoss` (information loss in bits) is the
//! default implementation.
