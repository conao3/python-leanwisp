{
  description = "Leanwisp: parser-checked Lean 4 whitespace formatting";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-darwin"
      ];
      imports = [ inputs.treefmt-nix.flakeModule ];
      perSystem = { pkgs, ... }: {
        treefmt = {
          projectRootFile = "flake.nix";
          programs.nixfmt.enable = true;
          programs.ruff-check.enable = true;
          programs.ruff-format.enable = true;
        };
        devShells.default = pkgs.mkShell {
          packages = [
            pkgs.python315
            pkgs.uv
            pkgs.lean4
            pkgs.gnumake
          ];
        };
        packages.default = pkgs.python315Packages.buildPythonApplication {
          pname = "leanwisp";
          version = "0.1.0";
          src = ./.;
          pyproject = true;
          build-system = [ pkgs.python315Packages.hatchling ];
          nativeBuildInputs = [ pkgs.makeWrapper ];
          postFixup = "wrapProgram $out/bin/leanwisp --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.lean4 ]}";
          meta.mainProgram = "leanwisp";
          meta.license = pkgs.lib.licenses.asl20;
        };
      };
    };
}
