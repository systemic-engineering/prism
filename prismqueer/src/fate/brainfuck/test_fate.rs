/// test_fate.rs — Brainfuck interpreter + Fate selector tests
///
/// Verifies that fate.bf implements the Fate five-model selector correctly.
/// The interpreter is ~60 lines. The tests verify context-dependent selection.
///
/// Run: rustc test_fate.rs -o test_fate && ./test_fate

use std::fs;

/// Minimal Brainfuck interpreter.
/// Returns the output bytes produced by the program.
fn bf_run(program: &str, input: &[u8]) -> Vec<u8> {
    let ops: Vec<char> = program.chars().filter(|c| "<>+-.,[]".contains(*c)).collect();

    // Precompute bracket matching
    let mut bracket_map = vec![0usize; ops.len()];
    let mut stack = Vec::new();
    for (i, &op) in ops.iter().enumerate() {
        if op == '[' {
            stack.push(i);
        } else if op == ']' {
            let j = stack.pop().expect("unmatched ]");
            bracket_map[i] = j;
            bracket_map[j] = i;
        }
    }
    assert!(stack.is_empty(), "unmatched [");

    // Execute
    let mut tape = vec![0u8; 4096];
    let mut dp: usize = 0;
    let mut ip: usize = 0;
    let mut inp: usize = 0;
    let mut output = Vec::new();
    let mut steps: u64 = 0;
    let max_steps: u64 = 10_000_000;

    while ip < ops.len() {
        steps += 1;
        if steps > max_steps {
            panic!(
                "exceeded {} steps (ip={}, dp={}, tape[dp]={})",
                max_steps, ip, dp, tape[dp]
            );
        }
        match ops[ip] {
            '>' => {
                dp += 1;
                if dp >= tape.len() {
                    tape.resize(dp + 1024, 0);
                }
            }
            '<' => {
                assert!(dp > 0, "data pointer underflow at instruction {}", ip);
                dp -= 1;
            }
            '+' => tape[dp] = tape[dp].wrapping_add(1),
            '-' => tape[dp] = tape[dp].wrapping_sub(1),
            '.' => output.push(tape[dp]),
            ',' => {
                tape[dp] = if inp < input.len() { input[inp] } else { 0 };
                inp += 1;
            }
            '[' => {
                if tape[dp] == 0 {
                    ip = bracket_map[ip];
                }
            }
            ']' => {
                if tape[dp] != 0 {
                    ip = bracket_map[ip];
                }
            }
            _ => {}
        }
        ip += 1;
    }

    output
}

/// Build input: 16 feature bytes + 1 model index byte
fn make_input(model_index: u8, features: &[u8; 16]) -> Vec<u8> {
    let mut v = features.to_vec();
    v.push(model_index);
    v
}

/// Run a single test case and return pass/fail
fn run_test(
    program: &str,
    name: &str,
    model: u8,
    features: &[u8; 16],
    expected: u8,
) -> bool {
    print!("  {} ... ", name);
    let input = make_input(model, features);
    let output = bf_run(program, &input);
    if output.is_empty() {
        println!("FAIL (no output)");
        return false;
    }
    if output[0] != expected {
        println!("FAIL (expected {}, got {})", expected, output[0]);
        return false;
    }
    println!("PASS ({})", output[0]);
    true
}

