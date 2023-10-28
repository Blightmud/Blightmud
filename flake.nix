{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      perSystem = { config, self', pkgs, lib, system, ... }:
        let
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

          runtimeDeps = with pkgs; [ alsa-lib openssl speechd ];
          buildDeps = with pkgs; [ pkg-config rustPlatform.bindgenHook ];
          devDeps = with pkgs; [ gdb asciinema ];

          withFeatures = features: {
            inherit (cargoToml.package) name version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            buildFeatures = features;
            nativeBuildInputs = buildDeps;
            buildInputs = runtimeDeps;
            doCheck = false; # Some tests require networking
          };

          mkDevShell = rustc:
            pkgs.mkShell {
              shellHook = ''
                export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
              '';
              buildInputs = runtimeDeps;
              nativeBuildInputs = buildDeps ++ devDeps ++ [ rustc ];
            };
        in {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ (import inputs.rust-overlay) ];
          };
          packages.default = self'.packages.blightmud-tts;
          devShells.default = self'.devShells.nightly;

          # Blightmud w/ text to speech enabled.
          packages.blightmud-tts =
            pkgs.rustPlatform.buildRustPackage (withFeatures "tts");
          # Blightmud w/o text to speech enabled.
          packages.blightmud =
            pkgs.rustPlatform.buildRustPackage (withFeatures "");

          # Nightly Rust dev env
          devShells.nightly = (mkDevShell (pkgs.rust-bin.selectLatestNightlyWith
            (toolchain: toolchain.default)));
          # Stable Rust dev env
          devShells.stable = (mkDevShell pkgs.rust-bin.stable.latest.default);
        };
    };
}
