//! `calha` (Brazilian-Portuguese: gutter/channel) — drives the
//! restart-required half of a config hot-swap change through a normal
//! rolling update, gated by `outorga`'s shadow-first promotion policy,
//! structurally walled to the DISCOVERED config tier. Design tier per
//! `theory/CALHA.md`; this crate carries the M2-M4 controller work.

pub mod controller;
pub mod crd;
pub mod runtime;
pub mod watermark;
