{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.nci.url = "github:yusdacra/nix-cargo-integration";
  inputs.nci.inputs.nixpkgs.follows = "nixpkgs";
  inputs.parts.url = "github:hercules-ci/flake-parts";
  inputs.parts.inputs.nixpkgs-lib.follows = "nixpkgs";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = inputs@{ parts, nci, flake-utils, ... }:
    parts.lib.mkFlake { inherit inputs; } {
      systems = flake-utils.lib.defaultSystems;
      imports = [ nci.flakeModule ./crates.nix ];
      perSystem = { pkgs, config, ... }:
        let
          # shorthand for accessing this crate's outputs
          # you can access crate outputs under `config.nci.outputs.<crate name>` (see documentation)
          crateOutputs = config.nci.outputs."hydrus-ai-tagger";
        in {
          # export the crate devshell as the default devshell
          devShells.default = crateOutputs.devShell;
          # export the release package of the crate as default package
          packages.default = crateOutputs.packages.release;
        };
    };
}
