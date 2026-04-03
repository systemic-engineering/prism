import gleeunit
import gleeunit/should
import prism_beam.{Oid, Precision, ShannonLoss}

pub fn main() {
  gleeunit.main()
}

pub fn new_beam_is_lossless_test() {
  let b = prism_beam.new(42)
  should.be_true(prism_beam.is_lossless(b))
  should.be_false(prism_beam.has_loss(b))
}

pub fn beam_with_loss_test() {
  let b =
    prism_beam.new(0)
    |> prism_beam.with_loss(ShannonLoss(1.5))
  should.be_true(prism_beam.has_loss(b))
  should.be_false(prism_beam.is_lossless(b))
}

pub fn beam_map_test() {
  let b =
    prism_beam.new(10)
    |> prism_beam.map(fn(x) { x * 2 })
  should.equal(b.result, 20)
}

pub fn beam_with_step_test() {
  let b =
    prism_beam.new("data")
    |> prism_beam.with_step(Oid("step1"))
  should.equal(b.path, [Oid("step1")])
}

pub fn beam_with_precision_test() {
  let b =
    prism_beam.new(1)
    |> prism_beam.with_precision(Precision(0.001))
  should.equal(b.precision, Precision(0.001))
}

pub fn beam_result_always_present_test() {
  let b = prism_beam.new("always here")
  should.equal(b.result, "always here")
}

pub fn beam_not_recovered_by_default_test() {
  let b = prism_beam.new(0)
  should.be_false(prism_beam.was_recovered(b))
}
