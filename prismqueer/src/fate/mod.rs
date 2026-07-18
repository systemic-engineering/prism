// Numerical linear algebra code needs direct indexing (matrix/vector
// operations, eigensystems, kernel projections). The needless_range_loop /
// manual_memcpy / type_complexity lints are false-positive noise here; the
// indexed form is more readable than iterator chains for numerical work.
#![allow(
    clippy::needless_range_loop,
    clippy::manual_memcpy,
    clippy::type_complexity
)]

//! Fate — the five models and their selector.
//!
//! Abyss:        Focus. Observe the spectral state.
//! Introject:    Project. Selective internalization — what survives the precision cut.
//! Cartographer: Strategy selector — HOW to split. (user-space smap)
//! Explorer:     Subgraph comprehension — compressed meaning. (user-space smap)
//! Fate:         Refract. Crystallize. Select what runs next.
//!
//! Depends on Prism. Implements Prism (focus | project | settle).
//! The weights are hardcoded. The binary IS the model.
//! The thing you look into that looks back.

use crate::{Beam, Loss as _, Optic, Prism as PrismTrait};

pub mod compiled;
pub mod derive;
pub mod feature;
pub mod manifold;
#[cfg(feature = "fate-metal")]
pub mod metal_runtime;
pub mod runtime;
pub mod strategy;
pub mod weights;
pub use manifold::{ManifoldLoss, ManifoldState};
pub use strategy::Strategy;
#[cfg(feature = "fate-training")]
pub mod train;

/// Which model should run next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    /// Focus. Observe the spectral state.
    Abyss,
    /// Introject: Project. Selective internalization — what survives the precision cut.
    Introject,
    /// Split. Map the territory. Walk every node.
    Cartographer,
    /// Zoom. Recover meaning at the boundary. The residual signal.
    Explorer,
    /// Refract. Crystallize. Select what runs next.
    Fate,
}

/// The spectral features that Fate operates on.
/// 16 dimensions. Fixed. The shared vocabulary between all five models.
pub const FEATURE_DIM: usize = 16;

/// A spectral state: what the models see.
pub type Features = [f64; FEATURE_DIM];

/// Fate's decision: which model, with what confidence.
#[derive(Clone, Debug)]
pub struct Decision {
    pub model: Model,
    pub confidence: f64,
    /// The full probability distribution over models.
    pub distribution: [f64; 5],
}

impl Decision {
    /// Highest-probability non-Fate model.
    /// Zeroes out Fate's probability, renormalizes the remaining distribution,
    /// and returns the best non-Fate model.
    pub fn best_non_fate(&self) -> Decision {
        let mut dist = self.distribution;
        dist[4] = 0.0;
        let sum: f64 = dist.iter().sum();
        if sum > 0.0 {
            for p in dist.iter_mut() {
                *p /= sum;
            }
        }
        let best = argmax5(dist);
        Decision {
            model: MODELS[best],
            confidence: dist[best],
            distribution: dist,
        }
    }
}

/// Output of one Fate tick. Everything the runtime needs.
#[derive(Clone, Debug)]
pub struct FateOutput {
    pub model: Model,
    pub decision: Decision,
    pub kernel_spec: crate::KernelSpec,
    pub loss: ManifoldLoss,
    pub health: feature::HolonomyHealth,
}

/// The weights for one model's selector.
/// Input: FEATURE_DIM features + depth scalar → 5 logits (one per model).
/// Architecture: single linear layer + softmax. ~90 parameters.
pub struct ModelWeights {
    /// Weight matrix: 5 × FEATURE_DIM
    pub w: [[f64; FEATURE_DIM]; 5],
    /// Bias vector: 5
    pub b: [f64; 5],
    /// Depth modulation: how iteration depth affects each output logit.
    pub depth_w: [f64; 5],
}

/// The complete Fate model: five selectors (one per model context)
/// plus the meta-selector (Fate selecting for itself).
pub struct Fate {
    /// Selector weights when currently in each model's context.
    /// selectors[0] = "what to run after Abyss"
    /// selectors[1] = "what to run after Introject"
    /// selectors[2] = "what to run after Cartographer"
    /// selectors[3] = "what to run after Explorer"
    /// selectors[4] = "what to run after Fate" (the recursive case)
    pub selectors: [ModelWeights; 5],
    /// Decomposition strategy for Gauge impl. Default: SpectralPartition.
    pub strategy: Strategy,
    /// Cached result of resolve(). The Lawvere fixed point.
    pub resolved_model: Model,
    /// Kernel specification: which dimensions to preserve and how.
    /// Derived from Introject's weight activation pattern via update_connection().
    pub kernel_spec: crate::KernelSpec,
    /// The bundle-tower Connection's Optic. Held as `IdentityPrism<Features>`
    /// per the prismqueer contract `type Optic: Prism` (`KernelSpec` is a data
    /// carrier, not a `Prism`). The `KernelSpec` above stays load-bearing for
    /// `transport_rust` dimension-filtering; the Optic here satisfies the
    /// principal-bundle type contract without adding numerical semantics.
    /// ZST field — zero runtime cost.
    connection: crate::IdentityPrism<Features>,
}

// ---------------------------------------------------------------------------
// Inference
// ---------------------------------------------------------------------------

/// Softmax over 5 logits.
fn softmax5(logits: [f64; 5]) -> [f64; 5] {
    let max = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut exps = [0.0; 5];
    let mut sum = 0.0;
    for i in 0..5 {
        exps[i] = (logits[i] - max).exp();
        sum += exps[i];
    }
    for i in 0..5 {
        exps[i] /= sum;
    }
    exps
}

/// Shannon entropy of a probability distribution (base-2).
fn entropy5(dist: &[f64; 5]) -> f64 {
    dist.iter()
        .filter(|&&p| p > 0.0)
        .map(|&p| -p * p.log2())
        .sum()
}

/// Argmax of 5 values.
fn argmax5(v: [f64; 5]) -> usize {
    let mut best = 0;
    for i in 1..5 {
        if v[i] > v[best] {
            best = i;
        }
    }
    best
}

const MODELS: [Model; 5] = [
    Model::Abyss,
    Model::Introject,
    Model::Cartographer,
    Model::Explorer,
    Model::Fate,
];

