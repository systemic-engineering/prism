//! FateRuntime — Fate's inference engine.
//!
//! Under the hood: a Brainfuck interpreter.
//! Because of course it does.
//!
//! The 816 characters of fate.bf ARE the model.
//! The interpreter IS the runtime.
//! The tape IS the spectral state.
//! Halting IS the crystal.

use crate::fate::{Features, Model};
use crate::{Beam, Optic, Prism as PrismTrait};

// ---------------------------------------------------------------------------
// The Brainfuck interpreter — the runtime beneath all runtimes
// ---------------------------------------------------------------------------

/// Execute a Brainfuck program with given input bytes.
/// Returns the output bytes.
fn bf_execute(program: &[u8], input: &[u8]) -> Vec<u8> {
    let mut tape = vec![0u8; 256];
    let mut dp: usize = 0; // data pointer
    let mut ip: usize = 0; // instruction pointer
    let mut inp: usize = 0; // input pointer
    let mut output = Vec::new();

    // Precompute bracket matching
    let mut bracket_map = vec![0usize; program.len()];
    let mut stack = Vec::new();
    for i in 0..program.len() {
        if program[i] == b'[' {
            stack.push(i);
        } else if program[i] == b']' {
            if let Some(j) = stack.pop() {
                bracket_map[i] = j;
                bracket_map[j] = i;
            }
        }
    }

    let mut steps = 0usize;
    let max_steps = 1_000_000;

    while ip < program.len() && steps < max_steps {
        match program[ip] {
            b'>' => {
                dp = (dp + 1).min(255);
            }
            b'<' => {
                dp = dp.saturating_sub(1);
            }
            b'+' => tape[dp] = tape[dp].wrapping_add(1),
            b'-' => tape[dp] = tape[dp].wrapping_sub(1),
            b'.' => output.push(tape[dp]),
            b',' => {
                tape[dp] = if inp < input.len() { input[inp] } else { 0 };
                inp += 1;
            }
            b'[' => {
                if tape[dp] == 0 {
                    ip = bracket_map[ip];
                }
            }
            b']' => {
                if tape[dp] != 0 {
                    ip = bracket_map[ip];
                }
            }
            _ => {}
        }
        ip += 1;
        steps += 1;
    }

    output
}

// ---------------------------------------------------------------------------
// FateRuntime
// ---------------------------------------------------------------------------

/// The fate.bf program, embedded.
// Post-2026-07-18 fate pull-in: brainfuck/ is now a sibling dir
// (`src/fate/brainfuck/`) not `../brainfuck/` — Q4 scoped adjudication.
const FATE_BF: &str = include_str!("brainfuck/fate.bf");

/// The FateRuntime: runs Fate's inference via Brainfuck.
pub struct FateRuntime {
    /// The compiled BF program (just the instruction bytes).
    program: Vec<u8>,
}

impl Default for FateRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl FateRuntime {
    /// Create a new FateRuntime from the embedded fate.bf.
    pub fn new() -> Self {
        let program: Vec<u8> = FATE_BF
            .bytes()
            .filter(|b| matches!(b, b'>' | b'<' | b'+' | b'-' | b'.' | b',' | b'[' | b']'))
            .collect();
        FateRuntime { program }
    }

    /// Encode features + model index + default cycle biases as input bytes for the BF program.
    /// Produces 22 bytes: 16 features + 1 model index + 5 bias values.
    /// Uses default_cycle() biases so the standalone runtime matches historical behavior.
    pub fn encode_input(model: Model, features: &Features) -> Vec<u8> {
        let model_idx = match model {
            Model::Abyss => 0u8,
            Model::Introject => 1,
            Model::Cartographer => 2,
            Model::Explorer => 3,
            Model::Fate => 4,
        };
        let default_weights = crate::fate::weights::Weights::default_cycle();
        let bias = default_weights.sets[model_idx as usize].bias;
        Self::encode_input_with_bias(model_idx, features, &bias)
    }

