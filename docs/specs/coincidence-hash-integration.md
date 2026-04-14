# Coincidence Hash Integration into prism-core

**Author:** Mara
**Date:** 2026-04-14
**Status:** Proposal

## Problem

Three crates define content addressing independently:

- **fragmentation** defines `HashAlg` trait + `Sha` (SHA-256), `Fractal<E, H>`, `FrgmntStore`, `Store`
- **coincidence** defines `CoincidenceHash<N>: HashAlg`, `Detector<N>`, `Projection`, `StateVector`, `Detection`
- **prism-core** defines `Oid` (double-hash via `DefaultHasher`), `Addressable`, `Store` trait, `SpectralOid`

The dependency chain today:

```
fragmentation (defines HashAlg, Sha, Fractal, FrgmntStore, Store)
     ^
     |
coincidence (implements CoincidenceHash<N>: HashAlg, depends on fragmentation)
```

prism-core is isolated. Its `Oid::hash()` uses `DefaultHasher` (SipHash) with a
comment saying "Replace with SHA-256 behind a feature flag later." That later is
now.

## 1. Minimal coincidence hash for prism-core

### What moves into prism-core

The coincidence hash needs three things to produce an eigenvalue from bytes:

1. **StateVector** -- sparse vector in a named space (BTreeMap<String, f64>)
2. **Projection** -- idempotent linear map (P^2 = P), rank-1 from seeded vectors
3. **Detector<N>** -- N canonical projections, `detect(&[u8]) -> Detection`
4. **Encoding** -- `&[u8] -> StateVector` (sliding window, SHA-256 basis labels)
5. **Detection vocabulary** -- Eigenvalue, Focus, Magnitude, Measurement, Outcome, Strength, Detection

### Dependencies these bring

Current coincidence `Cargo.toml` dependencies:

| Dependency | Used by hash path | Used by graph/session/spectral | Verdict |
|---|---|---|---|
| `sha2` | Yes (encoding, projection seeds) | Yes | **Moves** |
| `hex` | Yes (eigenvalue display, encoding labels) | Yes | **Moves** |
| `fragmentation` | Only for `HashAlg` trait in `hash.rs` | `fragment_projection.rs` | **Does not move** |
| `git2` | No | `session.rs`, CLI | Stays |
| `hkdf` | No | `seal.rs` | Stays |
| `chacha20poly1305` | No | `seal.rs` | Stays |
| `serde_json` | No | `session.rs` | Stays |
| `rayon` | No | `concurrent_eigen.rs` | Stays |
| `rustfft` | No | `spectral.rs` (Laplacian eigenvalues) | Stays |
| `cc` | No | `ffi.rs` (build) | Stays |

**Net cost to prism-core:** two new dependencies (`sha2`, `hex`). Both are
small, widely used, zero-unsafe. prism-core currently has zero non-workspace
dependencies besides `terni`. This is the only real cost.

### Feature gate

```toml
[features]
coincidence = ["dep:sha2", "dep:hex"]
```

Without the feature, `Oid::hash()` stays as-is (DefaultHasher). With the
feature, `Oid::hash()` delegates to the coincidence detector. This preserves
prism-core's zero-dependency default for users who don't need the coincidence
hash.

### What N should be

`DEFAULT_DIMENSION = 16` in coincidence is the vector space dimension (how many
basis labels the encoding produces). This is independent of N.

N is the number of independent observers (projections). Currently:

- `CoincidenceHash<2>` -- Bothe's original: two counters, minimal
- `CoincidenceHash<5>` -- mirror's five optics (focus, project, refract, split, zoom)
- `CoincidenceHash<6>` -- used in some tests

For prism-core's `Oid::hash()`, **N=3** is the right default. Three is the
minimum for non-trivial coincidence (two can be accidental agreement; three
starts being structural). It matches prism's three operations (focus, project,
refract). It's cheap enough to be the default hash.

The const generic stays -- callers can use any N by constructing a detector
directly. The default canonical hash uses N=3.

