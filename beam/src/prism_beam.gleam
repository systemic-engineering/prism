//// Prism types for the BEAM.
//// Mirrors the Rust prism crate. Type surface equivalence.

import gleam/list
import gleam/option.{type Option, None, Some}

/// Content address. The identity of a thing is its content.
pub type Oid {
  Oid(value: String)
}

/// Shannon loss — what didn't survive the projection.
pub type ShannonLoss {
  ShannonLoss(bits: Float)
}

/// Spectral precision mask — the zoom level.
pub type Precision {
  Precision(value: Float)
}

/// Memory pressure — precision applied to storage.
pub type Pressure {
  Pressure(ratio: Float)
}

/// How a beam was recovered after a degraded projection.
pub type Recovery {
  Coarsened(from: Precision, to: Precision)
  Replayed(from_step: Int)
  Failed(reason: String)
}

/// The trace of a projection through a Prism.
/// Always lands. Loss tells you what didn't survive.
pub type Beam(t) {
  Beam(
    result: t,
    path: List(Oid),
    loss: ShannonLoss,
    precision: Precision,
    recovered: Option(Recovery),
  )
}

/// Create a beam with a result and no loss.
pub fn new(result: t) -> Beam(t) {
  Beam(
    result: result,
    path: [],
    loss: ShannonLoss(0.0),
    precision: Precision(0.0),
    recovered: None,
  )
}

/// Whether the projection was lossless.
pub fn is_lossless(beam: Beam(t)) -> Bool {
  beam.loss.bits == 0.0
}

/// Whether the projection lost information.
pub fn has_loss(beam: Beam(t)) -> Bool {
  beam.loss.bits >. 0.0
}

/// Whether recovery was attempted.
pub fn was_recovered(beam: Beam(t)) -> Bool {
  option.is_some(beam.recovered)
}

/// Map the result, preserving the trace.
pub fn map(beam: Beam(a), f: fn(a) -> b) -> Beam(b) {
  Beam(
    result: f(beam.result),
    path: beam.path,
    loss: beam.loss,
    precision: beam.precision,
    recovered: beam.recovered,
  )
}

/// Add a step to the path.
pub fn with_step(beam: Beam(t), oid: Oid) -> Beam(t) {
  Beam(..beam, path: list.append(beam.path, [oid]))
}

/// Set the loss.
pub fn with_loss(beam: Beam(t), loss: ShannonLoss) -> Beam(t) {
  Beam(..beam, loss: loss)
}

/// Set the precision.
pub fn with_precision(beam: Beam(t), precision: Precision) -> Beam(t) {
  Beam(..beam, precision: precision)
}

/// Set recovery.
pub fn with_recovery(beam: Beam(t), recovery: Recovery) -> Beam(t) {
  Beam(..beam, recovered: Some(recovery))
}
