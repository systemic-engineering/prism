//! The witness quorum (Mara's Lemma B, `research-ledger-design.md` §2.3).
//!
//! N witnesses, at most f dishonest; each honest witness cosigns only
//! extensions of what it cosigned before. A reader accepting a checkpoint
//! only with **Q = ⌊(N+f)/2⌋ + 1** distinct valid cosignatures can't be
//! shown two inconsistent checkpoints: two Q-sets share ≥ 2Q − N ≥ f + 1
//! witnesses, so an honest one cosigned both. Liveness (Corollary C) needs
//! N ≥ 3f + 1; it is reported, not required.
//!
//! **The quorum is exactly as post-quantum as its cosignature scheme**
//! (Mara, BROKEN on "nothing to break"): Lemma B assumes unforgeable
//! cosignatures. Here they are ML-DSA-44 (C2SP type 0x06), so the count
//! rests on Module-LWE/SIS. With Ed25519 cosignatures a quantum adversary
//! forges a quorum.

use alloc::vec::Vec;

use crate::checkpoint::{Checkpoint, MlDsaKey};
use crate::note::SignatureLine;

/// Q = ⌊(N+f)/2⌋ + 1.
pub fn required(n: usize, f: usize) -> usize {
    (n + f) / 2 + 1
}

/// Corollary C: a quorum exists without the f liars iff N ≥ 3f + 1.
#[expect(clippy::int_plus_one, reason = "written as Corollary C states it")]
pub fn live(n: usize, f: usize) -> bool {
    n >= 3 * f + 1
}

/// Why a witness set is not usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumError {
    /// Q > N: no checkpoint could ever be accepted.
    Unreachable { n: usize, f: usize, q: usize },
    /// Two witnesses with one name and key ID, or one public key.
    DuplicateWitness,
}

/// The configured witnesses and the tolerance f.
#[derive(Debug, Clone)]
pub struct Witnesses {
    keys: Vec<MlDsaKey>,
    f: usize,
}

/// The count over one checkpoint's cosignatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Distinct witnesses with at least one valid cosignature.
    pub valid: usize,
    /// Lines from a known witness that failed to verify.
    pub invalid: usize,
    /// Lines from no configured witness (ignored, as signed-note requires).
    pub unknown: usize,
    /// Q for this set.
    pub required: usize,
    /// N ≥ 3f + 1.
    pub live: bool,
}

impl Tally {
    /// Accept iff the distinct valid witnesses reach Q.
    pub fn accepted(&self) -> bool {
        self.valid >= self.required
    }
}

impl Witnesses {
    pub fn new(keys: Vec<MlDsaKey>, f: usize) -> Result<Witnesses, QuorumError> {
        let (n, q) = (keys.len(), required(keys.len(), f));
        if q > n {
            return Err(QuorumError::Unreachable { n, f, q });
        }
        // One party, one vote: no (name, key ID) twice, no public key twice.
        let publics: Vec<Vec<u8>> = keys.iter().map(MlDsaKey::public_key).collect();
        for i in 0..n {
            for j in i + 1..n {
                let same_label = keys[i].name == keys[j].name && keys[i].id == keys[j].id;
                if same_label || publics[i] == publics[j] {
                    return Err(QuorumError::DuplicateWitness);
                }
            }
        }
        Ok(Witnesses { keys, f })
    }

    /// N.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether no witness is configured.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Q for this set.
    pub fn required(&self) -> usize {
        required(self.keys.len(), self.f)
    }

    /// Count the cosignature lines over `checkpoint`.
    pub fn tally(&self, checkpoint: &Checkpoint, cosignatures: &[SignatureLine]) -> Tally {
        // Which witnesses have a valid cosignature: a set, so a witness
        // counts once however many lines it sent.
        let mut cosigned = alloc::vec![false; self.keys.len()];
        let (mut invalid, mut unknown) = (0, 0);
        for line in cosignatures {
            match self.keys.iter().position(|k| k.names(line)) {
                None => unknown += 1,
                Some(i) => match self.keys[i].verify(checkpoint, &line.signature) {
                    Some(_) => cosigned[i] = true,
                    None => invalid += 1,
                },
            }
        }
        Tally {
            valid: cosigned.iter().filter(|&&c| c).count(),
            invalid,
            unknown,
            required: self.required(),
            live: live(self.keys.len(), self.f),
        }
    }
}