    /// Encode features + model index + explicit biases as 22 input bytes.
    pub fn encode_input_with_bias(model_idx: u8, features: &Features, bias: &[u8; 5]) -> Vec<u8> {
        let mut input = Vec::with_capacity(22);
        // 16 feature bytes (clamp f64 to 0-255)
        for &f in features.iter() {
            input.push((f as f64).clamp(0.0_f64, 255.0_f64) as u8);
        }
        // Model index byte
        input.push(model_idx);
        // 5 bias bytes
        input.extend_from_slice(bias);
        input
    }

    /// Decode the output byte as a Model.
    pub fn decode_output(output: &[u8]) -> Model {
        match output.first().copied().unwrap_or(0) {
            0 => Model::Abyss,
            1 => Model::Introject,
            2 => Model::Cartographer,
            3 => Model::Explorer,
            4 => Model::Fate,
            _ => Model::Abyss, // fallback
        }
    }

    /// Run the selector: (current model, features) → next model.
    pub fn select(&self, current: Model, features: &Features) -> Model {
        let input = Self::encode_input(current, features);
        let output = bf_execute(&self.program, &input);
        Self::decode_output(&output)
    }

    /// The instruction count of the embedded program.
    pub fn instruction_count(&self) -> usize {
        self.program.len()
    }
}

// ---------------------------------------------------------------------------
// Prism implementation for FateRuntime
// ---------------------------------------------------------------------------

impl PrismTrait for FateRuntime {
    type Input = Optic<(), (Model, Features)>;
    type Focused = Optic<(Model, Features), Vec<u8>>;
    type Projected = Optic<Vec<u8>, Model>;
    type Refracted = Optic<Model, FateRuntime>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let (model, features) = beam.result().ok().expect("focus: Err beam");
        let encoded = Self::encode_input(*model, features);
        beam.next(encoded)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let input_bytes = beam.result().ok().expect("project: Err beam");
        let output = bf_execute(&self.program, input_bytes);
        beam.next(Self::decode_output(&output))
    }

    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        // FateRuntime is its own fixed-point.
        beam.next(FateRuntime::new())
    }
}

// ---------------------------------------------------------------------------
// CompiledFateRuntime — build.rs-compiled BF, same semantics, no interpreter
// ---------------------------------------------------------------------------

/// The compiled Fate runtime: runs the BF program as native Rust code.
/// Generated by build.rs from brainfuck/fate.bf.
/// Produces identical output to FateRuntime, but faster.
pub struct CompiledFateRuntime;

impl CompiledFateRuntime {
    /// Create a new CompiledFateRuntime.
    pub fn new() -> Self {
        CompiledFateRuntime
    }

    /// Run the selector: (current model, features) -> next model.
    pub fn select(&self, current: Model, features: &Features) -> Model {
        let input = FateRuntime::encode_input(current, features);
        let output = crate::fate::compiled::fate_bf(&input);
        FateRuntime::decode_output(&output)
    }
}

impl Default for CompiledFateRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Prism implementation for CompiledFateRuntime
// ---------------------------------------------------------------------------

impl PrismTrait for CompiledFateRuntime {
    type Input = Optic<(), (Model, Features)>;
    type Focused = Optic<(Model, Features), Vec<u8>>;
    type Projected = Optic<Vec<u8>, Model>;
    type Refracted = Optic<Model, CompiledFateRuntime>;

    fn focus(&self, beam: Self::Input) -> Self::Focused {
        let (model, features) = beam.result().ok().expect("focus: Err beam");
        let encoded = FateRuntime::encode_input(*model, features);
        beam.next(encoded)
    }

    fn project(&self, beam: Self::Focused) -> Self::Projected {
        let input_bytes = beam.result().ok().expect("project: Err beam");
        let output = crate::fate::compiled::fate_bf(input_bytes);
        beam.next(FateRuntime::decode_output(&output))
    }

    fn settle(&self, beam: Self::Projected) -> Self::Refracted {
        // CompiledFateRuntime is its own fixed-point.
        beam.next(CompiledFateRuntime::new())
    }
}