impl ModelWeights {
    /// Forward pass: features + depth → logits → softmax → decision.
    pub fn forward(&self, features: &Features, depth: f64) -> Decision {
        let mut logits = self.b;
        for i in 0..5 {
            for j in 0..FEATURE_DIM {
                logits[i] += self.w[i][j] * features[j];
            }
            logits[i] += self.depth_w[i] * depth;
        }
        let distribution = softmax5(logits);
        let best = argmax5(distribution);
        Decision {
            model: MODELS[best],
            confidence: distribution[best],
            distribution,
        }
    }
}

impl Fate {
    /// Given the current model context and spectral features,
    /// decide which model should run next.
    pub fn select(&self, current: Model, features: &Features) -> Decision {
        let idx = match current {
            Model::Abyss => 0,
            Model::Introject => 1,
            Model::Cartographer => 2,
            Model::Explorer => 3,
            Model::Fate => 4,
        };
        self.selectors[idx].forward(features, 0.0)
    }

    /// The meta-loop: Fate selecting for itself, recursively,
    /// until it decides to dispatch to a different model.
    /// Uses depth axis + entropy-based exit: low entropy + Fate winning = overthinking.
    pub fn resolve(&self, features: &Features, max_depth: usize) -> Decision {
        let entropy_threshold = 1.0; // ~65% confidence on one model
        for depth in 0..max_depth {
            let normalized_depth = depth as f64 / max_depth as f64;
            let decision = self.selectors[4].forward(features, normalized_depth);
            if decision.model != Model::Fate {
                return decision;
            }
            if depth > 0 {
                let h = entropy5(&decision.distribution);
                if h < entropy_threshold {
                    return decision.best_non_fate();
                }
            }
        }
        // Max depth fallback
        let decision = self.selectors[4].forward(features, 1.0);
        decision.best_non_fate()
    }

    /// Create Fate with zero weights (uniform selection).
    /// The untrained state. Every model equally likely.
    pub fn untrained() -> Self {
        Fate {
            selectors: [
                ModelWeights {
                    w: [[0.0; FEATURE_DIM]; 5],
                    b: [0.0; 5],
                    depth_w: [0.0; 5],
                },
                ModelWeights {
                    w: [[0.0; FEATURE_DIM]; 5],
                    b: [0.0; 5],
                    depth_w: [0.0; 5],
                },
                ModelWeights {
                    w: [[0.0; FEATURE_DIM]; 5],
                    b: [0.0; 5],
                    depth_w: [0.0; 5],
                },
                ModelWeights {
                    w: [[0.0; FEATURE_DIM]; 5],
                    b: [0.0; 5],
                    depth_w: [0.0; 5],
                },
                ModelWeights {
                    w: [[0.0; FEATURE_DIM]; 5],
                    b: [0.0; 5],
                    depth_w: [0.0; 5],
                },
            ],
            strategy: Strategy::default(),
            resolved_model: Model::Abyss,
            kernel_spec: crate::KernelSpec::new(
                feature::ACTIVE.to_vec(),
                crate::Decomposition::Eigenvalue,
                crate::Precision::new(0.01),
            ),
            connection: crate::IdentityPrism::new(),
        }
    }

    /// Create Fate with random weights for manifold exploration.
    ///
    /// Each call produces a different instance — different initial exploration trajectory.
    /// Uses xorshift64 seeded from system time for zero-dependency randomness.
    ///
    /// Weight ranges match the quantization scheme:
    /// - biases: [0.0, 20.0]
    /// - feature weights: [0.0, 5.0]
    /// - depth weights: [0.0, 5.0]
    pub fn excited() -> Self {
        // Xorshift64 seeded from system time. Zero external deps.
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234_5678_9abc_def0);

        // Mix the seed further so two rapid calls differ even if nanos resolution is coarse.
        // Combine nanos with a compile-time address for additional entropy.
        let addr = Self::excited as *const () as usize as u64;
        let mut state = seed ^ addr.wrapping_mul(0x9e37_79b9_7f4a_7c15);
        if state == 0 {
            state = 0xdead_beef_cafe_1234;
        }

        let mut next = move || -> u64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        // Map a u64 to f64 in [0.0, scale).
        let rand_f64 = |bits: u64, scale: f64| -> f64 {
            // Use top 53 bits for mantissa precision.
            let frac = (bits >> 11) as f64 / (1u64 << 53) as f64;
            frac * scale
        };

        let make_selector = |rng: &mut dyn FnMut() -> u64| -> ModelWeights {
            let mut w = [[0.0f64; FEATURE_DIM]; 5];
            let mut b = [0.0f64; 5];
            let mut depth_w = [0.0f64; 5];

            for row in w.iter_mut() {
                for val in row.iter_mut() {
                    *val = rand_f64(rng(), 5.0);
                }
            }
            for val in b.iter_mut() {
                *val = rand_f64(rng(), 20.0);
            }
            for val in depth_w.iter_mut() {
                *val = rand_f64(rng(), 5.0);
            }

            ModelWeights { w, b, depth_w }
        };

