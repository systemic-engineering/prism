//! prismqueer's integration tests: one binary, one module per file.
//!
//! One binary links once and is assessed by macOS once on first launch;
//! 51 binaries linked 51 times and paid ~2.5s each (Taut, 2026-11-16).
//! `autotests = false` in Cargo.toml, so a new `tests/*.rs` runs only once
//! it is declared here; `suite_manifest` fails until it is.
//!
//! Run one file's tests: `cargo test --features lapack,fate --test suite music::`.

mod agreement;
mod algedonic;
mod audit;
mod autopoietic_loop_integration;
mod band;
mod brain;
mod build_hygiene;
mod bundle_integration;
mod crossing;
mod declaration_round_trip;
mod dependency;
mod determinism;
mod discoball;
mod fate_composition;
mod field;
mod field_faces;
mod field_multiplicity;
mod field_prism;
mod flux_ouroboros;
mod gate_vectors;
mod hodge;
mod hypothesis;
mod identity_vectors;
mod instrument;
mod integration;
mod kleinos;
mod lambda_integration;
mod loss_laws;
mod metrix;
mod music;
mod no_clock;
mod one_graph;
mod optics_integration;
mod pbt_surface;
mod poly;
mod poly_factor;
mod poly_smith;
mod port;
mod pq;
mod pq_schema;
mod prism_laws;
mod progression;
mod red_narcissus_battery_five_op_collapse;
mod red_spectral_compose_paper_3_6_four_properties;
mod red_trust_chain_flux_void;
mod room;
mod room_faces;
mod room_holes;
mod room_rings;
mod rotation_splinter;
mod ruler_vectors;
mod search;
mod spec_vectors;
mod spectral_uuid;
mod stable;
mod steer;
mod substrate_ref;
mod suite_manifest;
mod trajectory;
mod verdict_composition;
mod window;