// ---------------------------------------------------------------------------
// UniversalRuntime — fate_bf with injected weight awareness
// ---------------------------------------------------------------------------

/// The UniversalRuntime: runs fate.bf via the compiled function.
/// Stores an injected Weights set. With default_cycle() weights, produces
/// identical output to CompiledFateRuntime.
pub struct UniversalRuntime {
    weights: crate::fate::weights::Weights,
}

impl UniversalRuntime {
    /// Create a new UniversalRuntime with the given weights.
    pub fn new(weights: crate::fate::weights::Weights) -> Self {
        UniversalRuntime { weights }
    }

    /// Run the algorithm: returns output index 0-4.
    ///
    /// Encodes 16 features as u8 (clamp 0-255), appends context as u8,
    /// then injects the stored weight set's bias values (bytes 17-21),
    /// and calls the compiled fate_bf function.
    pub fn run(&self, context: u8, features: &crate::fate::Features) -> u8 {
        let set = &self.weights.sets[context as usize % 5];
        let input = FateRuntime::encode_input_with_bias(context, features, &set.bias);
        let output = crate::fate::compiled::fate_bf(&input);
        output.first().copied().unwrap_or(0)
    }

    /// Run and return a Model.
    pub fn select(&self, current: Model, features: &crate::fate::Features) -> Model {
        let context = match current {
            Model::Abyss => 0u8,
            Model::Introject => 1,
            Model::Cartographer => 2,
            Model::Explorer => 3,
            Model::Fate => 4,
        };
        FateRuntime::decode_output(&[self.run(context, features)])
    }

    /// Access the stored weights.
    pub fn weights(&self) -> &crate::fate::weights::Weights {
        &self.weights
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Fate, FEATURE_DIM, MODELS};

    fn zero_features() -> Features {
        [0.0; FEATURE_DIM]
    }

    #[test]
    fn runtime_creates_from_embedded_bf() {
        let rt = FateRuntime::new();
        // The new fate.bf reads 22 bytes (16 features + model + 5 biases) and
        // performs argmax — far fewer instructions than the old dispatch-based version.
        let count = rt.instruction_count();
        eprintln!("  fate.bf: {} instructions", count);
        assert!(count > 0, "fate.bf should have instructions");
        assert!(
            count < 816,
            "new bias-injection design should be simpler than old 816-instruction dispatch"
        );
    }

    #[test]
    fn runtime_abyss_to_pathfinder() {
        let rt = FateRuntime::new();
        let result = rt.select(Model::Abyss, &zero_features());
        assert_eq!(
            result,
            Model::Introject,
            "after Abyss with zero features → Introject"
        );
    }

    #[test]
    fn runtime_pathfinder_to_cartographer() {
        let rt = FateRuntime::new();
        let result = rt.select(Model::Introject, &zero_features());
        assert_eq!(
            result,
            Model::Cartographer,
            "after Introject with zero features → Cartographer"
        );
    }

    #[test]
    fn runtime_cartographer_to_explorer() {
        let rt = FateRuntime::new();
        let result = rt.select(Model::Cartographer, &zero_features());
        assert_eq!(
            result,
            Model::Explorer,
            "after Cartographer with zero features → Explorer"
        );
    }

    #[test]
    fn runtime_explorer_to_fate() {
        let rt = FateRuntime::new();
        let result = rt.select(Model::Explorer, &zero_features());
        assert_eq!(
            result,
            Model::Fate,
            "after Explorer with zero features → Fate"
        );
    }

    #[test]
    fn runtime_fate_to_abyss() {
        let rt = FateRuntime::new();
        let result = rt.select(Model::Fate, &zero_features());
        assert_eq!(
            result,
            Model::Abyss,
            "after Fate with zero features → Abyss (the cycle)"
        );
    }