### Detector::canonical() -- can it be const?

No. `Detector::canonical()` calls `Projection::from_seed()` which calls
SHA-256 and constructs BTreeMaps. These are heap allocations and cannot be
const.

However, `Detector::canonical()` is deterministic and cheap (three SHA-256
hashes + three outer products for N=3, dim=16). It can be cached in a
`LazyLock<Detector<3>>` for the static path:

```rust
use std::sync::LazyLock;
static CANONICAL: LazyLock<Detector<3>> = LazyLock::new(|| {
    Detector::canonical("content", 16)
});
```

### Oid::hash() replacement

```rust
impl Oid {
    #[cfg(feature = "coincidence")]
    pub fn hash(bytes: &[u8]) -> Self {
        let detection = CANONICAL.detect(bytes);
        match detection.eigenvalue_hex() {
            Some(hex) => Oid(hex),
            None => {
                // Fallback for degenerate input (empty bytes, etc.)
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(b"prism-core:dark:");
                hasher.update(bytes);
                Oid(hex::encode(hasher.finalize()))
            }
        }
    }

    #[cfg(not(feature = "coincidence"))]
    pub fn hash(bytes: &[u8]) -> Self {
        // Current DefaultHasher implementation (unchanged)
        ...
    }
}
```

## 2. Fragmentation becomes prism-native

### Fractal<E, H> implements Addressable

`Addressable` in prism-core requires `fn oid(&self) -> Oid`. Fragmentation's
`Fractal<E, H>` already computes content OIDs via `content_oid()` (git-compatible
SHA-1). The bridge:

```rust
// In fragmentation, behind a `prism` feature
impl<E: Encode, H: HashAlg> Addressable for Fractal<E, H> {
    fn oid(&self) -> Oid {
        Oid::new(content_oid(self))
    }
}
```

This is a one-way bridge: fragmentation depends on prism-core for the
`Addressable` trait, but prism-core does not depend on fragmentation. The
`content_oid()` function stays in fragmentation because it uses git-compatible
SHA-1 hashing which is fragmentation-specific.

### FrgmntStore implements prism-core's Store

prism-core's `Store` trait:

```rust
pub trait Store {
    type Error;
    type L: Loss;
    fn get(&self, oid: &Oid) -> Imperfect<Vec<u8>, Self::Error, Self::L>;
    fn put(&mut self, oid: Oid, data: Vec<u8>) -> Imperfect<Oid, Self::Error, Self::L>;
    fn has(&self, oid: &Oid) -> Imperfect<Luminosity, Self::Error, Self::L>;
}
```

`FrgmntStore` currently uses `String` keys and its own `Error` type. The
implementation:

```rust
// In fragmentation, behind a `prism` feature
impl<N: Reconstructable + Clone> prism_core::Store for FrgmntStore<N>
where
    N::Data: Encode + Decode,
    N::Hash: HashAlg,
{
    type Error = frgmnt_store::Error;
    type L = ScalarLoss;  // from prism-core

    fn get(&self, oid: &Oid) -> Imperfect<Vec<u8>, Self::Error, Self::L> { ... }
    fn put(&mut self, oid: Oid, data: Vec<u8>) -> Imperfect<Oid, Self::Error, Self::L> { ... }
    fn has(&self, oid: &Oid) -> Imperfect<Luminosity, Self::Error, Self::L> { ... }
}
```

The `get` path: `oid.as_str()` -> `get_persistent(key)` -> encode data to
`Vec<u8>`. The `put` path: decode bytes into `N`, compute content_oid, insert.
The `has` path: check cache then disk, return `Light`/`Dark`.

### HashAlg bridge

fragmentation's `HashAlg` and prism-core's `Addressable` serve different roles:

- `HashAlg` is a hash function interface (`fn hash(&[u8]) -> Self`)
- `Addressable` is a content-identity interface (`fn oid(&self) -> Oid`)