fn main() {
    let program = fs::read_to_string("fate.bf").expect("cannot read fate.bf");
    let zero = [0u8; 16];

    println!("Fate Brainfuck Selector — Test Suite");
    println!("====================================\n");

    // --- Instruction count ---
    let total = program.chars().filter(|c| "<>+-.,[]".contains(*c)).count();
    let plus = program.chars().filter(|c| *c == '+').count();
    let minus = program.chars().filter(|c| *c == '-').count();
    let left = program.chars().filter(|c| *c == '<').count();
    let right = program.chars().filter(|c| *c == '>').count();
    let comma = program.chars().filter(|c| *c == ',').count();
    let dot = program.chars().filter(|c| *c == '.').count();
    let open = program.chars().filter(|c| *c == '[').count();
    let close = program.chars().filter(|c| *c == ']').count();

    println!("Instruction profile:");
    println!("  +  increment:  {:4}", plus);
    println!("  -  decrement:  {:4}", minus);
    println!("  <  move left:  {:4}", left);
    println!("  >  move right: {:4}", right);
    println!("  ,  input:      {:4}", comma);
    println!("  .  output:     {:4}", dot);
    println!("  [  loop open:  {:4}", open);
    println!("  ]  loop close: {:4}", close);
    println!("  TOTAL:         {:4}\n", total);

    // --- Tests ---
    println!("Context-dependent selection (bias only, zero features):");
    let mut passed = 0;
    let mut failed = 0;

    let tests: Vec<(&str, u8, &[u8; 16], u8)> = vec![
        ("Abyss(0) -> Cartographer(1)", 0, &zero, 1),
        ("Cartographer(1) -> Introject(2)", 1, &zero, 2),
        ("Introject(2) -> Explorer(3)", 2, &zero, 3),
        ("Explorer(3) -> Fate(4)", 3, &zero, 4),
        ("Fate(4) -> Abyss(0)", 4, &zero, 0),
    ];

    for (name, model, features, expected) in &tests {
        if run_test(&program, name, *model, features, *expected) {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    println!("\nFeature contribution (weight * feature added to score):");
    let mut f0_20 = [0u8; 16];
    f0_20[0] = 20;
    // After Abyss: Cartographer has bias 10, Introject gets feature[0]=20
    // Last-nonzero argmax picks Introject (index 2, rightmost nonzero)
    if run_test(
        &program,
        "Abyss(0) + f[0]=20 -> Introject(2)",
        0,
        &f0_20,
        2,
    ) {
        passed += 1;
    } else {
        failed += 1;
    }

    println!("\n------------------------------------");
    if failed == 0 {
        println!("All {} tests passed.\n", passed);
    } else {
        println!("{} passed, {} failed.\n", passed, failed);
    }

    // --- Kolmogorov Complexity Analysis ---
    // DATA: +/- chars that encode specific numeric values (weights, biases,
    //       dispatch offsets). These change when the model changes.
    // ALGORITHM: everything else (movement, control, I/O, structural +/-).
    //            This stays the same regardless of weight values.
    //
    // Data chars breakdown:
    //   5 bias values of 10  = 50 '+' chars
    //   5 flag initializations = 5 '+' chars
    //   Subtraction for dispatch (0+1+2+3+4) = 10 '-' chars
    //   5 flag clears = 5 '-' chars
    //   Argmax index values (0+1+2+3+4) = 10 '+' chars
    //   Total data = 80

    let data_chars = 80usize; // hand-counted, specific to this weight set
    let algo_chars = total - data_chars;

    println!("Kolmogorov Complexity Analysis");
    println!("==============================");
    println!("Total BF instructions:      {}", total);
    println!();
    println!("ALGORITHM (universal):       {} ({:.1}%)", algo_chars, algo_chars as f64 / total as f64 * 100.0);
    println!("  Movement (<>):            {}", left + right);
    println!("  Control ([]):             {}", open + close);
    println!("  I/O (,.):                {}", comma + dot);
    println!("  Structural +/-:           {}", (plus + minus) - data_chars);
    println!();
    println!("DATA (weight-specific):      {} ({:.1}%)", data_chars, data_chars as f64 / total as f64 * 100.0);
    println!("  Bias values (5 x 10):    50");
    println!("  Flag init/clear:         10");
    println!("  Dispatch offsets:        10");
    println!("  Argmax index values:     10");
    println!();
    println!("The algorithm is UNIVERSAL. The weights are SPECIFIC.");
    println!("Change the biases -> only the {} data chars change.", data_chars);
    println!("The {} algorithm chars are the Kolmogorov complexity", algo_chars);
    println!("of Fate's decision procedure itself.");

    if failed > 0 {
        std::process::exit(1);
    }
}
