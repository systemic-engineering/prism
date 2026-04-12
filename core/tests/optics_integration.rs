//! Integration test: optics with the new Beam/Prism API.
//!
//! These tests verify that all six optic types (Iso, Lens, OpticPrism,
//! Traversal, Fold, Setter) work with the PureBeam-based Prism trait.
//! The old API (Beam struct, Stage, split, zoom) is gone.

#![cfg(feature = "optics")]

use prism_core::optics::fold::Fold;
use prism_core::optics::gather::{AddGather, ConcatGather, Gather};
use prism_core::optics::iso::Iso;
use prism_core::optics::lens::Lens;
use prism_core::optics::monoid::PrismMonoid;
use prism_core::optics::optic_prism::OpticPrism;
use prism_core::optics::setter::Setter;
use prism_core::optics::traversal::Traversal;
use prism_core::{Beam, Prism, PureBeam};
use std::convert::Infallible;
use terni::ShannonLoss;

fn seed<T: Clone>(v: T) -> PureBeam<(), T, Infallible, ShannonLoss> {
    PureBeam::ok((), v)
}

// --- Iso ---

fn str_to_chars(s: String) -> Vec<char> {
    s.chars().collect()
}
fn chars_to_str(v: Vec<char>) -> String {
    v.into_iter().collect()
}

#[test]
fn iso_full_pipeline_round_trips() {
    let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
    let beam = seed("hello".to_string());
    let focused = iso.focus(beam);
    let projected = iso.project(focused);
    let refracted = iso.refract(projected);
    assert_eq!(refracted.result().ok(), Some(&"hello".to_string()));
}

// --- Lens ---

#[derive(Clone, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn point_view_x(p: &Point) -> i32 {
    p.x
}
fn point_set_x(p: Point, new_x: i32) -> Point {
    Point { x: new_x, ..p }
}

#[test]
fn lens_pipeline_extracts_field() {
    let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
    let focused = x_lens.focus(seed(Point { x: 42, y: 7 }));
    let projected = x_lens.project(focused);
    let refracted = x_lens.refract(projected);
    assert_eq!(refracted.result().ok(), Some(&42));
}

// --- OpticPrism ---

#[derive(Clone, Debug, PartialEq)]
enum Shape {
    Circle(i32),
    Square(i32),
}

fn is_circle(s: &Shape) -> bool {
    matches!(s, Shape::Circle(_))
}
fn extract_circle(s: &Shape) -> i32 {
    if let Shape::Circle(r) = s {
        *r
    } else {
        -1
    }
}
fn review_circle(r: i32) -> Shape {
    Shape::Circle(r)
}

#[test]
fn optic_prism_nonmatch_carries_infinite_loss() {
    let p: OpticPrism<Shape, i32> = OpticPrism::new(is_circle, extract_circle, review_circle);
    let focused = p.focus(seed(Shape::Square(3)));
    assert!(focused.is_partial());
}

// --- Traversal ---

#[test]
fn traversal_with_gather_via_smap() {
    fn double(x: i32) -> i32 {
        x * 2
    }
    let t: Traversal<i32, i32> = Traversal::new(double);
    let focused = t.focus(seed(vec![1, 2, 3]));
    let gathered = focused.smap(|v| {
        let result = AddGather.gather(v.clone());
        terni::Imperfect::Success(result)
    });
    assert_eq!(gathered.result().ok(), Some(&12));
}

// --- Fold ---

#[derive(Clone)]
struct Tree {
    leaves: Vec<i32>,
}

fn tree_leaves(t: &Tree) -> Vec<i32> {
    t.leaves.clone()
}

#[test]
fn fold_extracts_and_gathers() {
    let f: Fold<Tree, i32> = Fold::new(tree_leaves);
    let focused = f.focus(seed(Tree {
        leaves: vec![10, 20, 30],
    }));
    let gathered = focused.smap(|v| terni::Imperfect::Success(AddGather.gather(v.clone())));
    assert_eq!(gathered.result().ok(), Some(&60));
}

// --- Setter ---

#[derive(Clone, Debug, PartialEq)]
struct Box2 {
    label: String,
    count: i32,
}

fn box2_modify_count(b: Box2, f: &dyn Fn(i32) -> i32) -> Box2 {
    Box2 {
        count: f(b.count),
        ..b
    }
}

#[test]
fn setter_pipeline_preserves_value() {
    let s: Setter<Box2, i32> = Setter::new(box2_modify_count);
    let b = Box2 {
        label: "test".to_string(),
        count: 5,
    };
    let focused = s.focus(seed(b.clone()));
    let projected = s.project(focused);
    let refracted = s.refract(projected);
    assert_eq!(refracted.result().ok(), Some(&b));
}

