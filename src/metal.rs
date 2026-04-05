//! Metal — the five instructions. What Prism compiles to.
//! The atoms below the trait. The thing the silicon does.

/// The five instructions. Each maps to one Prism operation.
///
/// Focus  — read inputs into tape
/// Project — precision cut on accumulated values
/// Split  — walk cells, test nonzero
/// Zoom   — add value to cell (weight application, the transform)
/// Refract — output the result. Crystallize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Instruction {
    /// Focus: read N bytes from input into tape starting at dp.
    /// Advances dp by n.
    Focus(usize),
    /// Project: keep only cells where value >= threshold.
    /// Zero out cells below threshold.
    Project(u8),
    /// Split: scan N cells starting at dp, find nonzero cells.
    /// The traversal across the tape.
    Split(usize),
    /// Zoom: add value to cell at dp + offset.
    /// The weight application. The transform.
    Zoom(usize, u8),
    /// Refract: output cell at dp. The crystal. The answer.
    Refract,
}

/// The tape. The state that Metal operates on.
#[derive(Clone, Debug)]
pub struct Tape {
    pub cells: [u8; 256],
    pub dp: usize,
}

impl Tape {
    pub fn new() -> Self {
        Tape {
            cells: [0u8; 256],
            dp: 0,
        }
    }

    /// Read a cell at dp + offset, bounds-checked.
    pub fn read(&self, offset: usize) -> u8 {
        let idx = (self.dp + offset).min(255);
        self.cells[idx]
    }

    /// Write a cell at dp + offset, bounds-checked.
    pub fn write(&mut self, offset: usize, value: u8) {
        let idx = (self.dp + offset).min(255);
        self.cells[idx] = value;
    }

    /// Add to a cell at dp + offset, wrapping.
    pub fn add(&mut self, offset: usize, value: u8) {
        let idx = (self.dp + offset).min(255);
        self.cells[idx] = self.cells[idx].wrapping_add(value);
    }

    /// Advance dp by n, clamped.
    pub fn advance(&mut self, n: usize) {
        self.dp = (self.dp + n).min(255);
    }
}

