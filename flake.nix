{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };
  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.git-hooks-nix.flakeModule
        inputs.treefmt-nix.flakeModule
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
          ...
        }:
        {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ config.pre-commit.devShell ];
            packages = [
              pkgs.nodejs
              pkgs.pnpm
            ]
            ++ lib.optionals pkgs.stdenv.isLinux [ pkgs.chromium ];
            shellHook = lib.optionalString pkgs.stdenv.isLinux ''
              export PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH="${lib.getExe pkgs.chromium}"
            '';
          };
          treefmt = {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
          };
          pre-commit = {
            check.enable = true;
            settings.hooks = {
              ripsecrets.enable = true;
              typos.enable = true;
              treefmt.enable = true;
            };
          };
        };
    };
}