With prism-core gaining coincidence hash, we can implement `HashAlg` for `Oid`:

```rust
// In fragmentation, behind a `prism` feature
impl HashAlg for Oid {
    fn hash(data: &[u8]) -> Self { Oid::hash(data) }
    fn from_hex(hex: impl Into<String>) -> Self { Oid::new(hex) }
    fn as_str(&self) -> &str { self.as_str() }
}
```

This means `Fractal<E, Oid>` works -- fragments addressed by prism-core OIDs.

## 3. Dependency chain after integration

### Before

```
fragmentation          prism-core (isolated)
     ^
     |
coincidence
```

### After

```
terni
  ^
  |
prism-core[coincidence]  -- gains sha2, hex; contains Detector, Projection,
  ^     ^                   StateVector, Detection, encoding
  |     |
  |   fragmentation[prism]  -- gains dep on prism-core; implements Addressable
  |     ^                      for Fractal, Store for FrgmntStore, HashAlg for Oid
  |     |
coincidence (slimmed)       -- keeps graph, spectral, session, seal, ffi, sigma,
                               trajectory, evolve; depends on prism-core for hash
```

### Key principle

prism-core does NOT depend on fragmentation. The direction is:
fragmentation -> prism-core. This is correct because prism-core is the
foundational layer (content addressing, beams, stores) and fragmentation is
a data structure layer that uses those foundations.

## 4. What stays in coincidence vs what moves to prism-core

### Moves to prism-core (behind `coincidence` feature)

| Module | Why |
|---|---|
| `state.rs` (StateVector) | Required by Detector |
| `projection.rs` (Projection) | Required by Detector |
| `coincidence.rs` (Detector, Coincidence trait) | Core hash mechanism |
| `detection.rs` (Detection, Eigenvalue, Focus, Magnitude, Measurement, Outcome, Strength) | Return types from Detector |
| `encoding.rs` (encode, encode_into_basis, encode_in_space) | &[u8] -> StateVector |
| `eigenvalue.rs` (extract fn) | Helper used by Detector |

### Stays in coincidence

| Module | Why |
|---|---|
| `graph.rs` | Graph construction, adjacency -- domain-specific |
| `spectral.rs` | Laplacian eigendecomposition -- needs rustfft |
| `eigenvalues.rs` | Heat kernel, spectral dimension -- graph analysis |
| `sigma.rs` | Von Neumann entropy -- graph analysis |
| `session.rs` | Session management -- needs git2, serde_json |
| `seal.rs` | HKDF + ChaCha20 sealing -- needs hkdf, chacha20poly1305 |
| `ffi.rs` | C FFI -- needs cc |
| `evolve.rs` | Graph evolution |
| `trajectory.rs` | Trajectory tracking |
| `curvature.rs` | Information curvature |
| `neighborhood.rs` | Graph neighborhoods |
| `incidence.rs` | Incidence matrices |
| `dense.rs` | Dense matrix operations |
| `bounded_eigen.rs` | Bounded eigenvalue computation |
| `concurrent_eigen.rs` | Concurrent eigenvalue computation -- needs rayon |
| `eigen_cache.rs` | Eigenvalue caching |
| `agreement.rs` | Agreement metrics |
| `complexity.rs` | Graph complexity |
| `commutator.rs` | Commutator brackets |
| `crystallize.rs` | Crystallization |
| `fragment_projection.rs` | Fragment-specific projections |
| `hash.rs` | CoincidenceHash<N>: HashAlg -- bridge code, stays until fragmentation adopts Oid |
| `cli.rs` | CLI interface |
| `session_hash.rs` | Session hashing |
| `hash_cache.rs` | Hash caching |

The split is clean: **measurement substrate** moves to prism-core,
**graph analysis** stays in coincidence. coincidence becomes a graph/spectral
crate that depends on prism-core for its hash, rather than defining its own.

## 5. Migration path

### Phase 1: prism-core gains coincidence hash (no breaking changes)