    #[test]
    fn runtime_full_cycle() {
        let rt = FateRuntime::new();
        let features = zero_features();

        // The cycle: Abyss → Path → Cart → Expl → Fate → Abyss
        let m1 = rt.select(Model::Abyss, &features);
        assert_eq!(m1, Model::Introject);
        let m2 = rt.select(m1, &features);
        assert_eq!(m2, Model::Cartographer);
        let m3 = rt.select(m2, &features);
        assert_eq!(m3, Model::Explorer);
        let m4 = rt.select(m3, &features);
        assert_eq!(m4, Model::Fate);
        let m5 = rt.select(m4, &features);
        assert_eq!(m5, Model::Abyss); // back to start
    }

    #[test]
    fn runtime_feature_overrides_bias() {
        let rt = FateRuntime::new();
        let mut features = zero_features();
        features[0] = 20.0; // feature 0 boosts Cartographer (BF cell 19 → output index 2)

        let result = rt.select(Model::Abyss, &features);
        assert_eq!(
            result,
            Model::Cartographer,
            "feature[0]=20 should override bias toward Introject"
        );
    }

    #[test]
    fn runtime_implements_prism() {
        use crate::{Beam, Optic, Prism as PrismTrait};

        let rt = FateRuntime::new();
        let input: Optic<(), (Model, Features)> = Optic::ok((), (Model::Abyss, zero_features()));

        // focus: returns Focused beam
        let focused = rt.focus(input);
        assert_eq!(
            focused.result().ok().unwrap().len(),
            22,
            "22 input bytes: 16 features + 1 model + 5 biases"
        );

        // project: returns Projected beam
        let projection = rt.project(focused);
        assert_eq!(*projection.result().ok().unwrap(), Model::Introject);

        // settle: returns Refracted beam
        let crystal_beam = rt.settle(projection);
        // Crystal is FateRuntime — verify it works
        let _ = crystal_beam
            .result()
            .ok()
            .unwrap()
            .select(Model::Abyss, &zero_features());
    }

    #[test]
    fn runtime_performance() {
        let rt = FateRuntime::new();
        let features = zero_features();
        let iterations = 10_000;

        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let _ = rt.select(Model::Abyss, &features);
        }
        let elapsed = start.elapsed();
        let per_call = elapsed / iterations as u32;
        eprintln!("  FateRuntime: {} iterations in {:?}", iterations, elapsed);
        eprintln!("  Per inference: {:?}", per_call);
        eprintln!(
            "  Inferences/sec: {:.0}",
            iterations as f64 / elapsed.as_secs_f64()
        );

