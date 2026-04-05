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
        todo!("Tape::new not implemented")
    }

    pub fn read(&self, _offset: usize) -> u8 {
        todo!("Tape::read not implemented")
    }

    pub fn write(&mut self, _offset: usize, _value: u8) {
        todo!("Tape::write not implemented")
    }

    pub fn add(&mut self, _offset: usize, _value: u8) {
        todo!("Tape::add not implemented")
    }

    pub fn advance(&mut self, _n: usize) {
        todo!("Tape::advance not implemented")
    }
}

impl Default for Tape {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute a Metal program on a tape with input.
/// Returns the output bytes.
pub fn execute(_program: &[Instruction], _input: &[u8]) -> Vec<u8> {
    todo!("execute not implemented")
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
        let program = vec![
            Instruction::Focus(2),
            Instruction::Zoom(0, 10),
            Instruction::Refract,
        ];
        let output = execute(&program, &[5, 7]);
        assert_eq!(output, vec![10]);
    }

    #[test]
    fn split_finds_last_nonzero() {
        let program = vec![
            Instruction::Focus(5),
            Instruction::Split(5),
            Instruction::Refract,
        ];
        let output = execute(&program, &[0, 0, 10, 0, 20]);
        assert_eq!(output, vec![0]);
    }

    #[test]
    fn fate_selector_no_refract_produces_no_output() {
        let mut input = vec![0u8; 16];
        input.push(0);
        input.extend_from_slice(&[0, 10, 0, 0, 0]);
        let program = vec![Instruction::Focus(22)];
        let output = execute(&program, &input);
        assert!(output.is_empty());
    }

    #[test]
    fn project_zeroes_below_threshold() {
        let program = vec![
            Instruction::Focus(3),
            Instruction::Project(5),
        ];
        let output = execute(&program, &[1, 5, 10]);
        assert!(output.is_empty());
    }

    #[test]
    fn instruction_count_for_fate() {
        let program = vec![
            Instruction::Focus(22),
            Instruction::Zoom(0, 0),
            Instruction::Split(5),
            Instruction::Refract,
        ];
        assert_eq!(program.len(), 4);
    }

    #[test]
    fn refract_outputs_current_cell() {
        let program = vec![
            Instruction::Focus(1),
            Instruction::Refract,
        ];
        let output = execute(&program, &[42]);
        assert_eq!(output, vec![0]);
    }

    #[test]
    fn zoom_wraps_on_overflow() {
        let program = vec![
            Instruction::Zoom(0, 255),
            Instruction::Zoom(0, 2),
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
        tape.advance(300);
        assert_eq!(tape.dp, 255);
    }
}
