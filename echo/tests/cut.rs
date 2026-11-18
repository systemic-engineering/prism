//! P3, the cut. Reflection seam §2.1 (mirror `4f6bdc17`): the seam is the
//! first line that is exactly `---`, the bytes `2d 2d 2d 0a`, at offset 0 or
//! right after an LF; only the first one cuts.

use echo::cut;

type Parts<'a> = (&'a [u8], Option<(&'a [u8], usize)>);

fn parts(s: &[u8]) -> Parts<'_> {
    let c = cut(s);
    (c.above, c.reflection.map(|r| (r.bytes, r.seam_line)))
}

fn unreflected(s: &[u8]) -> Parts<'_> {
    (s, None)
}

#[test]
fn a_file_with_no_seam_is_unreflected() {
    assert_eq!(parts(b"a\nb\n"), unreflected(b"a\nb\n"));
    assert_eq!(parts(b""), unreflected(b""));
}

#[test]
fn the_seam_at_offset_zero_leaves_an_empty_observation() {
    assert_eq!(
        parts(b"---\nfact p\n"),
        (&b""[..], Some((&b"fact p\n"[..], 1)))
    );
}

#[test]
fn the_seam_right_after_an_lf() {
    assert_eq!(
        parts(b"a\nb\n---\nfact p\n"),
        (&b"a\nb\n"[..], Some((&b"fact p\n"[..], 3)))
    );
}

#[test]
fn the_seam_as_the_last_line_gives_a_present_empty_reflection() {
    assert_eq!(parts(b"a\n---\n"), (&b"a\n"[..], Some((&b""[..], 2))));
}

#[test]
fn only_the_first_seam_cuts() {
    assert_eq!(
        parts(b"a\n---\nb\n---\nc\n"),
        (&b"a\n"[..], Some((&b"b\n---\nc\n"[..], 2)))
    );
}

#[test]
fn a_trailing_space_is_not_the_seam() {
    assert_eq!(parts(b"a\n--- \nb\n"), unreflected(b"a\n--- \nb\n"));
    // and the real seam after it still cuts
    assert_eq!(
        parts(b"--- \n---\nr\n"),
        (&b"--- \n"[..], Some((&b"r\n"[..], 2)))
    );
}

#[test]
fn near_misses_are_not_the_seam() {
    for s in [
        &b" ---\n"[..],
        b"----\n",
        b"--\n",
        b"-- -\n",
        b"x---\n",
        b"a\nx---\n",
        b"a ---\n",
    ] {
        assert_eq!(parts(s), unreflected(s), "{:?}", String::from_utf8_lossy(s));
    }
}

/// The seam's bytes are `2d 2d 2d 0a`: a CR before the LF makes the line
/// `---\r`, which isn't exactly `---`. (Open question for Mara: does a CRLF
/// file have a seam? docs/spec/echo.md §9 Q1.)
#[test]
fn crlf_is_not_the_seam() {
    assert_eq!(
        parts(b"a\r\n---\r\nb\r\n"),
        unreflected(b"a\r\n---\r\nb\r\n")
    );
}

/// `---` at the end of the file with no LF is not `2d 2d 2d 0a`. (Open
/// question for Mara: docs/spec/echo.md §9 Q2.)
#[test]
fn three_hyphens_at_eof_without_their_lf_do_not_cut() {
    assert_eq!(parts(b"a\n---"), unreflected(b"a\n---"));
    assert_eq!(parts(b"---"), unreflected(b"---"));
}
