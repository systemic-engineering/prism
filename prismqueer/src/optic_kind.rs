//! OpticKind — the taxonomy of optic field annotations.
//!
//! Six kinds form a lattice under composition (`then_*`):
//!
//! ```text
//!     Iso
//!    /   \
//!  Lens  Prism
//!    \   /
//!   Traversal
//!      |
//!    Fold  Setter
//! ```
//!
//! The `compose` method implements the cross-tier composition table.

/// The kind of optic a field is annotated with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpticKind {
    /// Round-trip lossless. `T: Into<U> + From<U>`.
    Iso,
    /// Total, bidirectional access. Always present.
    Lens,
    /// Partial access. May be absent (`Option<T>`, `Result<T, E>`).
    Prism,
    /// Multiple targets. `Vec<T>`, slices.
    Traversal,
    /// Read-only collapse. Cannot set back.
    Fold,
    /// Write-only. Cannot read.
    Setter,
}

/// Metadata for a single optic-annotated field.
#[derive(Debug, Clone)]
pub struct FieldOptic {
    /// The field name as written in the struct.
    pub name: &'static str,
    /// Which optic kind the field was annotated with.
    pub kind: OpticKind,
}

impl OpticKind {
    /// The `then_*` composition table. Returns the result of composing
    /// `self` with `other` in sequence.
    ///
    /// The rule: composition moves DOWN the lattice. Iso is identity.
    /// Fold and Setter absorb everything.
    pub fn compose(self, other: OpticKind) -> OpticKind {
        use OpticKind::*;
        match (self, other) {
            // Iso is the identity element
            (Iso, x) | (x, Iso) => x,

            // Lens . Lens = Lens (total . total = total)
            (Lens, Lens) => Lens,

            // Lens . Prism or Prism . Lens = Prism (partial wins)
            (Lens, Prism) | (Prism, Lens) => Prism,

            // Lens . Traversal or Traversal . Lens = Traversal
            (Lens, Traversal) | (Traversal, Lens) => Traversal,

            // Prism . Prism = Prism
            (Prism, Prism) => Prism,

            // Prism . Traversal or Traversal . Prism = Traversal
            (Prism, Traversal) | (Traversal, Prism) => Traversal,

            // Traversal . Traversal = Traversal
            (Traversal, Traversal) => Traversal,

            // Fold absorbs everything
            (Fold, _) | (_, Fold) => Fold,

            // Setter absorbs everything
            (Setter, _) | (_, Setter) => Setter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_is_identity() {
        use OpticKind::*;
        for kind in [Iso, Lens, Prism, Traversal, Fold, Setter] {
            assert_eq!(Iso.compose(kind), kind);
            assert_eq!(kind.compose(Iso), kind);
        }
    }

    #[test]
    fn lens_lens_is_lens() {
        assert_eq!(OpticKind::Lens.compose(OpticKind::Lens), OpticKind::Lens);
    }

    #[test]
    fn lens_prism_is_prism() {
        assert_eq!(OpticKind::Lens.compose(OpticKind::Prism), OpticKind::Prism);
        assert_eq!(OpticKind::Prism.compose(OpticKind::Lens), OpticKind::Prism);
    }

    #[test]
    fn prism_traversal_is_traversal() {
        assert_eq!(
            OpticKind::Prism.compose(OpticKind::Traversal),
            OpticKind::Traversal
        );
        assert_eq!(
            OpticKind::Traversal.compose(OpticKind::Prism),
            OpticKind::Traversal
        );
    }

    #[test]
    fn fold_absorbs_all() {
        use OpticKind::*;
        for kind in [Iso, Lens, Prism, Traversal, Fold, Setter] {
            assert_eq!(Fold.compose(kind), Fold);
            assert_eq!(kind.compose(Fold), Fold);
        }
    }

    #[test]
    fn setter_absorbs_all() {
        use OpticKind::*;
        for kind in [Iso, Lens, Prism, Traversal, Setter] {
            assert_eq!(Setter.compose(kind), Setter);
            assert_eq!(kind.compose(Setter), Setter);
        }
    }

    #[test]
    fn composition_table_full() {
        use OpticKind::*;
        assert_eq!(Lens.compose(Lens), Lens);
        assert_eq!(Lens.compose(Prism), Prism);
        assert_eq!(Prism.compose(Traversal), Traversal);
        assert_eq!(Iso.compose(Lens), Lens);
        assert_eq!(Fold.compose(Lens), Fold);
    }

    #[test]
    fn field_optic_metadata() {
        let field = FieldOptic {
            name: "status",
            kind: OpticKind::Lens,
        };
        assert_eq!(field.name, "status");
        assert_eq!(field.kind, OpticKind::Lens);
    }
}