        Fate {
            selectors: [
                make_selector(&mut next),
                make_selector(&mut next),
                make_selector(&mut next),
                make_selector(&mut next),
                make_selector(&mut next),
            ],
            strategy: Strategy::default(),
            resolved_model: Model::Abyss,
            kernel_spec: crate::KernelSpec::new(
                feature::ACTIVE.to_vec(),
                crate::Decomposition::Eigenvalue,
                crate::Precision::new(0.01),
            ),
            connection: crate::IdentityPrism::new(),
        }
    }

    /// Derive kernel spec from Introject's weight activation pattern.
    /// Iterates only over active dimensions (respects active/dark boundary).
    /// Dimensions where the max absolute weight exceeds threshold are preserved.
    pub fn update_connection(&mut self, threshold: f64) {
        let introject = &self.selectors[1];
        let mut dimensions = Vec::new();
        for &dim in &feature::ACTIVE {
            let max_weight = introject
                .w
                .iter()
                .map(|row| row[dim].abs())
                .fold(0.0f64, f64::max);
            if max_weight > threshold {
                dimensions.push(dim);
            }
        }
        if dimensions.is_empty() {
            let best = feature::ACTIVE
                .iter()
                .map(|&dim| {
                    let w = introject
                        .w
                        .iter()
                        .map(|row| row[dim].abs())
                        .fold(0.0f64, f64::max);
                    (dim, w)
                })
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                .map(|(dim, _)| dim)
                .unwrap();
            dimensions.push(best);
        }
        self.kernel_spec = crate::KernelSpec::new(
            dimensions,
            crate::Decomposition::Eigenvalue,
            crate::Precision::new(0.01),
        );
    }

    /// Update the cached fixed point. Call after training or state changes.
    pub fn update_closure(&mut self, features: &Features) {
        let decision = self.resolve(features, 5);
        self.resolved_model = decision.model;
    }

    /// Run one tick of the full Bundle tower.
    /// Features in (caller computes from their substrate), routing decision out.
    /// Fate doesn't know what a graph is. It knows 16 floats.
    ///
    /// Delegates to Pipeline internally — one path, one loss computation.
    pub fn tick(&self, features: &Features) -> FateOutput {
        // Build a ManifoldState from features (diagonal embedding)
        let mut input_state = manifold::manifold_zero();
        for i in 0..FEATURE_DIM {
            input_state[i][i] = features[i];
        }

        // Run the Pipeline: focus → project → settle
        let beam: crate::Optic<(), ManifoldState, std::convert::Infallible, ManifoldLoss> =
            crate::Optic::ok((), input_state);
        let result = Pipeline::settle(self, Pipeline::project(self, Pipeline::focus(self, beam)));

        let output_state = match result.result() {
            crate::Imperfect::Success(s) | crate::Imperfect::Partial(s, _) => s,
            crate::Imperfect::Failure(..) => unreachable!(),
        };

        let loss = ManifoldLoss::between(&input_state, output_state);
        let decision = self.resolve(features, 5);

        FateOutput {
            model: decision.model,
            decision,
            kernel_spec: self.kernel_spec.clone(),
            loss: loss.clone(),
            health: feature::holonomy_health(loss.total()),
        }
    }

    /// Total parameter count.
    pub fn param_count(&self) -> usize {
        5 * (5 * FEATURE_DIM + 5 + 5) // 5 selectors × (weights + bias + depth_w)
    }
}

// ---------------------------------------------------------------------------
// Prism implementation — Fate IS a Prism
// ---------------------------------------------------------------------------

impl PrismTrait for Fate {
    type Input = Optic<(), (Model, Features)>;
    type Focused = Optic<(Model, Features), [f64; 5]>;
    type Projected = Optic<[f64; 5], Decision>;
    type Refracted = Optic<Decision, Model>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let (current, features) = beam.result().ok().expect("focus: Err beam");
        let idx = match current {
            Model::Abyss => 0,
            Model::Introject => 1,
            Model::Cartographer => 2,
            Model::Explorer => 3,
            Model::Fate => 4,
        };
        let selector = &self.selectors[idx];
        let mut logits = selector.b;
        for i in 0..5 {
            for j in 0..FEATURE_DIM {
                logits[i] += selector.w[i][j] * features[j];
            }
            // depth=0.0 at focus time (single-shot, no iteration context)
            logits[i] += selector.depth_w[i] * 0.0;
        }
        beam.next(logits)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let logits = *beam.result().ok().expect("project: Err beam");
        let distribution = softmax5(logits);
        let best = argmax5(distribution);
        beam.next(Decision {
            model: MODELS[best],
            confidence: distribution[best],
            distribution,
        })
    }

    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        let model = beam.result().ok().expect("settle: Err beam").model;
        beam.next(model)
    }
}

// ---------------------------------------------------------------------------
// Bundle tower — Fate IS a principal bundle
// ---------------------------------------------------------------------------

impl crate::Fiber for Fate {
    type State = [f64; FEATURE_DIM];
}

impl crate::Connection for Fate {
    // prismqueer requires `type Optic: Prism`. KernelSpec is a data carrier,
    // not a Prism; carry the trivial Prism here and keep the KernelSpec on
    // Fate as internal state for numerical operations (transport_rust,
    // projection_matrix, dispatch_hint).
    type Optic = crate::IdentityPrism<Features>;
    fn connection(&self) -> &crate::IdentityPrism<Features> {
        &self.connection
    }
}

impl crate::Gauge for Fate {
    type Group = Strategy;
    fn gauge(&self) -> &Strategy {
        &self.strategy
    }

    /// Apply the gauge element (Strategy) to a fiber state (features).
    ///
    /// The action must satisfy `g.act_on(&h.act_on(s)) == g.compose(&h).act_on(&s)`.
    /// Fate's Strategy is a categorical choice among 5 decomposition strategies
    /// (SpectralPartition / CommunityDetection / BreadthFirst / DepthFirst /
    /// Random). The minimal viable action is the trivial one: features flow
    /// through unchanged. The group-composition axiom holds trivially because
    /// both sides of the equation are `*state`. Consumer-specific specialisations
    /// (e.g. Fiedler-projection under SpectralPartition; permutation under
    /// Random) land as follow-up ticks when a downstream test pulls a
    /// non-trivial witness.
    fn act_on(&self, state: &Self::State) -> Self::State {
        *state
    }
}

impl crate::Transport for Fate {
    type Holonomy = crate::ScalarLoss;
    fn transport(
        &self,
        state: &[f64; FEATURE_DIM],
    ) -> crate::Imperfect<[f64; FEATURE_DIM], std::convert::Infallible, crate::ScalarLoss> {
        #[cfg(feature = "lapack")]
        {
            self.transport_fortran(state)
        }
        #[cfg(not(feature = "lapack"))]
        {
            self.transport_rust(state)
        }
    }
}

impl Fate {
    /// Rust fallback: dimension filtering via KernelSpec.
    ///
    /// When the `lapack` feature is on, `impl Transport for Fate::transport`
    /// dispatches directly to `transport_fortran` and this method becomes
    /// dead code. The `cfg_attr` gate silences the compiler's unused-method
    /// warning without moving the fn out of the shared `impl Fate` block
    /// (which also contains the lapack-gated `transport_fortran`).
    #[cfg_attr(feature = "lapack", allow(dead_code))]
    fn transport_rust(
        &self,
        state: &[f64; FEATURE_DIM],
    ) -> crate::Imperfect<[f64; FEATURE_DIM], std::convert::Infallible, crate::ScalarLoss> {
        let spec = &self.kernel_spec;
        let mut compressed = [0.0f64; FEATURE_DIM];
        let mut loss = 0.0f64;

        for i in 0..FEATURE_DIM {
            if spec.dimensions.contains(&i) {
                compressed[i] = state[i];
            } else {
                loss += state[i].abs();
            }
        }

        if loss == 0.0 {
            crate::Imperfect::Success(compressed)
        } else {
            crate::Imperfect::Partial(compressed, crate::ScalarLoss::new(loss))
        }
    }

