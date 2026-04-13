//! Bundle — principal bundle tower with connection.
//!
//! Five traits forming a supertrait chain:
//! Fiber → Connection → Gauge → Transport → Closure → Bundle (blanket)
//!
//! The same mathematical object at every scale:
//! Fate chip, BEAM runtime, Mirror compiler.

use std::convert::Infallible;
use terni::{Imperfect, Loss};

/// Level 0: the observed state. The section of the bundle.
/// Abyss. The fiber.
pub trait Fiber {
    type State;
}

/// Level 1: the optic that determines how information transports.
/// Introject. The connection on the principal bundle.
pub trait Connection: Fiber {
    type Optic;
    fn connection(&self) -> &Self::Optic;
}

/// Level 2: the structure group. Which decomposition strategy.
/// Cartographer. The gauge transformation.
pub trait Gauge: Connection {
    type Group;
    fn gauge(&self) -> &Self::Group;
}

/// Level 3: parallel transport with holonomy.
/// Explorer. Comprehension always costs something.
/// The holonomy IS the loss. Returns Partial by design.
pub trait Transport: Gauge {
    type Holonomy: Loss;
    fn transport(&self, state: &Self::State) -> Imperfect<Self::State, Infallible, Self::Holonomy>;
}

/// Level 4: autopoietic closure. The Lawvere fixed point.
/// Fate. selectors[4] = self-reference.
pub trait Closure: Transport {
    type Fixed;
    fn close(&self) -> &Self::Fixed;
}

/// A complete principal bundle tower.
/// Blanket impl: any type that implements all five levels is a Bundle.
pub trait Bundle: Closure {}
impl<T: Closure> Bundle for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScalarLoss;

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
        let c = TestConnection {
            optic: "lens".to_string(),
        };
        assert_eq!(c.connection(), "lens");
    }

    #[test]
    fn connection_has_state_from_fiber() {
        let _: <TestConnection as Fiber>::State = [1.0, 2.0, 3.0, 4.0];
    }

    struct TestBundle {
        optic: String,
        strategy: u8,
        fixed: bool,
    }

    impl Fiber for TestBundle {
        type State = [f64; 4];
    }

    impl Connection for TestBundle {
        type Optic = String;
        fn connection(&self) -> &String {
            &self.optic
        }
    }

    impl Gauge for TestBundle {
        type Group = u8;
        fn gauge(&self) -> &u8 {
            &self.strategy
        }
    }

    impl Transport for TestBundle {
        type Holonomy = ScalarLoss;
        fn transport(&self, state: &[f64; 4]) -> Imperfect<[f64; 4], Infallible, ScalarLoss> {
            let compressed = [state[0], state[1], 0.0, 0.0];
            let loss = state[2].abs() + state[3].abs();
            if loss == 0.0 {
                Imperfect::Success(compressed)
            } else {
                Imperfect::Partial(compressed, ScalarLoss::new(loss))
            }
        }
    }

    impl Closure for TestBundle {
        type Fixed = bool;
        fn close(&self) -> &bool {
            &self.fixed
        }
    }

    #[test]
    fn gauge_requires_connection() {
        let b = TestBundle {
            optic: "traversal".to_string(),
            strategy: 3,
            fixed: true,
        };
        assert_eq!(*b.gauge(), 3);
    }

    #[test]
    fn transport_returns_partial() {
        let b = TestBundle {
            optic: "traversal".to_string(),
            strategy: 3,
            fixed: true,
        };
        let state = [1.0, 2.0, 3.0, 4.0];
        let result = b.transport(&state);
        assert!(result.is_partial());
    }

    #[test]
    fn transport_holonomy_measures_loss() {
        let b = TestBundle {
            optic: "traversal".to_string(),
            strategy: 3,
            fixed: true,
        };
        let state = [1.0, 2.0, 3.0, 4.0];
        match b.transport(&state) {
            Imperfect::Partial(compressed, loss) => {
                assert_eq!(compressed, [1.0, 2.0, 0.0, 0.0]);
                assert_eq!(loss.as_f64(), 7.0);
            }
            _ => panic!("expected Partial"),
        }
    }

    #[test]
    fn transport_zero_loss_returns_success() {
        let b = TestBundle {
            optic: "traversal".to_string(),
            strategy: 3,
            fixed: true,
        };
        let state = [1.0, 2.0, 0.0, 0.0];
        let result = b.transport(&state);
        assert!(result.is_ok());
    }

    #[test]
    fn closure_is_fixed_point() {
        let b = TestBundle {
            optic: "traversal".to_string(),
            strategy: 3,
            fixed: true,
        };
        assert_eq!(*b.close(), true);
    }

    #[test]
    fn full_tower_is_bundle() {
        fn accepts_bundle<B: Bundle>(_b: &B) {}
        let b = TestBundle {
            optic: "traversal".to_string(),
            strategy: 3,
            fixed: true,
        };
        accepts_bundle(&b);
    }

    #[test]
    fn bundle_associated_types_accessible() {
        fn read_tower<B: Bundle>(b: &B) -> bool
        where
            B::Fixed: Copy + Into<bool>,
        {
            (*b.close()).into()
        }
        let b = TestBundle {
            optic: "traversal".to_string(),
            strategy: 3,
            fixed: true,
        };
        assert!(read_tower(&b));
    }
}
