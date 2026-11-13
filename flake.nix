{
  description = "Leanwisp: Rust source formatter for Lean 4";
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
      perSystem =
        { pkgs, ... }:
        let
          syntax = pkgs.stdenv.mkDerivation {
            pname = "leanwisp-syntax";
            version = "0.2.0";
            src = ./lean;
            nativeBuildInputs = [
              pkgs.lean4
              pkgs.makeWrapper
            ];
            buildPhase = "export HOME=$TMPDIR; lake build";
            installPhase = ''
              install -Dm755 .lake/build/bin/leanwisp-syntax $out/bin/leanwisp-syntax
              wrapProgram $out/bin/leanwisp-syntax --set LEAN_SYSROOT ${pkgs.lean4}
            '';
          };
        in
        {
          treefmt = {
            projectRootFile = "flake.nix";
            programs.nixfmt.enable = true;
            programs.rustfmt.enable = true;
          };
          devShells.default = pkgs.mkShell {
            packages = [
              pkgs.cargo
              pkgs.rustc
              pkgs.clippy
              pkgs.rustfmt
              pkgs.rust-analyzer
              pkgs.lean4
              pkgs.gnumake
            ];
            LEANWISP_SYNTAX = "${syntax}/bin/leanwisp-syntax";
          };
          packages.default = pkgs.rustPlatform.buildRustPackage {
            pname = "leanwisp";
            version = "0.2.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            nativeBuildInputs = [ pkgs.makeWrapper ];
            nativeCheckInputs = [ pkgs.lean4 ];
            LEANWISP_SYNTAX = "${syntax}/bin/leanwisp-syntax";
            postInstall = "wrapProgram $out/bin/leanwisp --set LEANWISP_SYNTAX ${syntax}/bin/leanwisp-syntax --prefix PATH : ${
              pkgs.lib.makeBinPath [ pkgs.lean4 ]
            }";
            meta = {
              mainProgram = "leanwisp";
              license = pkgs.lib.licenses.asl20;
            };
          };
        };
    };
}