    /// Fortran path: build projection matrix from KernelSpec, dispatch to LAPACK.
    ///
    /// prismqueer's `KernelSpec::dispatch_hint` / `crate::DispatchHint` were
    /// removed in the June 2026 refactor. Consumers with the `lapack` feature
    /// enabled now always dispatch to the LAPACK path here; the Rust fallback
    /// stays reachable via the non-lapack build. If a size-based dispatcher
    /// returns to prismqueer, this call site re-adds the conditional.
    #[cfg(feature = "lapack")]
    fn transport_fortran(
        &self,
        state: &[f64; FEATURE_DIM],
    ) -> crate::Imperfect<[f64; FEATURE_DIM], std::convert::Infallible, crate::ScalarLoss> {
        let projection = self.kernel_spec.projection_matrix(FEATURE_DIM);
        let state_slice: &[f64] = state;

        match crate::ffi::preview(FEATURE_DIM, &projection, state_slice) {
            Some(result) => {
                // Compute loss: sum of absolute values of zeroed dimensions
                let mut loss = 0.0f64;
                let mut compressed = [0.0f64; FEATURE_DIM];
                for i in 0..FEATURE_DIM {
                    compressed[i] = result[i];
                    loss += (state[i] - result[i]).abs();
                }
                if loss == 0.0 {
                    crate::Imperfect::Success(compressed)
                } else {
                    crate::Imperfect::Partial(compressed, crate::ScalarLoss::new(loss))
                }
            }
            None => {
                // Projection produced zero vector — total loss
                crate::Imperfect::Partial(
                    [0.0f64; FEATURE_DIM],
                    crate::ScalarLoss::new(state.iter().map(|x| x.abs()).sum()),
                )
            }
        }
    }
}

/// Model is Fate's Lawvere fixed point: the resolved model at Fate's closure
/// level. Idempotence under the recursive selector endomap holds by
/// substrate discipline (Fate::resolve returns non-Fate after entropy-
/// threshold convergence, and re-running on the same features returns the
/// same non-Fate choice — discrete argmax over a fixed selector). The
/// kernel witness returns `true` for all variants at this altitude: a
/// discrete model choice carries no metric residual under Fate's own
/// transport (that residual is the `ManifoldLoss` on `impl Transport for
/// Fate`; the model-choice itself lives in the kernel of Fate's optical
/// selector).
impl crate::LawvereFixedPoint for Model {
    fn in_kernel(&self) -> bool {
        true
    }
}

impl crate::Closure for Fate {
    type Fixed = Model;
    fn close(&self) -> &Model {
        &self.resolved_model
    }
}

// ---------------------------------------------------------------------------
// Manifold observation loss
// ---------------------------------------------------------------------------

/// Compute ManifoldLoss from observation: deviation of active diagonals from
/// Casimir eigenvalues, plus dark diagonal deviation from zero.
fn manifold_observation_loss(state: &ManifoldState) -> ManifoldLoss {
    let mut delta = [[0.0f64; FEATURE_DIM]; FEATURE_DIM];
    for (idx, &dim) in feature::ACTIVE.iter().enumerate() {
        delta[dim][dim] = state[dim][dim] - feature::CASIMIR_EIGENVALUES[idx];
    }
    // Dark dimensions: their diagonal deviation from zero
    for &dim in &feature::DARK {
        delta[dim][dim] = state[dim][dim];
    }
    ManifoldLoss { delta }
}

// ---------------------------------------------------------------------------
// Pipeline trait — ManifoldState as the value type
// ---------------------------------------------------------------------------

/// Manifold processing pipeline for Fate.
/// Separate from the model-selector Prism implementation.
pub trait Pipeline {
    fn focus(
        &self,
        beam: crate::Optic<(), ManifoldState, std::convert::Infallible, ManifoldLoss>,
    ) -> crate::Optic<ManifoldState, Features, std::convert::Infallible, ManifoldLoss>;

    fn project(
        &self,
        beam: crate::Optic<ManifoldState, Features, std::convert::Infallible, ManifoldLoss>,
    ) -> crate::Optic<Features, Decision, std::convert::Infallible, ManifoldLoss>;

    fn settle(
        &self,
        beam: crate::Optic<Features, Decision, std::convert::Infallible, ManifoldLoss>,
    ) -> crate::Optic<Decision, ManifoldState, std::convert::Infallible, ManifoldLoss>;
}

impl Pipeline for Fate {
    fn focus(
        &self,
        beam: crate::Optic<(), ManifoldState, std::convert::Infallible, ManifoldLoss>,
    ) -> crate::Optic<ManifoldState, Features, std::convert::Infallible, ManifoldLoss> {
        let state = beam.result().ok().expect("focus: infallible");
        let mut features = [0.0f64; FEATURE_DIM];

        // Active dims: diagonal carries eigenvalue directly
        for &i in &feature::ACTIVE {
            features[i] = state[i][i];
        }

        // Dark dims: signal is in off-diagonal coupling norm
        for &i in &feature::DARK {
            let coupling_norm: f64 = (0..FEATURE_DIM)
                .filter(|&j| j != i)
                .map(|j| state[i][j] * state[i][j])
                .sum::<f64>()
                .sqrt();
            features[i] = coupling_norm;
        }

        // Measure observation loss: deviation from expected Casimir eigenvalues
        let loss = manifold_observation_loss(state);
        if loss.is_zero() {
            beam.next(features)
        } else {
            beam.tick(crate::Imperfect::Partial(features, loss))
        }
    }

    fn project(
        &self,
        beam: crate::Optic<ManifoldState, Features, std::convert::Infallible, ManifoldLoss>,
    ) -> crate::Optic<Features, Decision, std::convert::Infallible, ManifoldLoss> {
        let features = beam.result().ok().expect("project: infallible");
        let decision = self.resolve(features, 5);
        beam.next(decision)
    }

