{
  description = "Calha (Brazilian-Portuguese, gutter/channel) -- a Kubernetes controller that drives the restart-required half of a config-hot-swap change through a normal rolling update, gated by formigueiro's outorga shadow-first promotion policy, structurally walled to the DISCOVERED config tier (never touches git/Flux-owned config). Composes shikumi::ConfigStore/resolve_progressive/Provenance + breathe-provider's ConfigReload/DisruptionClass shape + formigueiro::outorga::PromotionPolicy + engenho_controllers::Controller. Design tier per theory/CALHA.md; this repo carries the M2-M4 controller work (CalhaPolicy CRD, plan_tick, the controller binary).";
  inputs = {
    nixpkgs = {
      follows = "substrate/nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    substrate = {
      url = "github:pleme-io/substrate";
    };
  };
  outputs = inputs @ { self, nixpkgs, crate2nix, flake-utils, substrate, devenv, ... }:
    (import "${substrate}/lib/rust-tool-release-flake.nix" {
      inherit nixpkgs crate2nix flake-utils devenv;
    }) {
      toolName = "calha";
      src = self;
      repo = "pleme-io/calha";
    };
}
