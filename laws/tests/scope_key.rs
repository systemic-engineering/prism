//! A `ScopeKey` zeroes its bytes when it drops (prism-026 item 1).
//!
//! The test owns the stack slot the key lives in (a `MaybeUninit`), runs
//! the key's drop in place, and reads the slot's 32 bytes back through a raw
//! pointer. `ScopeKey` is `#[repr(transparent)]` over `[u8; 32]`, so the slot
//! is exactly the key's bytes, and they stay initialized: the read is sound.
//!
//! What this shows: the drop writes zeros over the key's own storage, and
//! the write is still there after the drop (not elided as a dead store in
//! this build). What no test can show: that no other copy exists. A move is
//! a memcpy that may leave the old bytes in a dead stack slot or a register;
//! the caller's source buffer is zeroed by `ScopeKey::take`, but a copy the
//! caller made before that is the caller's. That is why an erasure is the
//! holder's attested act (Sealed), never a proof (Law).

use std::mem::MaybeUninit;
use std::ptr;

use prism_laws::oid::{Oid, Scope, ScopeKey};

#[test]
fn a_key_is_zeroed_when_it_drops() {
    let mut slot = MaybeUninit::new(ScopeKey::take(&mut [0xc3; 32]).expect("held"));
    let at = slot.as_mut_ptr();
    // Before the drop, the slot holds the key: it hashes as the key would.
    // SAFETY: the slot was initialized just above and not yet dropped.
    let before = Oid::hash(&Scope::keyed(unsafe { &*at }), b"x");
    let expected = Oid::hash(
        &Scope::keyed(&ScopeKey::take(&mut [0xc3; 32]).expect("held")),
        b"x",
    );
    assert_eq!(before, expected);
    // SAFETY: `at` points at an initialized key we own; it is dropped once,
    // here, and never used as a key again.
    unsafe { ptr::drop_in_place(at) };
    // SAFETY: the slot is our stack memory, still live; `ScopeKey` is
    // repr(transparent) over [u8; 32], so its 32 bytes are initialized u8s.
    let after: [u8; 32] = unsafe { ptr::read(at.cast::<[u8; 32]>()) };
    assert_eq!(after, [0; 32], "the key's bytes are zeroed on drop");
}
