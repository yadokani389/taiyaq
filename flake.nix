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

          databasePath = "data/taiyaq.sqlite";
          databaseUrl = "sqlite://${databasePath}";

          backup = pkgs.writeShellScriptBin "backup" ''
            mkdir -p backups
            while true; do
              db_path=''${DATABASE_URL#sqlite://}
              db_path=''${db_path:-${databasePath}}
              ${pkgs.sqlite}/bin/sqlite3 "$db_path" ".backup 'backups/taiyaq_$(date +%Y%m%d_%H%M).sqlite'"
              sleep 600
            done
          '';
        in
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
          };

          packages.taiyaq-backend = pkgs.callPackage ./backend/package.nix { inherit rustPlatform; };

          packages.default = config.packages.taiyaq-backend;

          devShells.default = pkgs.mkShell {
            inputsFrom = [
              config.pre-commit.devShell
              config.process-compose."app".services.outputs.devShell
            ];
            inherit (config.packages.default) buildInputs;
            inherit (config.packages.default) nativeBuildInputs;
            packages = with pkgs; [
              rust-toolchain
              nodejs
              pnpm_10
              sqlx-cli
              process-compose
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
                typos = {
                  enable = true;
                  settings.ignored-words = [ "styl" ];
                };
                treefmt.enable = true;
              };
            };
          };

          checks.clippy-backend = inputs.git-hooks-nix.lib.${system}.run {
            src = ./backend;
            settings.rust = {
              cargoManifestPath = "Cargo.toml";
              check.cargoDeps = config.packages.taiyaq-backend.cargoDeps;
            };
            hooks.clippy = {
              enable = true;
              packageOverrides.cargo = rust-toolchain;
              packageOverrides.clippy = rust-toolchain;
              settings.denyWarnings = true;
              extraPackages = with pkgs; [
                openssl
                pkg-config
                sqlx-cli
              ];
            };
          };

          process-compose."app" = {
            imports = [
              inputs.services-flake.processComposeModules.default
            ];

            cli.options.no-server = false;
            settings = {
              processes = {
                backend-server.command = "DATABASE_URL=${databaseUrl} ${lib.getExe config.packages.taiyaq-backend}";
                backup.command = "DATABASE_URL=${databaseUrl} ${lib.getExe backup}";
                staff-panel.command = "${lib.getExe pkgs.serve} -s frontend/staff-panel/dist -l 38001";
                display-screen.command = "${lib.getExe pkgs.serve} -s frontend/display-screen/dist -l 38002";
                user-display.command = "${lib.getExe pkgs.serve} -s frontend/user-display/dist -l 38003";
              };
            };
          };
        };
    };
}
