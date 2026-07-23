# calha

> Calha (Brazilian-Portuguese, gutter/channel) -- a Kubernetes controller that drives the restart-required half of a config-hot-swap change through a normal rolling update, gated by formigueiro's outorga shadow-first promotion policy, structurally walled to the DISCOVERED config tier (never touches git/Flux-owned config). Composes shikumi::ConfigStore/resolve_progressive/Provenance + breathe-provider's ConfigReload/DisruptionClass shape + formigueiro::outorga::PromotionPolicy. Design tier per [theory/CALHA.md](https://github.com/pleme-io/theory/blob/main/CALHA.md); this repo carries the M2-M4 controller work (CalhaPolicy CRD, plan_tick, the controller binary).

## Status (tier-honest, never rounded up)

- **`src/crd.rs`, `src/runtime.rs`** -- real, tested. `plan_tick` is a pure,
  zero-I/O decision function (7 unit tests, all green) composing
  `outorga::PromotionPolicy` exactly as `theory/CALHA.md` §6/§14.2 designs it.
- **`src/watermark.rs`, `src/controller.rs`** -- real, compiles clean, but
  **never run against a live cluster or a real target's `/healthz/config`
  endpoint.** The wire shape (`ConfigSyncProof`) is `calha`'s own mirror of
  what `shikumi::hotswap` (M1, unbuilt elsewhere) will eventually serve --
  there is no real producer of that endpoint anywhere in the fleet yet.
- **No `#[derive(HotSwap)]` exists.** That's `pleme-hotswap-derive` (a
  separate, also-new repo) plus a genuinely non-trivial extension to
  `tatara-rust-ast`'s `PerFieldDeriveSpec` emitter -- named, scoped, and
  explicitly deferred; not started here.
- **No CI-verified `cargo clippy --all-targets -- -D warnings`, no `nix build`
  end-to-end**, beyond this session's local `cargo build`/`cargo test`. Watch
  this repo's own CI run on first push for that verification.
- **M0 (the doc's own recommended actual first step) is scoped to `logan`'s
  own repo, not this one** -- it needs zero code here.

## Building

```bash
nix run .#calha -- --help
```

## License

MIT.
