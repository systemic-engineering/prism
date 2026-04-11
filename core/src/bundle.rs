//! Bundle — principal bundle tower with connection.
//!
//! Five traits forming a supertrait chain:
//! Fiber → Connection → Gauge → Transport → Closure → Bundle (blanket)
//!
//! The same mathematical object at every scale:
//! Fate chip, BEAM runtime, Mirror compiler.

use imperfect::{Imperfect, Loss};

#[cfg(test)]
mod tests {
    use super::*;
    use imperfect::ShannonLoss;

    struct TestFiber;

    impl Fiber for TestFiber {
        type State = [f64; 4];
    }

    #[test]
    fn fiber_has_state() {
        let _f = TestFiber;
        let _: <TestFiber as Fiber>::State = [1.0, 2.0, 3.0, 4.0];
    }

    struct TestConnection {
        optic: String,
    }

    impl Fiber for TestConnection {
        type State = [f64; 4];
    }

    impl Connection for TestConnection {
        type Optic = String;
        fn connection(&self) -> &String {
            &self.optic
        }
    }

    #[test]
    fn connection_requires_fiber() {
        let c = TestConnection { optic: "lens".to_string() };
        assert_eq!(c.connection(), "lens");
    }

    #[test]
    fn connection_has_state_from_fiber() {
        let _: <TestConnection as Fiber>::State = [1.0, 2.0, 3.0, 4.0];
    }
}
