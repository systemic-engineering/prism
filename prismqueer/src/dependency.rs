//! `prismqueer::dependency` — the engine's dependency graph (prism-008).
//!
//! Directed (premise → head), signed (positive, or through a `not`), and a
//! multigraph: parallel edges are kept, so two rules deriving the same head
//! from the same premise both count, and reuse shows in the cycle rank b₁.
//! Plus reachability. The shadow, R⁺, the removal seed and N1 are mirror's
//! meaning on top; the graph is the engine's. Deterministic: edges in
//! insertion order, nodes and reach results ordered.

use std::collections::{BTreeMap, BTreeSet};

/// Whether an edge is a plain dependency or one through a `not`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Sign {
    /// The head depends on the premise holding.
    Positive,
    /// The head depends on the premise *not* holding.
    Negative,
}

/// Which edges reachability may follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Through {
    /// Only positive edges.
    Positive,
    /// Every edge, positive or negative.
    Any,
}

/// One dependency: `from` (premise) → `to` (head), with its sign.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge<N> {
    from: N,
    to: N,
    sign: Sign,
}

impl<N> Edge<N> {
    /// The premise.
    pub fn from(&self) -> &N {
        &self.from
    }
    /// The head.
    pub fn to(&self) -> &N {
        &self.to
    }
    /// Positive or through a `not`.
    pub fn sign(&self) -> Sign {
        self.sign
    }
}

/// A directed, signed multigraph of dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependencyGraph<N: Ord> {
    nodes: BTreeSet<N>,
    edges: Vec<Edge<N>>,
}

impl<N: Ord> Default for DependencyGraph<N> {
    fn default() -> Self {
        DependencyGraph {
            nodes: BTreeSet::new(),
            edges: Vec::new(),
        }
    }
}

impl<N: Ord + Clone> DependencyGraph<N> {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node with no edges.
    pub fn add_node(&mut self, n: N) {
        self.nodes.insert(n);
    }

    /// Add an edge `from → to`. Parallel edges are kept.
    pub fn add_edge(&mut self, from: N, to: N, sign: Sign) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());
        self.edges.push(Edge { from, to, sign });
    }

    /// How many edges, counting parallel ones.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// How many edges run `from → to`, of either sign.
    pub fn multiplicity(&self, from: &N, to: &N) -> usize {
        self.edges
            .iter()
            .filter(|e| &e.from == from && &e.to == to)
            .count()
    }

    /// The edges through a `not`, in insertion order.
    pub fn negative_edges(&self) -> impl Iterator<Item = &Edge<N>> {
        self.edges.iter().filter(|e| e.sign == Sign::Negative)
    }

    /// The seeds and everything reachable from them along edges `through`
    /// allows.
    pub fn reach(&self, seeds: &BTreeSet<N>, through: Through) -> BTreeSet<N> {
        let mut out: BTreeSet<N> = seeds
            .iter()
            .filter(|s| self.nodes.contains(s))
            .cloned()
            .collect();
        let mut frontier: Vec<N> = out.iter().cloned().collect();
        while let Some(n) = frontier.pop() {
            for e in &self.edges {
                let allowed = through == Through::Any || e.sign == Sign::Positive;
                if allowed && e.from == n && out.insert(e.to.clone()) {
                    frontier.push(e.to.clone());
                }
            }
        }
        out
    }

    /// Connected components of the underlying undirected multigraph.
    pub fn components(&self) -> usize {
        let index: BTreeMap<&N, usize> =
            self.nodes.iter().enumerate().map(|(i, n)| (n, i)).collect();
        let mut parent: Vec<usize> = (0..index.len()).collect();
        fn root(p: &mut [usize], i: usize) -> usize {
            if p[i] == i {
                i
            } else {
                let r = root(p, p[i]);
                p[i] = r;
                r
            }
        }
        let mut count = index.len();
        for e in &self.edges {
            let (a, b) = (
                root(&mut parent, index[&e.from]),
                root(&mut parent, index[&e.to]),
            );
            if a != b {
                parent[a] = b;
                count -= 1;
            }
        }
        count
    }

    /// b₁ of the underlying undirected multigraph: |E| − |V| + components.
    /// Parallel edges and self-loops count, which is how reuse shows.
    pub fn cycle_rank(&self) -> usize {
        self.edges.len() + self.components() - self.nodes.len()
    }
}
