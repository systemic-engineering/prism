//! The witness quorum (Mara's Lemma B, `research-ledger-design.md` §2.3).
//!
//! N witnesses, at most f dishonest; each honest witness cosigns only
//! extensions of what it cosigned before. A reader accepting a checkpoint
//! only with **Q = ⌊(N+f)/2⌋ + 1** distinct valid cosignatures can't be
//! shown two inconsistent checkpoints: two Q-sets share ≥ 2Q − N ≥ f + 1
//! witnesses, so an honest one cosigned both. Liveness (Corollary C) needs
//! N ≥ 3f + 1; it is reported, not required.
//!
//! **The AND rule, for witnesses** (bullet-up-008, Alex, 2026-11-16): a
//! witness is declared with two keys, Ed25519 (C2SP `tlog-cosignature`
//! 0x04) and ML-DSA-44 (0x06), and counts toward Q only if **both** its
//! cosignatures verify. The weaker scheme never decides alone, so forging
//! a quorum takes forging both schemes: the count is as strong as the
//! stronger of the two, until both fall.

use alloc::vec::Vec;

use crate::checkpoint::{Checkpoint, Ed25519CosignerKey, MlDsaKey};
use crate::note::{SignatureLine, Vkey, ED25519_COSIGNATURE, MLDSA44_COSIGNATURE};

/// Q = ⌊(N+f)/2⌋ + 1.
pub fn required(n: usize, f: usize) -> usize {
    (n + f) / 2 + 1
}

/// Corollary C: a quorum exists without the f liars iff N ≥ 3f + 1.
#[expect(clippy::int_plus_one, reason = "written as Corollary C states it")]
pub fn live(n: usize, f: usize) -> bool {
    n >= 3 * f + 1
}

/// Why a witness or a witness set is not usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuorumError {
    /// Q > N: no checkpoint could ever be accepted.
    Unreachable { n: usize, f: usize, q: usize },
    /// Two witnesses with one name and key ID, or one public key.
    DuplicateWitness,
    /// A witness not declared with exactly one Ed25519 (0x04) and one
    /// ML-DSA-44 (0x06) key under one name.
    WitnessKeys,
}

/// One witness: its Ed25519 and its ML-DSA-44 cosigner key, one name.
#[derive(Debug, Clone)]
pub struct Witness {
    ed25519: Ed25519CosignerKey,
    mldsa44: MlDsaKey,
}

impl Witness {
    /// Both keys, under one cosigner name.
    pub fn new(ed25519: Ed25519CosignerKey, mldsa44: MlDsaKey) -> Result<Witness, QuorumError> {
        if ed25519.name != mldsa44.name {
            return Err(QuorumError::WitnessKeys);
        }
        Ok(Witness { ed25519, mldsa44 })
    }

    /// From two vkeys, in either order: one of type 0x04, one of type 0x06.
    pub fn from_vkeys(a: &Vkey, b: &Vkey) -> Result<Witness, QuorumError> {
        let (ed, ml) = match (a.sig_type, b.sig_type) {
            (ED25519_COSIGNATURE, MLDSA44_COSIGNATURE) => (a, b),
            (MLDSA44_COSIGNATURE, ED25519_COSIGNATURE) => (b, a),
            _ => return Err(QuorumError::WitnessKeys),
        };
        let ed = Ed25519CosignerKey::from_vkey(ed).map_err(|_| QuorumError::WitnessKeys)?;
        let ml = MlDsaKey::from_vkey(ml).map_err(|_| QuorumError::WitnessKeys)?;
        Witness::new(ed, ml)
    }

    /// The cosigner name.
    pub fn name(&self) -> &str {
        &self.mldsa44.name
    }
}

/// The configured witnesses and the tolerance f.
#[derive(Debug, Clone)]
pub struct Witnesses {
    witnesses: Vec<Witness>,
    f: usize,
}

/// The count over one checkpoint's cosignatures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Distinct witnesses whose Ed25519 **and** ML-DSA-44 cosignatures
    /// both verified.
    pub valid: usize,
    /// Distinct witnesses with exactly one of the two valid (counted zero).
    pub partial: usize,
    /// Lines from a known witness key that failed to verify.
    pub invalid: usize,
    /// Lines from no configured witness key (ignored, as signed-note requires).
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

/// The AND rule for one witness: both, never either.
fn counts(ed25519: bool, mldsa44: bool) -> bool {
    ed25519 && mldsa44
}

impl Witnesses {
    pub fn new(witnesses: Vec<Witness>, f: usize) -> Result<Witnesses, QuorumError> {
        let (n, q) = (witnesses.len(), required(witnesses.len(), f));
        if q > n {
            return Err(QuorumError::Unreachable { n, f, q });
        }
        // One party, one vote: no (name, key ID) twice, no public key
        // twice, in either scheme.
        let ml: Vec<Vec<u8>> = witnesses.iter().map(|w| w.mldsa44.public_key()).collect();
        for i in 0..n {
            for j in i + 1..n {
                let (a, b) = (&witnesses[i], &witnesses[j]);
                let same_ed = (a.ed25519.name == b.ed25519.name && a.ed25519.id == b.ed25519.id)
                    || a.ed25519.public_key() == b.ed25519.public_key();
                let same_ml = (a.mldsa44.name == b.mldsa44.name && a.mldsa44.id == b.mldsa44.id)
                    || ml[i] == ml[j];
                if same_ed || same_ml {
                    return Err(QuorumError::DuplicateWitness);
                }
            }
        }
        Ok(Witnesses { witnesses, f })
    }

    /// N.
    pub fn len(&self) -> usize {
        self.witnesses.len()
    }

    /// Whether no witness is configured.
    pub fn is_empty(&self) -> bool {
        self.witnesses.is_empty()
    }

    /// Q for this set.
    pub fn required(&self) -> usize {
        required(self.witnesses.len(), self.f)
    }

    /// Count the cosignature lines over `checkpoint`.
    pub fn tally(&self, checkpoint: &Checkpoint, cosignatures: &[SignatureLine]) -> Tally {
        // Per witness, which of its two schemes has a valid cosignature: a
        // set, so a witness counts once however many lines it sent.
        let n = self.witnesses.len();
        let (mut ed, mut ml) = (alloc::vec![false; n], alloc::vec![false; n]);
        let (mut invalid, mut unknown) = (0, 0);
        for line in cosignatures {
            let by_ed = self.witnesses.iter().position(|w| w.ed25519.names(line));
            let by_ml = self.witnesses.iter().position(|w| w.mldsa44.names(line));
            let verified = match (by_ed, by_ml) {
                (Some(i), _) => self.witnesses[i]
                    .ed25519
                    .verify(checkpoint, &line.signature)
                    .map(|_| &mut ed[i]),
                (None, Some(i)) => self.witnesses[i]
                    .mldsa44
                    .verify(checkpoint, &line.signature)
                    .map(|_| &mut ml[i]),
                (None, None) => {
                    unknown += 1;
                    continue;
                }
            };
            match verified {
                Some(seen) => *seen = true,
                None => invalid += 1,
            }
        }
        let valid = (0..n).filter(|&i| counts(ed[i], ml[i])).count();
        let partial = (0..n).filter(|&i| ed[i] != ml[i]).count();
        Tally {
            valid,
            partial,
            invalid,
            unknown,
            required: self.required(),
            live: live(n, self.f),
        }
    }
}
