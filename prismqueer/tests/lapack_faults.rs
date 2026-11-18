//! Every LAPACK and Fortran failure is a value (docs/spec/lapack-failures.md).
//!
//! What a routine would refuse is refused first, in Rust: a dimension past a
//! C `int` is the argument LAPACK would name, a slice of the wrong length is a
//! `Shape`. Neither reaches Fortran, where the first would come back as
//! `INFO < 0` and the second would be read or written out of bounds.

use prismqueer::ffi::{self, Caught, Fault, Routine};
use terni::Imperfect;

const PAST_INT: usize = i32::MAX as usize + 1;

fn pre(routine: Routine, position: i32) -> Fault {
    Fault::Argument {
        routine,
        position,
        caught: Caught::PreCheck,
    }
}

#[test]
fn a_dimension_past_a_c_int_is_the_argument_lapack_would_refuse() {
    assert_eq!(ffi::eigenvalues(PAST_INT, &[]), Err(pre(Routine::Dsyev, 3)));
    assert_eq!(
        ffi::eigensystem(PAST_INT, &[]).map(|_| ()),
        Err(pre(Routine::Dsyev, 3))
    );
    assert_eq!(
        ffi::singular_values(PAST_INT, 1, &[]),
        Err(pre(Routine::Dgesvd, 3))
    );
    assert_eq!(
        ffi::svd(1, PAST_INT, &[]).map(|_| ()),
        Err(pre(Routine::Dgesvd, 4))
    );
    // Even where m·n = 0 would have returned empty without a call.
    assert_eq!(
        ffi::singular_values(0, PAST_INT, &[]),
        Err(pre(Routine::Dgesvd, 4))
    );
    assert_eq!(
        ffi::compose(PAST_INT, &[], &[]),
        Err(pre(Routine::Compose, 1))
    );
}

#[test]
fn a_slice_of_the_wrong_length_is_a_shape_not_a_panic_or_a_stray_read() {
    let shape = |routine, expected, found| Fault::Shape {
        routine,
        expected,
        found,
    };
    assert_eq!(
        ffi::eigenvalues(3, &[1.0]),
        Err(shape(Routine::Dsyev, 9, 1))
    );
    assert_eq!(
        ffi::eigensystem(2, &[1.0; 5]).map(|_| ()),
        Err(shape(Routine::Dsyev, 4, 5))
    );
    assert_eq!(
        ffi::eigenvalues(0, &[1.0]),
        Err(shape(Routine::Dsyev, 0, 1))
    );
    assert_eq!(
        ffi::singular_values(2, 3, &[1.0; 4]),
        Err(shape(Routine::Dgesvd, 6, 4))
    );
    assert_eq!(
        ffi::svd(3, 2, &[1.0; 7]).map(|_| ()),
        Err(shape(Routine::Dgesvd, 6, 7))
    );
    let p = [1.0, 0.0, 0.0, 1.0];
    assert_eq!(
        ffi::preview(2, &p, &[1.0]),
        Err(shape(Routine::Preview, 2, 1))
    );
    assert_eq!(
        ffi::preview(2, &p[..3], &[1.0, 2.0]),
        Err(shape(Routine::Preview, 4, 3))
    );
    assert_eq!(
        ffi::review(2, &p, &[1.0, 2.0, 3.0]),
        Err(shape(Routine::Review, 2, 3))
    );
    assert_eq!(
        ffi::modify(2, &p, &[1.0, 2.0], &p[..1]),
        Err(shape(Routine::Modify, 4, 1))
    );
    assert_eq!(
        ffi::compose(2, &p, &p[..2]),
        Err(shape(Routine::Compose, 4, 2))
    );
    assert_eq!(
        ffi::phase_lock(&[0.0, 1.0], &[1.0], 1.0, 1, 0.1),
        Err(shape(Routine::PhaseLock, 2, 1))
    );
}

#[test]
fn phase_lock_refuses_what_its_c_int_would_wrap() {
    // usize::MAX steps as a C int is -1: zero steps, silently. Refused.
    assert_eq!(
        ffi::phase_lock(&[0.0], &[1.0], 1.0, usize::MAX, 0.1),
        Err(pre(Routine::PhaseLock, 5))
    );
    assert_eq!(
        ffi::phase_lock(&[], &[], 1.0, 1, 0.1),
        Err(pre(Routine::PhaseLock, 1))
    );
    // The first step count a C int can't hold, refused before a single step.
    assert_eq!(
        ffi::phase_lock(&[0.0], &[1.0], 1.0, PAST_INT, 0.1),
        Err(pre(Routine::PhaseLock, 5))
    );
    assert!(ffi::phase_lock(&[0.0], &[1.0], 1.0, 3, 0.1).is_ok());
}

#[test]
fn the_wrappers_still_answer_when_nothing_is_wrong() {
    let p = [1.0, 0.0, 0.0, 0.0];
    assert_eq!(ffi::review(2, &p, &[3.0, 4.0]), Ok(vec![3.0, 0.0]));
    assert_eq!(
        ffi::modify(2, &p, &[3.0, 4.0], &[2.0, 0.0, 0.0, 2.0]),
        Ok(vec![6.0, 4.0])
    );
    assert_eq!(ffi::singular_values(0, 3, &[]), Ok(vec![]));
    assert_eq!(ffi::svd(0, 0, &[]), Ok((vec![], vec![], vec![])));
    assert_eq!(ffi::eigensystem(0, &[]), Ok((vec![], vec![])));
}

#[test]
fn a_fault_is_an_imperfect_failure() {
    let measured: Imperfect<Vec<f64>, Fault, prismqueer::ScalarLoss> =
        ffi::eigenvalues(2, &[1.0]).into();
    assert!(matches!(
        measured,
        Imperfect::Failure(
            Fault::Shape {
                routine: Routine::Dsyev,
                expected: 4,
                found: 1
            },
            _
        )
    ));
}