        // Should be under 1ms per inference (generous — likely under 100μs)
        assert!(
            per_call.as_millis() < 1,
            "BF inference should be under 1ms, got {:?}",
            per_call
        );
    }

    #[test]
    fn runtime_matches_native_fate() {
        // The BF runtime should produce the same results as the native Rust Fate
        // (when Fate has matching biases)
        let rt = FateRuntime::new();
        let mut fate = Fate::untrained();

        // Set biases to match fate.bf's hardcoded cycle
        fate.selectors[0].b[1] = 10.0; // Abyss → Path
        fate.selectors[1].b[2] = 10.0; // Path → Cart
        fate.selectors[2].b[3] = 10.0; // Cart → Expl
        fate.selectors[3].b[4] = 10.0; // Expl → Fate
        fate.selectors[4].b[0] = 10.0; // Fate → Abyss

        let features = zero_features();

        for &model in &MODELS {
            let bf_result = rt.select(model, &features);
            let native_result = fate.select(model, &features).model;
            assert_eq!(
                bf_result, native_result,
                "BF and native should agree for {:?}: BF={:?} native={:?}",
                model, bf_result, native_result
            );
        }
    }

    // -----------------------------------------------------------------------
    // CompiledFateRuntime tests
    // -----------------------------------------------------------------------

    #[test]
    fn compiled_matches_interpreted_zero_features() {
        let interpreted = FateRuntime::new();
        let compiled = CompiledFateRuntime::new();
        let features = zero_features();

        for &model in &MODELS {
            let i_result = interpreted.select(model, &features);
            let c_result = compiled.select(model, &features);
            assert_eq!(
                i_result, c_result,
                "compiled should match interpreted for {:?}: interp={:?} compiled={:?}",
                model, i_result, c_result
            );
        }
    }

    #[test]
    fn compiled_matches_interpreted_with_features() {
        let interpreted = FateRuntime::new();
        let compiled = CompiledFateRuntime::new();

        // Test with various feature vectors
        let test_features: Vec<Features> = vec![
            [0.0; FEATURE_DIM],
            {
                let mut f = [0.0; FEATURE_DIM];
                f[0] = 20.0;
                f
            },
            {
                let mut f = [0.0; FEATURE_DIM];
                f[0] = 255.0;
                f
            },
            {
                let mut f = [0.0; FEATURE_DIM];
                for i in 0..FEATURE_DIM {
                    f[i] = (i * 10) as f64;
                }
                f
            },
        ];

        for features in &test_features {
            for &model in &MODELS {
                let i_result = interpreted.select(model, features);
                let c_result = compiled.select(model, features);
                assert_eq!(
                    i_result,
                    c_result,
                    "compiled should match interpreted for {:?} with features {:?}",
                    model,
                    &features[..4]
                );
            }
        }
    }

    #[test]
    fn compiled_full_cycle() {
        let compiled = CompiledFateRuntime::new();
        let features = zero_features();

        let m1 = compiled.select(Model::Abyss, &features);
        assert_eq!(m1, Model::Introject);
        let m2 = compiled.select(m1, &features);
        assert_eq!(m2, Model::Cartographer);
        let m3 = compiled.select(m2, &features);
        assert_eq!(m3, Model::Explorer);
        let m4 = compiled.select(m3, &features);
        assert_eq!(m4, Model::Fate);
        let m5 = compiled.select(m4, &features);
        assert_eq!(m5, Model::Abyss);
    }

    #[test]
    fn compiled_feature_overrides_bias() {
        let compiled = CompiledFateRuntime::new();
        let mut features = zero_features();
        features[0] = 20.0;

        let result = compiled.select(Model::Abyss, &features);
        assert_eq!(
            result,
            Model::Cartographer,
            "feature[0]=20 should override bias toward Introject"
        );
    }

    #[test]
    fn compiled_implements_prism() {
        use crate::{Beam, Optic, Prism as PrismTrait};

        let compiled = CompiledFateRuntime::new();
        let input: Optic<(), (Model, Features)> = Optic::ok((), (Model::Abyss, zero_features()));

        // focus: returns Focused beam
        let focused = compiled.focus(input);
        assert_eq!(
            focused.result().ok().unwrap().len(),
            22,
            "22 input bytes: 16 features + 1 model + 5 biases"
        );

        // project: returns Projected beam
        let projection = compiled.project(focused);
        assert_eq!(*projection.result().ok().unwrap(), Model::Introject);

        // settle: returns Refracted beam
        let crystal_beam = compiled.settle(projection);
        // Crystal is CompiledFateRuntime — verify it works
        let _ = crystal_beam
            .result()
            .ok()
            .unwrap()
            .select(Model::Abyss, &zero_features());
    }

    #[test]
    fn compiled_performance() {
        let compiled = CompiledFateRuntime::new();
        let features = zero_features();
        let iterations = 10_000;

        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let _ = compiled.select(Model::Abyss, &features);
        }
        let compiled_elapsed = start.elapsed();
        let compiled_per = compiled_elapsed / iterations as u32;

        let interpreted = FateRuntime::new();
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let _ = interpreted.select(Model::Abyss, &features);
        }
        let interp_elapsed = start.elapsed();
        let interp_per = interp_elapsed / iterations as u32;

        let speedup = interp_elapsed.as_nanos() as f64 / compiled_elapsed.as_nanos() as f64;

        eprintln!(
            "  Interpreted: {:?}/call ({} iterations in {:?})",
            interp_per, iterations, interp_elapsed
        );
        eprintln!(
            "  Compiled:    {:?}/call ({} iterations in {:?})",
            compiled_per, iterations, compiled_elapsed
        );
        eprintln!("  Speedup:     {:.1}x", speedup);

        assert!(
            speedup > 2.0,
            "compiled should be at least 2x faster, got {:.1}x",
            speedup
        );
    }

    // -----------------------------------------------------------------------
    // UniversalRuntime tests
    // -----------------------------------------------------------------------

    #[test]
    fn universal_runtime_default_cycle() {
        let weights = crate::fate::weights::Weights::default_cycle();
        let rt = UniversalRuntime::new(weights);
        let features = zero_features();

        // Abyss (context 0) → Introject (index 1)
        assert_eq!(rt.run(0, &features), 1, "Abyss → Introject");
        // Introject (context 1) → Cartographer (index 2)
        assert_eq!(rt.run(1, &features), 2, "Introject → Cartographer");
        // Fate (context 4) → Abyss (index 0)
        assert_eq!(rt.run(4, &features), 0, "Fate → Abyss");
    }

    #[test]
    fn universal_runtime_full_cycle() {
        let weights = crate::fate::weights::Weights::default_cycle();
        let rt = UniversalRuntime::new(weights);
        let features = zero_features();

        // Walk the full cycle using run()
        assert_eq!(rt.run(0, &features), 1); // Abyss → Path
        assert_eq!(rt.run(1, &features), 2); // Path → Cart
        assert_eq!(rt.run(2, &features), 3); // Cart → Expl
        assert_eq!(rt.run(3, &features), 4); // Expl → Fate
        assert_eq!(rt.run(4, &features), 0); // Fate → Abyss
    }

    #[test]
    fn universal_matches_compiled_runtime() {
        let weights = crate::fate::weights::Weights::default_cycle();
        let rt = UniversalRuntime::new(weights);
        let compiled = CompiledFateRuntime::new();
        let features = zero_features();

        for &model in &MODELS {
            let u_result = rt.select(model, &features);
            let c_result = compiled.select(model, &features);
            assert_eq!(
                u_result, c_result,
                "UniversalRuntime should match CompiledFateRuntime for {:?}: universal={:?} compiled={:?}",
                model, u_result, c_result
            );
        }
    }

    #[test]
    fn universal_select_returns_model() {
        let weights = crate::fate::weights::Weights::default_cycle();
        let rt = UniversalRuntime::new(weights);
        let features = zero_features();

        assert_eq!(rt.select(Model::Abyss, &features), Model::Introject);
        assert_eq!(rt.select(Model::Introject, &features), Model::Cartographer);
        assert_eq!(rt.select(Model::Cartographer, &features), Model::Explorer);
        assert_eq!(rt.select(Model::Explorer, &features), Model::Fate);
        assert_eq!(rt.select(Model::Fate, &features), Model::Abyss);
    }

    #[test]
    fn universal_stores_weights() {
        let weights = crate::fate::weights::Weights::default_cycle();
        let rt = UniversalRuntime::new(weights);
        // Verify the weights are accessible and have the expected structure
        let w = rt.weights();
        // default_cycle: context 0 has bias peak at index 1
        assert_eq!(
            w.sets[0].bias[1], 10,
            "Abyss context bias for Introject should be 10"
        );
        assert_eq!(
            w.sets[0].bias[0], 0,
            "Abyss context bias for Abyss should be 0"
        );
    }

    #[test]
    fn universal_feature_override() {
        let weights = crate::fate::weights::Weights::default_cycle();
        let rt = UniversalRuntime::new(weights);
        let mut features = zero_features();
        features[0] = 20.0; // feature 0 boosts Cartographer (BF cell 19 → output index 2)

        // Same override as FateRuntime test
        let result = rt.select(Model::Abyss, &features);
        assert_eq!(
            result,
            Model::Cartographer,
            "feature[0]=20 should override bias toward Introject"
        );
    }
}