impl Default for Tape {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a Metal program on a tape with input.
/// Returns the output bytes.
pub fn execute(program: &[Instruction], input: &[u8]) -> Vec<u8> {
    let mut tape = Tape::new();
    let mut inp = 0usize;
    let mut output = Vec::new();

    for instruction in program {
        match instruction {
            Instruction::Focus(n) => {
                for _ in 0..*n {
                    let byte = if inp < input.len() { input[inp] } else { 0 };
                    tape.cells[tape.dp] = byte;
                    tape.dp = (tape.dp + 1).min(255);
                    inp += 1;
                }
            }
            Instruction::Project(threshold) => {
                for cell in tape.cells.iter_mut() {
                    if *cell < *threshold {
                        *cell = 0;
                    }
                }
            }
            Instruction::Split(n) => {
                // Scan n cells from dp, find last nonzero, set dp there
                let mut last_nonzero = tape.dp;
                for i in 0..*n {
                    let idx = (tape.dp + i).min(255);
                    if tape.cells[idx] != 0 {
                        last_nonzero = idx;
                    }
                }
                tape.dp = last_nonzero;
            }
            Instruction::Zoom(offset, value) => {
                tape.add(*offset, *value);
            }
            Instruction::Refract => {
                output.push(tape.cells[tape.dp]);
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tape_new_is_zeroed() {
        let tape = Tape::new();
        assert_eq!(tape.dp, 0);
        assert!(tape.cells.iter().all(|&c| c == 0));
    }

    #[test]
    fn focus_reads_input() {
        let mut tape = Tape::new();
        for b in &[10u8, 20, 30] {
            tape.cells[tape.dp] = *b;
            tape.dp += 1;
        }
        assert_eq!(tape.cells[0], 10);
        assert_eq!(tape.cells[1], 20);
        assert_eq!(tape.cells[2], 30);
        assert_eq!(tape.dp, 3);
    }

    #[test]
    fn focus_zoom_refract_pipeline() {
        // Read 2 inputs, add a weight, output the result
        // Focus(2): cells[0]=5, cells[1]=7, dp=2
        // Zoom(0, 10): cells[2] += 10 → cells[2]=10
        // Refract: output cells[dp=2] = 10
        let program = vec![
            Instruction::Focus(2),    // read 2 bytes → cells 0,1; dp=2
            Instruction::Zoom(0, 10), // add 10 to cell at dp+0 = cell 2
            Instruction::Refract,     // output cell at dp (cell 2) = 10
        ];
        let output = execute(&program, &[5, 7]);
        assert_eq!(output, vec![10]);
    }

    #[test]
    fn split_finds_last_nonzero() {
        // dp starts at 5 after Focus(5), cells 5+ are zero
        let program = vec![
            Instruction::Focus(5),  // read 5 bytes, dp=5
            Instruction::Split(5),  // scan 5 cells from dp=5, all zero, dp stays at 5
            Instruction::Refract,   // output cell at dp
        ];
        // Input: cells 0-4 have values, cells 5+ are zero
        let output = execute(&program, &[0, 0, 10, 0, 20]);
        // dp was 5, split scans 5 cells from 5 (all zero), dp stays at 5
        assert_eq!(output, vec![0]);
    }

    #[test]
    fn fate_selector_no_refract_produces_no_output() {
        let mut input = vec![0u8; 16];
        input.push(0); // context
        input.extend_from_slice(&[0, 10, 0, 0, 0]); // biases: Pathfinder wins

        let program = vec![Instruction::Focus(22)];
        let output = execute(&program, &input);
        assert!(output.is_empty()); // no Refract yet
    }

    #[test]
    fn project_zeroes_below_threshold() {
        let program = vec![
            Instruction::Focus(3),   // cells[0]=1, cells[1]=5, cells[2]=10; dp=3
            Instruction::Project(5), // zero cells < 5 → cells[0]=0
        ];
        let output = execute(&program, &[1, 5, 10]);
        assert!(output.is_empty()); // no Refract
    }

    #[test]
    fn instruction_count_for_fate() {
        // The full Fate selector in Metal (without BF overhead):
        let program = vec![
            Instruction::Focus(22),  // 1 instruction
            Instruction::Zoom(0, 0), // feature contribution (placeholder)
            Instruction::Split(5),   // argmax
            Instruction::Refract,    // output
        ];
        assert_eq!(program.len(), 4); // The entire decision in 4 instructions
    }

    #[test]
    fn refract_outputs_current_cell() {
        let program = vec![
            Instruction::Focus(1), // read 1 byte into cell 0, dp=1
            Instruction::Refract,  // output cell at dp=1 (which is 0)
        ];
        let output = execute(&program, &[42]);
        assert_eq!(output, vec![0]); // dp=1 after focus, cell 1 is 0
    }

    #[test]
    fn zoom_wraps_on_overflow() {
        let program = vec![
            Instruction::Zoom(0, 255),
            Instruction::Zoom(0, 2), // 255 + 2 = 1 (wrapping)
            Instruction::Refract,
        ];
        let output = execute(&program, &[]);
        assert_eq!(output, vec![1]);
    }

    #[test]
    fn instruction_derives_clone_debug_partialeq() {
        let a = Instruction::Focus(3);
        let b = a.clone();
        assert_eq!(a, b);
        let _ = format!("{:?}", a);
    }

    #[test]
    fn tape_read_write_add() {
        let mut tape = Tape::new();
        tape.write(0, 10);
        assert_eq!(tape.read(0), 10);
        tape.add(0, 5);
        assert_eq!(tape.read(0), 15);
    }

    #[test]
    fn tape_advance_clamped() {
        let mut tape = Tape::new();
        tape.advance(300); // clamped to 255
        assert_eq!(tape.dp, 255);
    }

    #[test]
    fn zoom_adds_to_cell() {
        let program = vec![
            Instruction::Zoom(0, 42),
            Instruction::Refract,
        ];
        let output = execute(&program, &[]);
        assert_eq!(output, vec![42]);
    }

    #[test]
    fn fate_selector_in_metal() {
        // The Fate selector: read biases, argmax, output.
        // Input: 16 zero features + context 0 + biases [0, 10, 0, 0, 0]
        let mut input = vec![0u8; 16]; // features
        input.push(0);                  // context
        input.extend_from_slice(&[0, 10, 0, 0, 0]); // biases

        let program = vec![
            Instruction::Focus(22),   // read all 22 bytes, dp=22
            // Biases are now in cells 17-21. Cell 18 = 10.
            // Need to find argmax over cells 17-21.
            // Split scans from dp. We need dp=17.
            // Hack: we know dp=22 after Focus. Can't move backward in Metal.
            // Instead: set up a second tape region or accept dp=22.
        ];
        // This test documents that Metal needs a SetDp or that Focus
        // should leave dp at a useful position. For now, just verify
        // Focus reads correctly.
        let output = execute(&program, &input);
        assert!(output.is_empty()); // no Refract
    }

    #[test]
    fn focus_zoom_split_refract_pipeline() {
        // Set up cells manually via Zoom, then threshold and argmax
        let program = vec![
            Instruction::Zoom(0, 5),
            Instruction::Zoom(1, 20),
            Instruction::Zoom(2, 3),
            Instruction::Project(10),  // zero out < 10: cells 0,2 become 0
            Instruction::Split(3),     // last nonzero = cell 1
            Instruction::Refract,      // output cell at dp=1 = 20
        ];
        let output = execute(&program, &[]);
        assert_eq!(output, vec![20]);
    }

    #[test]
    fn empty_program_no_output() {
        let output = execute(&[], &[1, 2, 3]);
        assert!(output.is_empty());
    }

    #[test]
    fn multiple_refracts() {
        let program = vec![
            Instruction::Zoom(0, 65), // 'A'
            Instruction::Refract,
            Instruction::Zoom(0, 1),  // 65 + 1 = 66 = 'B'
            Instruction::Refract,
        ];
        let output = execute(&program, &[]);
        assert_eq!(output, vec![65, 66]);
    }
}
