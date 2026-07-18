/// Decomposition strategies. Cartographer selects one per tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Strategy {
    /// Eigenvalue-based graph partitioning (Fiedler bisection)
    #[default]
    SpectralPartition,
    /// Modularity-based community detection
    CommunityDetection,
    /// Level-set traversal
    BreadthFirst,
    /// Recursive subgraph descent
    DepthFirst,
    /// Random partition (baseline)
    Random,
}

impl Strategy {
    /// Zero-based ordinal in the cyclic-group structure. Fixed by
    /// declaration order in the enum; used by [`crate::GroupStructure`].
    fn ordinal(&self) -> u8 {
        match self {
            Strategy::SpectralPartition => 0,
            Strategy::CommunityDetection => 1,
            Strategy::BreadthFirst => 2,
            Strategy::DepthFirst => 3,
            Strategy::Random => 4,
        }
    }

    fn from_ordinal(i: u8) -> Self {
        match i % 5 {
            0 => Strategy::SpectralPartition,
            1 => Strategy::CommunityDetection,
            2 => Strategy::BreadthFirst,
            3 => Strategy::DepthFirst,
            _ => Strategy::Random,
        }
    }
}

/// Strategy carries the cyclic group Z/5 structure required by the
/// prismqueer principal-bundle tower (`crate::Gauge::Group: GroupStructure`).
/// The variants are ordered by declaration; composition is modular addition
/// on the 5 ordinals. Associativity + identity + inverse hold trivially.
///
/// This is a substrate-honest minimal impl — the categorical semantics of
/// each strategy do NOT imply this group structure; the group is chosen
/// solely to satisfy the type contract. Consumers pulling non-trivial
/// gauge composition (e.g. an actual sequential strategy schedule) can
/// specialise later without breaking downstream.
impl crate::GroupStructure for Strategy {
    fn identity() -> Self {
        Strategy::SpectralPartition
    }

    fn inverse(&self) -> Self {
        Strategy::from_ordinal((5 - self.ordinal()) % 5)
    }

    fn compose(&self, other: &Self) -> Self {
        Strategy::from_ordinal(self.ordinal() + other.ordinal())
    }
}