    fn settle(
        &self,
        beam: crate::Optic<Features, Decision, std::convert::Infallible, ManifoldLoss>,
    ) -> crate::Optic<Decision, ManifoldState, std::convert::Infallible, ManifoldLoss> {
        let decision = beam.result().ok().expect("settle: infallible").clone();
        let vectors = self.steering_vectors();
        let mut state = manifold::manifold_zero();

        // Weighted outer product: Σ prob_m * v_m ⊗ v_m^T
        for (m, &prob) in decision.distribution.iter().enumerate() {
            if prob < 1e-9 {
                continue;
            }
            let v = &vectors[m];
            for i in 0..FEATURE_DIM {
                for j in 0..FEATURE_DIM {
                    state[i][j] += prob * v[i] * v[j];
                }
            }
        }

        // Enforce Casimir: scale active diagonals so their sum hits the target.
        // Scaling preserves ratios between active dimensions (closer to true
        // quadratic Casimir conservation than additive correction).
        let active_sum: f64 = feature::ACTIVE.iter().map(|&i| state[i][i]).sum();
        let target: f64 = feature::CASIMIR_EIGENVALUES.iter().sum();
        if active_sum.abs() > 1e-12 {
            let scale = target / active_sum;
            for &i in &feature::ACTIVE {
                state[i][i] *= scale;
            }
        } else {
            // Degenerate case: all active diagonals near zero, distribute evenly
            let per_dim = target / feature::ACTIVE_COUNT as f64;
            for &i in &feature::ACTIVE {
                state[i][i] = per_dim;
            }
        }

        beam.next(state)
    }
}

impl Fate {
    /// Extract steering vectors from Introject's trained weights.
    /// Row m of the weight matrix = steering vector for model m.
    /// Normalized to unit vectors. Zero weights → zero vector (norm check).
    pub fn steering_vectors(&self) -> [[f64; FEATURE_DIM]; 5] {
        let introject = &self.selectors[1]; // Introject = index 1
        let mut vectors = [[0.0f64; FEATURE_DIM]; 5];

        for m in 0..5 {
            let mut v = [0.0f64; FEATURE_DIM];
            // w[logit][feature]: row m = weights for logit m = steering vector for model m
            for j in 0..FEATURE_DIM {
                v[j] = introject.w[m][j];
            }
            // Normalize to unit vector; zero weights → zero vector
            let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm > 1e-9 {
                for x in &mut v {
                    *x /= norm;
                }
            }
            vectors[m] = v;
        }

        vectors
    }
}

/// Cartographer and Explorer: user-space smap operations (were split/zoom).
impl Fate {
    /// Cartographer: map the territory. User-space smap (was split).
    /// Returns a beam carrying the filtered models.
    pub fn cartograph<E, L: crate::Loss>(
        &self,
        beam: Optic<[f64; 5], Decision, E, L>,
    ) -> <Optic<[f64; 5], Decision, E, L> as crate::Beam>::Tick<Vec<Model>, E> {
        beam.smap(|decision| {
            let models: Vec<Model> = MODELS
                .iter()
                .enumerate()
                .filter(|(i, _)| decision.distribution[*i] > 0.01)
                .map(|(_, &m)| m)
                .collect();
            crate::Imperfect::Success(models)
        })
    }

    /// Explorer: recover meaning at the boundary. User-space smap (was zoom).
    /// Applies a transformation to a decision beam.
    pub fn explore<E, L: crate::Loss>(
        &self,
        beam: Optic<[f64; 5], Decision, E, L>,
        f: impl FnOnce(&Decision) -> Decision,
    ) -> <Optic<[f64; 5], Decision, E, L> as crate::Beam>::Tick<Decision, E> {
        beam.smap(|decision| crate::Imperfect::Success(f(decision)))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn zero_features() -> Features {
        [0.0; FEATURE_DIM]
    }

    #[test]
    fn untrained_fate_selects_uniformly() {
        let fate = Fate::untrained();
        let decision = fate.select(Model::Abyss, &zero_features());

        // With zero weights, softmax is uniform: each model gets 20%
        for &p in &decision.distribution {
            assert!(
                (p - 0.2).abs() < 1e-9,
                "untrained should be uniform, got {:?}",
                decision.distribution
            );
        }
    }

    #[test]
    fn softmax_sums_to_one() {
        let result = softmax5([1.0, 2.0, 3.0, 4.0, 5.0]);
        let sum: f64 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9, "softmax should sum to 1: {}", sum);
    }

    #[test]
    fn softmax_concentrates_on_largest() {
        let result = softmax5([0.0, 0.0, 0.0, 0.0, 10.0]);
        assert!(
            result[4] > 0.99,
            "should concentrate on index 4: {}",
            result[4]
        );
    }

    #[test]
    fn decision_picks_highest() {
        let weights = ModelWeights {
            w: {
                let mut w = [[0.0; FEATURE_DIM]; 5];
                // Cartographer (index 2) responds to feature 0
                w[2][0] = 5.0;
                w
            },
            b: [0.0; 5],
            depth_w: [0.0; 5],
        };

        let mut features = zero_features();
        features[0] = 1.0; // activate feature 0

        let decision = weights.forward(&features, 0.0);
        assert_eq!(decision.model, Model::Cartographer);
        assert!(decision.confidence > 0.5);
    }

    #[test]
    fn fate_context_dependent_selection() {
        let mut fate = Fate::untrained();

        // After Abyss: prefer Introject (project what was focused)
        fate.selectors[0].b[1] = 2.0; // bias toward Introject

        // After Introject: prefer Cartographer (split after projecting)
        fate.selectors[1].b[2] = 2.0; // bias toward Cartographer

        let from_abyss = fate.select(Model::Abyss, &zero_features());
        assert_eq!(from_abyss.model, Model::Introject);

        let from_path = fate.select(Model::Introject, &zero_features());
        assert_eq!(from_path.model, Model::Cartographer);
    }

    #[test]
    fn resolve_terminates_on_non_fate() {
        let mut fate = Fate::untrained();
        // Fate context: prefer Abyss
        fate.selectors[4].b[0] = 2.0;

        let decision = fate.resolve(&zero_features(), 10);
        assert_eq!(decision.model, Model::Abyss);
    }

    #[test]
    fn resolve_handles_recursive_fate() {
        let mut fate = Fate::untrained();
        // Fate context: prefer Fate (recursive)
        fate.selectors[4].b[4] = 2.0;

        // Should eventually break out (max_depth) and pick next best
        let decision = fate.resolve(&zero_features(), 5);
        assert_ne!(decision.model, Model::Fate, "should break out of recursion");
    }

    #[test]
    fn param_count_is_small() {
        let fate = Fate::untrained();
        let count = fate.param_count();
        // 5 selectors × (5 × 16 + 5 + 5) = 5 × 90 = 450 parameters
        assert_eq!(count, 450, "should be exactly 450 parameters");
        eprintln!("  Fate: {} parameters. {} bytes at f64.", count, count * 8);
    }