1. Add `sha2` and `hex` as optional dependencies behind `coincidence` feature
2. Copy (not move) `state.rs`, `projection.rs`, `coincidence.rs` (renamed to
   `detector.rs`), `detection.rs`, `encoding.rs`, `eigenvalue.rs` into
   `prism-core/src/` behind `#[cfg(feature = "coincidence")]`
3. Add `Oid::hash()` implementation behind the feature flag
4. Add `LazyLock<Detector<3>>` for the canonical detector
5. All existing tests pass. New tests for coincidence-backed Oid::hash()
6. prism-core's `Oid` comment "Replace with SHA-256 behind a feature flag later"
   gets resolved

### Phase 2: fragmentation depends on prism-core

1. Add `prism-core` as optional dependency behind `prism` feature
2. Implement `Addressable for Fractal<E, H>` behind the feature
3. Implement `Store for FrgmntStore<N>` behind the feature
4. Implement `HashAlg for Oid` behind the feature
5. This enables `Fractal<E, Oid>` -- fragments using coincidence-backed content
   addresses

### Phase 3: coincidence depends on prism-core instead of fragmentation

1. coincidence swaps `fragmentation` dependency for `prism-core[coincidence]`
2. `CoincidenceHash<N>` in `hash.rs` wraps `Oid` instead of reimplementing
   the detector logic -- or is removed entirely if callers switch to `Oid`
3. `hash.rs` becomes a thin compatibility shim: `impl HashAlg for CoincidenceHash<N>`
   delegates to `Detector<N>` from prism-core
4. Graph/spectral modules use `Oid` for content addressing instead of
   `CoincidenceHash<N>` directly
5. coincidence's direct `sha2`/`hex` dependencies may be removable if all hash
   paths go through prism-core (projection seeds still need sha2 directly)

### Phase 4: cleanup

1. Remove duplicated StateVector/Projection/Detector/Detection from coincidence
2. Re-export from prism-core: `pub use prism_core::{Detector, Detection, ...}`
3. `CoincidenceHash<N>` becomes a type alias or thin wrapper if still needed
4. fragmentation's `Sha` (SHA-256) remains available for users who want plain
   SHA-256 without coincidence detection

### What fragmentation gains

- **Addressable**: Fractals participate in prism pipelines natively
- **Store trait**: FrgmntStore is a first-class prism Store with Imperfect
  return types and Luminosity
- **Coincidence hash**: `Fractal<E, Oid>` uses eigenvalue-based content
  addressing -- collision resistance from multiple independent projections
  instead of a single hash function

### What fragmentation loses

- **Nothing breaks**: The `prism` feature is opt-in. `Fractal<String, Sha>`
  still works exactly as before. All existing code compiles unchanged.
- **One new dependency** (prism-core) when the feature is enabled

### What coincidence gains

- **Canonical home** for its measurement substrate in prism-core
- **Lighter crate**: graph analysis without carrying the hash primitives
- **prism integration**: Detector output slots directly into Oid

### What coincidence loses

- **Self-containment**: the hash path moves to prism-core. This is correct
  because the hash IS prism-core's content addressing. The detector IS the
  hash function. Keeping it in coincidence was always a dependency inversion.

## Appendix: DEFAULT_DIMENSION = 16

The dimension parameter controls the vector space size for encoding. Bytes are
mapped into a space with `dimension` basis labels via SHA-256 seeded sliding
window encoding. Larger dimension = more collision resistance, more compute.

- `dimension = 8`: fast, lower collision resistance. Good for tests.
- `dimension = 16`: default. 16 basis dimensions, each projection is a 16x16
  matrix. Good balance of collision resistance and speed.
- `dimension = 32+`: higher collision resistance, used for high-security paths.

The dimension is independent of N (number of projections). N=3, dim=16 means
three 16x16 projection matrices applied to a 16-dimensional state vector.
This produces a 64-char hex eigenvalue string, matching Oid's current format.