// --- Gather ---

#[test]
fn concat_gather_collapses_strings() {
    let result = ConcatGather.gather(vec![
        "hello".to_string(),
        " ".to_string(),
        "world".to_string(),
    ]);
    assert_eq!(result, "hello world");
}

// --- Monoid ---

#[test]
fn monoid_laws_hold() {
    use prism_core::optics::monoid::CountMonoid;
    let a = CountMonoid::new(1);
    let b = CountMonoid::new(2);
    let c = CountMonoid::new(3);
    let left = a.clone().compose(b.clone()).compose(c.clone());
    let right = a.compose(b.compose(c));
    assert_eq!(left.count(), right.count());
}

// --- Additional coverage: panic paths and partial propagation ---

fn wrap_success_i32(v: &i32) -> terni::Imperfect<i32, String, ShannonLoss> {
    terni::Imperfect::Success(*v)
}

#[test]
#[should_panic(expected = "smap on Err beam")]
fn smap_on_err_panics_in_integration() {
    let b: PureBeam<(), i32, String, ShannonLoss> = PureBeam::err((), "fail".into());
    let _ = b.smap(wrap_success_i32);
}

#[test]
fn smap_fn_ptr_executes_in_integration() {
    let b: PureBeam<(), i32, String, ShannonLoss> = PureBeam::ok((), 5);
    let n = b.smap(wrap_success_i32);
    assert_eq!(n.result().ok(), Some(&5));
}

#[test]
#[should_panic(expected = "tick on Err beam")]
fn tick_on_err_panics_in_integration() {
    let b: PureBeam<(), i32, String, ShannonLoss> = PureBeam::err((), "fail".into());
    let _ = b.next(99i32);
}

#[test]
fn tick_partial_to_partial_accumulates_loss_in_integration() {
    let b: PureBeam<(), i32, String, ShannonLoss> =
        PureBeam::partial((), 1i32, ShannonLoss::new(1.0));
    let n = b.tick(terni::Imperfect::<i32, String, ShannonLoss>::Partial(
        2,
        ShannonLoss::new(0.5),
    ));
    assert!(n.is_partial());
    assert_eq!(n.result().loss().as_f64(), 1.5);
}

#[test]
fn tick_partial_to_failure_in_integration() {
    let b: PureBeam<(), i32, String, ShannonLoss> =
        PureBeam::partial((), 1i32, ShannonLoss::new(1.0));
    let n = b.tick(terni::Imperfect::<i32, String, ShannonLoss>::Failure(
        "e".into(),
    ));
    assert!(n.is_err());
}

#[test]
fn optic_prism_review_covers_review_fn() {
    let p: OpticPrism<Shape, i32> = OpticPrism::new(is_circle, extract_circle, review_circle);
    // review_circle: i32 -> Shape
    let s = p.review(7);
    assert_eq!(s, Shape::Circle(7));
}

#[test]
fn optic_prism_matching_focus_calls_next() {
    // This test ensures PureBeam<(), Shape>::next::<i32> is called
    // (which happens in OpticPrism::focus on a matching shape)
    let p: OpticPrism<Shape, i32> = OpticPrism::new(is_circle, extract_circle, review_circle);
    let focused = p.focus(seed(Shape::Circle(42)));
    assert!(focused.is_ok());
    assert_eq!(focused.result().ok(), Some(&42i32));
}

#[test]
fn lens_set_covers_setter_fn() {
    let x_lens: Lens<Point, i32> = Lens::new(point_view_x, point_set_x);
    let p = Point { x: 1, y: 2 };
    let updated = x_lens.set(p, 99);
    assert_eq!(updated.x, 99);
    assert_eq!(updated.y, 2);
}

#[test]
fn count_monoid_identity_in_integration() {
    use prism_core::optics::monoid::{CountMonoid, PrismMonoid};
    let id = CountMonoid::identity();
    let a = CountMonoid::new(3);
    assert_eq!(id.compose(a.clone()).count(), a.count());
}

// --- Loss propagation ---

#[test]
fn loss_propagation_through_optic_pipeline() {
    let iso: Iso<String, Vec<char>> = Iso::new(str_to_chars, chars_to_str);
    let beam: PureBeam<(), String, Infallible, ShannonLoss> =
        PureBeam::partial((), "hi".to_string(), ShannonLoss::new(0.5));
    let focused = iso.focus(beam);
    assert!(focused.is_partial(), "loss must propagate through focus");
    let projected = iso.project(focused);
    assert!(
        projected.is_partial(),
        "loss must propagate through project"
    );
    let refracted = iso.refract(projected);
    assert!(
        refracted.is_partial(),
        "loss must propagate through refract"
    );
}