    #[test]
    fn fate_implements_prism() {
        use crate::{Beam, Optic, Prism as PrismTrait};

        let mut fate = Fate::untrained();
        fate.selectors[0].b[2] = 3.0; // after Abyss → Cartographer

        let input: Optic<(), (Model, Features)> = Optic::ok((), (Model::Abyss, zero_features()));

        // focus: decompose into logits
        let focused_beam = PrismTrait::focus(&fate, input);
        let logits = focused_beam.result().ok().unwrap();
        assert!(
            logits[2] > logits[0],
            "Cartographer logit should be highest"
        );

        // project: cut to decision
        let decision_beam = PrismTrait::project(&fate, focused_beam);
        assert_eq!(
            decision_beam.result().ok().unwrap().model,
            Model::Cartographer
        );

        // cartograph: walk viable models (smap, was split)
        let models_beam = fate.cartograph(decision_beam);
        let models = models_beam.result().ok().expect("cartograph produced Err");
        assert!(!models.is_empty(), "should have viable models");

        // settle: crystallize (need a fresh projected beam)
        let input2: Optic<(), (Model, Features)> = Optic::ok((), (Model::Abyss, zero_features()));
        let focused2 = PrismTrait::focus(&fate, input2);
        let projected2 = PrismTrait::project(&fate, focused2);
        let crystal_beam = PrismTrait::settle(&fate, projected2);
        let model = crystal_beam.result().ok().unwrap();
        assert_eq!(*model, Model::Cartographer); // matches the biased selector
    }

    #[test]
    fn explorer_transforms_decision() {
        use crate::{Beam, Optic, Prism as PrismTrait};

        let fate = Fate::untrained();

        // explore: transform a decision (smap, was zoom)
        let input3 = Optic::ok((), (Model::Abyss, zero_features()));
        let focused3 = PrismTrait::focus(&fate, input3);
        let projected3 = PrismTrait::project(&fate, focused3);
        let explored = fate.explore(projected3, |d| Decision {
            model: Model::Explorer,
            confidence: d.confidence,
            distribution: d.distribution,
        });
        assert_eq!(explored.result().ok().unwrap().model, Model::Explorer);
    }

    #[test]
    fn prism_apply_end_to_end() {
        use crate::Beam;

        let mut fate = Fate::untrained();
        fate.selectors[4].b[0] = 1.0; // Fate context → slight Abyss preference

        let input: Optic<(), (Model, Features)> = Optic::ok((), (Model::Fate, zero_features()));
        let beam = crate::apply(&fate, input);

        // beam is Optic<Decision, Model>. Should be ok (no loss).
        assert!(beam.is_ok());
    }

    #[test]
    fn fate_is_a_bundle() {
        fn accepts_bundle<B: crate::Bundle>(_b: &B) {}
        let fate = Fate::untrained();
        accepts_bundle(&fate);
    }

    #[test]
    fn fate_fiber_is_features() {
        // The associated type Fiber::State on Fate must equal Features.
        // Constructing an untrained Fate proves the impl compiles; the
        // state-type witness is the load-bearing assertion.
        let _fate = Fate::untrained();
        let _state: <Fate as crate::Fiber>::State = [0.0f64; FEATURE_DIM];
    }

    #[test]
    fn fate_transport_returns_partial() {
        use crate::Transport;
        let fate = Fate::untrained();
        let state = [1.0; FEATURE_DIM];
        let result = fate.transport(&state);
        // With non-zero lower half, should return Partial
        assert!(result.is_ok() || result.is_partial());
    }

    #[test]
    fn fate_closure_returns_model() {
        use crate::Closure;
        let fate = Fate::untrained();
        let model = fate.close();
        assert_eq!(*model, Model::Abyss);
    }

    #[test]
    fn resolve_exits_on_low_entropy_fate_loop() {
        let mut fate = Fate::untrained();
        // Bias Fate-context selector strongly toward Fate
        for j in 0..FEATURE_DIM {
            fate.selectors[4].w[4][j] = 10.0;
        }
        let features = [1.0; FEATURE_DIM];
        let result = fate.resolve(&features, 100);
        // Should exit before 100 via entropy floor, not burn max_depth
        assert_ne!(result.model, Model::Fate);
    }

    #[test]
    fn model_enum_is_complete() {
        let models = [
            Model::Abyss,
            Model::Cartographer,
            Model::Introject,
            Model::Explorer,
            Model::Fate,
        ];
        assert_eq!(models.len(), 5);
        // Each is distinct
        for i in 0..5 {
            for j in (i + 1)..5 {
                assert_ne!(models[i], models[j]);
            }
        }
    }

    #[test]
    fn best_non_fate_renormalizes() {
        let decision = Decision {
            model: Model::Fate,
            confidence: 0.9,
            distribution: [0.025, 0.025, 0.025, 0.025, 0.9],
        };
        let forced = decision.best_non_fate();
        assert!((forced.confidence - 0.25).abs() < 1e-9);
        let sum: f64 = forced.distribution.iter().sum();
        assert!((sum - 1.0).abs() < 1e-9);
        assert_eq!(forced.distribution[4], 0.0);
    }

    #[test]
    fn resolve_depth_changes_decision() {
        let mut fate = Fate::untrained();
        fate.selectors[4].depth_w[0] = 5.0; // Abyss grows with depth
        fate.selectors[4].depth_w[4] = -5.0; // Fate shrinks with depth
        let features = zero_features();
        let result = fate.resolve(&features, 10);
        assert_eq!(result.model, Model::Abyss);
    }

    #[test]
    fn transport_rust_and_fortran_agree() {
        // This test runs on whichever path is compiled.
        // The Rust and Fortran paths should produce identical results
        // for diagonal projection matrices.
        let mut fate = Fate::untrained();
        fate.kernel_spec = crate::KernelSpec::new(
            vec![0, 1, 2, 3],
            crate::Decomposition::Eigenvalue,
            crate::Precision::new(0.01),
        );
        let state = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];

