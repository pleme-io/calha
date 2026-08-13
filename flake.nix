{
  description = "Calha (Brazilian-Portuguese, gutter/channel) -- a Kubernetes controller that drives the restart-required half of a config-hot-swap change through a normal rolling update, gated by formigueiro's outorga shadow-first promotion policy, structurally walled to the DISCOVERED config tier (never touches git/Flux-owned config). Composes shikumi::ConfigStore/resolve_progressive/Provenance + breathe-provider's ConfigReload/DisruptionClass shape + formigueiro::outorga::PromotionPolicy + engenho_controllers::Controller. Design tier per theory/CALHA.md; this repo carries the M2-M4 controller work (CalhaPolicy CRD, plan_tick, the controller binary).";
  # Regenerated shape, 2026-08-13. As originally rendered by repo-forge, the
  # outputs lambda destructured `crate2nix` and `devenv` while `inputs`
  # declared neither, so `nix flake check` died on
  # `cannot find flake 'flake:crate2nix' in the flake registries` — step one
  # of this repo's own ci.yml. `calha` and `masume` were the two repos born
  # with it.
  #
  # `devenv` is dropped rather than declared: substrate's helper takes it as
  # `devenv ? null` and its documented call site does not pass it.
  #
  # Fixed at the generator in repo-forge@7d1ad35, where the inputs list and
  # the outputs lambda were two independent statements of one set, now gated
  # by `every_name_the_outputs_lambda_binds_is_a_declared_input`.
  inputs = {
    nixpkgs = {
      follows = "substrate/nixpkgs";
    };
    crate2nix = {
      url = "github:nix-community/crate2nix";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    substrate = {
      url = "github:pleme-io/substrate";
    };
  };
  outputs = inputs @ { self, nixpkgs, crate2nix, flake-utils, substrate, ... }:
    (import "${substrate}/lib/rust-tool-release-flake.nix" {
      inherit nixpkgs crate2nix flake-utils;
    }) {
      toolName = "calha";
      src = self;
      repo = "pleme-io/calha";
    };
}
