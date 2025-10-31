{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-parts.url = "github:hercules-ci/flake-parts";
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    services-flake.url = "github:juspay/services-flake";
    process-compose-flake.url = "github:Platonic-Systems/process-compose-flake";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{
      nixpkgs,
      flake-parts,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = with inputs; [
        treefmt-nix.flakeModule
        git-hooks-nix.flakeModule
        process-compose-flake.flakeModule
      ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];

      perSystem =
        {
          config,
          pkgs,
          lib,
          system,
          ...
        }:
        let
          rust-toolchain = pkgs.rust-bin.stable.latest.default;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = rust-toolchain;
            rustc = rust-toolchain;
          };

          backup = pkgs.writeShellScriptBin "backup" ''
            mkdir -p backups
            while true; do
              cp data.json backups/data_$(date +%Y%m%d_%H%M).json
              sleep 600
            done
          '';
        in
        rec {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          packages.taiyaq-backend = pkgs.callPackage ./backend/package.nix { inherit rustPlatform; };

          packages.default = packages.taiyaq-backend;

          devShells.default = pkgs.mkShell {
            inputsFrom = [
              config.pre-commit.devShell
              config.process-compose."app".services.outputs.devShell
            ];
            inherit (packages.default) buildInputs;
            inherit (packages.default) nativeBuildInputs;
            packages = with pkgs; [
              rust-toolchain
              nodejs
              pnpm_10
            ];
          };

          treefmt = {
            projectRootFile = "flake.nix";
            programs = {
              nixfmt.enable = true;
              rustfmt.enable = true;
              taplo.enable = true;
              prettier.enable = true;
            };

            settings.formatter = {
              taplo.options = [
                "fmt"
                "-o"
                "reorder_keys=true"
              ];
            };
          };

          pre-commit = {
            check.enable = true;
            settings = {
              hooks = {
                ripsecrets.enable = true;
                typos.enable = true;
                treefmt.enable = true;
                clippy = {
                  enable = true;
                  packageOverrides.cargo = rust-toolchain;
                  packageOverrides.clippy = rust-toolchain;
                  settings.extraArgs = "--manifest-path backend/Cargo.toml";
                };
              };
            };
          };

          process-compose."app" = {
            imports = [
              inputs.services-flake.processComposeModules.default
            ];

            cli.options.no-server = false;
            settings = {
              processes = {
                backend-server.command = lib.getExe config.packages.taiyaq-backend;
                backup.command = lib.getExe backup;
              };
            };
          };
        };
    };
}