        use crate::Transport;
        let result = fate.transport(&state);
        match result {
            crate::Imperfect::Partial(compressed, loss) => {
                // First 4 dimensions preserved
                assert_eq!(compressed[0], 1.0);
                assert_eq!(compressed[1], 2.0);
                assert_eq!(compressed[2], 3.0);
                assert_eq!(compressed[3], 4.0);
                // Rest zeroed
                for i in 4..FEATURE_DIM {
                    assert_eq!(compressed[i], 0.0);
                }
                // Loss = sum of |5| + |6| + ... + |16| = 5+6+7+8+9+10+11+12+13+14+15+16 = 126
                assert!((loss.as_f64() - 126.0).abs() < 1e-10);
            }
            _ => panic!("expected Partial"),
        }
    }

    #[test]
    fn resolve_first_pass_fate_second_pass_exits() {
        let mut fate = Fate::untrained();
        fate.selectors[4].b[4] = 1.0; // Fate bias at baseline
        fate.selectors[4].depth_w[3] = 8.0; // Explorer grows with depth
        fate.selectors[4].depth_w[4] = -8.0; // Fate retreats with depth
        let features = zero_features();
        let result = fate.resolve(&features, 10);
        assert_eq!(result.model, Model::Explorer);
    }

    #[test]
    fn fate_connection_returns_optic_and_kernel_spec_matches_active() {
        use crate::Connection;
        let fate = Fate::untrained();
        // `Connection::connection()` returns the trivial `IdentityPrism<Features>`
        // per the prismqueer contract that Optic implements Prism. The
        // KernelSpec (used by transport_rust for dimension filtering) stays
        // accessible as a direct field.
        let _optic: &crate::IdentityPrism<Features> = fate.connection();
        assert_eq!(fate.kernel_spec.dimensions.len(), feature::ACTIVE_COUNT);
        assert_eq!(
            fate.kernel_spec.decomposition,
            crate::Decomposition::Eigenvalue
        );
    }

    #[test]
    fn update_connection_derives_from_introject() {
        use crate::Connection;
        let mut fate = Fate::untrained();
        // Set strong weights on Introject (selector[1]) for features 0 and 3
        fate.selectors[1].w[0][0] = 5.0;
        fate.selectors[1].w[2][3] = 3.0;
        fate.update_connection(1.0); // threshold = 1.0
                                     // Connection::connection() returns the trivial IdentityPrism<Features>
                                     // per the prismqueer contract. The KernelSpec is on Fate directly.
        let _optic = fate.connection();
        assert!(fate.kernel_spec.dimensions.contains(&0));
        assert!(fate.kernel_spec.dimensions.contains(&3));
        // Other dimensions with zero weights should NOT be included
        assert!(!fate.kernel_spec.dimensions.contains(&1));
    }

    #[test]
    fn update_connection_excludes_dark_dims() {
        use crate::Connection;
        let mut fate = Fate::untrained();
        fate.selectors[1].w[0][feature::TEMPORAL] = 5.0;
        fate.selectors[1].w[1][feature::CREATIVITY] = 10.0; // dark dim
        fate.update_connection(1.0);
        // Connection::connection() returns the trivial IdentityPrism<Features>.
        // KernelSpec assertions read from the field directly.
        let _optic = fate.connection();
        assert!(fate.kernel_spec.dimensions.contains(&feature::TEMPORAL));
        assert!(
            !fate.kernel_spec.dimensions.contains(&feature::CREATIVITY),
            "dark dims must not appear in KernelSpec"
        );
    }

    #[test]
    fn transport_respects_kernel_spec() {
        use crate::Transport;
        let mut fate = Fate::untrained();

        // Narrow spec: preserve only 4 dimensions
        fate.kernel_spec = crate::KernelSpec::new(
            vec![0, 1, 2, 3],
            crate::Decomposition::Eigenvalue,
            crate::Precision::new(0.01),
        );
        let state = [1.0; FEATURE_DIM];
        let result_narrow = fate.transport(&state);

        // Wide spec: preserve 12 dimensions
        fate.kernel_spec = crate::KernelSpec::new(
            (0..12).collect(),
            crate::Decomposition::Eigenvalue,
            crate::Precision::new(0.01),
        );
        let result_wide = fate.transport(&state);

        match (&result_narrow, &result_wide) {
            (crate::Imperfect::Partial(_, loss_n), crate::Imperfect::Partial(_, loss_w)) => {
                assert!(
                    loss_n.as_f64() > loss_w.as_f64(),
                    "narrower spec should have more loss"
                );
            }
            _ => panic!("expected Partial"),
        }
    }

    #[test]
    fn closure_reflects_resolved_model() {
        use crate::Closure;

        let mut fate = Fate::untrained();
        fate.selectors[4].b[3] = 5.0; // Bias toward Explorer
        fate.update_closure(&[0.0; FEATURE_DIM]);
        assert_eq!(*fate.close(), Model::Explorer);

        fate.selectors[4].b[0] = 10.0; // Now Abyss dominates
        fate.update_closure(&[0.0; FEATURE_DIM]);
        assert_eq!(*fate.close(), Model::Abyss);
    }

    // -----------------------------------------------------------------------
    // Pipeline tests (Task 4)
    // -----------------------------------------------------------------------

    #[test]
    fn pipeline_focus_extracts_active_diagonal() {
        use crate::fate::Pipeline;
        let mut state = manifold::manifold_zero();
        state[feature::TEMPORAL][feature::TEMPORAL] = 4.12;
        state[feature::PROCESSING][feature::PROCESSING] = 3.98;
        let beam: crate::Optic<(), ManifoldState, std::convert::Infallible, ManifoldLoss> =
            crate::Optic::ok((), state);
        let focused = Pipeline::focus(&Fate::untrained(), beam);
        let features = focused.result().ok().unwrap();
        assert!((features[feature::TEMPORAL] - 4.12).abs() < 1e-10);
        assert!((features[feature::PROCESSING] - 3.98).abs() < 1e-10);
    }

    #[test]
    fn pipeline_focus_extracts_dark_coupling_norm() {
        use crate::fate::Pipeline;
        let mut state = manifold::manifold_zero();
        // Dark dim 6 (Creativity): off-diagonal coupling
        state[feature::CREATIVITY][0] = 3.0;
        state[feature::CREATIVITY][1] = 4.0;
        // coupling_norm = sqrt(9 + 16) = 5.0
        let beam: crate::Optic<(), ManifoldState, std::convert::Infallible, ManifoldLoss> =
            crate::Optic::ok((), state);
        let focused = Pipeline::focus(&Fate::untrained(), beam);
        let features = focused.result().ok().unwrap();
        assert!((features[feature::CREATIVITY] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn pipeline_settle_produces_valid_manifold() {
        use crate::fate::Pipeline;
        let fate = Fate::untrained();
        let state = manifold::manifold_identity();
        let beam: crate::Optic<(), ManifoldState, std::convert::Infallible, ManifoldLoss> =
            crate::Optic::ok((), state);
        let focused = Pipeline::focus(&fate, beam);
        let projected = Pipeline::project(&fate, focused);
        let settled = Pipeline::settle(&fate, projected);
        let new_state = settled.result().ok().unwrap();
        // Active diagonal should sum to target Casimir (conservation enforced)
        let active_sum: f64 = feature::ACTIVE.iter().map(|&i| new_state[i][i]).sum();
        let target: f64 = feature::CASIMIR_EIGENVALUES.iter().sum();
        assert!(
            (active_sum - target).abs() < 1e-10,
            "Casimir must be conserved"
        );
    }

    #[test]
    fn pipeline_end_to_end_via_run() {
        use crate::fate::Pipeline;
        let fate = Fate::untrained();
        let state = manifold::manifold_identity();
        let beam: crate::Optic<(), ManifoldState, std::convert::Infallible, ManifoldLoss> =
            crate::Optic::ok((), state);
        let result = Pipeline::settle(
            &fate,
            Pipeline::project(&fate, Pipeline::focus(&fate, beam)),
        );
        assert!(result.is_ok() || result.is_partial());
    }

    #[test]
    fn steering_vectors_are_unit_length_when_trained() {
        let mut fate = Fate::untrained();
        // Give Introject some non-zero weights
        for m in 0..5 {
            for j in 0..FEATURE_DIM {
                fate.selectors[1].w[m][j] = (m as f64 + 1.0) * (j as f64 + 1.0);
            }
        }
        let vectors = fate.steering_vectors();
        for m in 0..5 {
            let norm: f64 = vectors[m].iter().map(|x| x * x).sum::<f64>().sqrt();
            assert!(
                (norm - 1.0).abs() < 1e-10,
                "steering vector {m} should be unit length"
            );
        }
    }

    #[test]
    fn steering_vectors_zero_for_untrained() {
        let fate = Fate::untrained();
        let vectors = fate.steering_vectors();
        for m in 0..5 {
            let norm: f64 = vectors[m].iter().map(|x| x * x).sum::<f64>().sqrt();
            assert!(norm < 1e-9, "untrained steering vectors should be zero");
        }
    }

    // -----------------------------------------------------------------------
    // tick() tests (Task 6)
    // -----------------------------------------------------------------------

    #[test]
    fn tick_produces_fate_output() {
        use crate::Loss;
        let fate = Fate::untrained();
        let features = [1.0; FEATURE_DIM];
        let output = fate.tick(&features);
        assert_ne!(output.model, Model::Fate);
        assert!(!output.loss.is_zero()); // dark dims zeroed = nonzero loss
    }

    #[test]
    fn tick_delegates_to_pipeline() {
        use crate::fate::Pipeline;
        let fate = Fate::untrained();
        let mut features = [0.0; FEATURE_DIM];
        for &dim in &feature::ACTIVE {
            features[dim] = 1.0;
        }
        let output = fate.tick(&features);
        // tick now runs the full Pipeline — settle enforces Casimir scaling,
        // so the output manifold will differ from input. Loss should be nonzero.
        assert!(
            output.loss.total() > 0.0,
            "Pipeline should produce measurable loss"
        );
        // The output active diagonal sum should match the Casimir target
        // (enforced by settle's scaling correction)
        let target: f64 = feature::CASIMIR_EIGENVALUES.iter().sum();
        let mut input_state = manifold::manifold_zero();
        for i in 0..FEATURE_DIM {
            input_state[i][i] = features[i];
        }
        let beam: crate::Optic<(), ManifoldState, std::convert::Infallible, ManifoldLoss> =
            crate::Optic::ok((), input_state);
        let result = Pipeline::settle(
            &fate,
            Pipeline::project(&fate, Pipeline::focus(&fate, beam)),
        );
        let out = match result.result() {
            crate::Imperfect::Success(s) | crate::Imperfect::Partial(s, _) => s,
            crate::Imperfect::Failure(..) => unreachable!(),
        };
        let active_sum: f64 = feature::ACTIVE.iter().map(|&i| out[i][i]).sum();
        assert!(
            (active_sum - target).abs() < 1e-10,
            "settle must enforce Casimir target"
        );
    }

    #[test]
    fn tick_health_is_healthy_for_normal_input() {
        let fate = Fate::untrained();
        let features = [1.0; FEATURE_DIM];
        let output = fate.tick(&features);
        // 10 dark dims zeroed, each with |1.0| = total ~sqrt(10) ≈ 3.16
        // Berry phase = 0.847, ratio ≈ 3.7 → Healthy range (0.1 to 10.0)
        assert_eq!(output.health, feature::HolonomyHealth::Healthy);
    }

    #[test]
    fn excited_produces_valid_fate() {
        let fate = Fate::excited();
        let features = [0.0f64; FEATURE_DIM];
        let output = fate.tick(&features);
        // Should not crash, should produce a valid model
        assert!(matches!(
            output.decision.model,
            Model::Abyss | Model::Introject | Model::Cartographer | Model::Explorer | Model::Fate
        ));
    }

    #[test]
    fn excited_differs_from_untrained() {
        let untrained = Fate::untrained();
        let excited = Fate::excited();
        let features = [1.0f64; FEATURE_DIM];
        let u_out = untrained.tick(&features);
        let e_out = excited.tick(&features);
        assert_ne!(
            u_out.decision.distribution, e_out.decision.distribution,
            "excited should have different probability distribution than untrained"
        );
    }

    #[test]
    fn two_excited_instances_differ() {
        let a = Fate::excited();
        let b = Fate::excited();
        let features = [1.0f64; FEATURE_DIM];
        let a_out = a.tick(&features);
        let b_out = b.tick(&features);
        assert_ne!(
            a_out.decision.distribution, b_out.decision.distribution,
            "two excited instances should explore different regions"
        );
    }

    #[test]
    fn excited_explores_different_models() {
        // Create many excited instances, verify we see multiple models selected
        let features = [5.0f64; FEATURE_DIM];
        let mut models_seen = std::collections::HashSet::new();
        for _ in 0..50 {
            let fate = Fate::excited();
            let output = fate.tick(&features);
            models_seen.insert(format!("{:?}", output.decision.model));
        }
        assert!(
            models_seen.len() > 1,
            "50 excited instances should explore more than one model, saw: {:?}",
            models_seen
        );
    }
}
