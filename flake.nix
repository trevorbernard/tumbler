{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11-small";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    ...
  }: let
    supportedSystems = [
      "x86_64-linux"
      "aarch64-darwin"
    ];
    forEachSupportedSystem = f:
      nixpkgs.lib.genAttrs supportedSystems (
        system:
          f {
            pkgs = import nixpkgs {
              inherit system;
              overlays = [rust-overlay.overlays.default];
            };
          }
      );
  in {
    formatter = forEachSupportedSystem ({pkgs}: pkgs.alejandra);

    packages = forEachSupportedSystem (
      {pkgs}: let
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };
        # shortRev is absent on dirty trees; dirtyShortRev carries a "-dirty" suffix
        gitShortSha = nixpkgs.lib.removeSuffix "-dirty" (self.shortRev or (self.dirtyShortRev or ""));
        default = pkgs.callPackage ./default.nix {inherit rustPlatform gitShortSha;};

        muslPkgs = pkgs.pkgsCross.musl64;
        rustWithMusl = rust.override {
          extensions = [];
          targets = ["x86_64-unknown-linux-musl"];
        };
        muslRustPlatform = muslPkgs.makeRustPlatform {
          cargo = rustWithMusl;
          rustc = rustWithMusl;
        };
        static = (muslPkgs.callPackage ./default.nix {
          rustPlatform = muslRustPlatform;
          inherit gitShortSha;
        }).overrideAttrs (_: {
          RUSTFLAGS = "-C target-feature=+crt-static --remap-path-prefix=/nix/store=/build";
          stripAllList = ["bin"];
        });
      in
        {inherit default static;}
        // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          dockerImage = pkgs.dockerTools.buildLayeredImage {
            name = "tumbler";
            tag = "latest";
            contents = [static];
            config = {
              Entrypoint = ["/bin/tumbler"];
            };
          };
        }
    );

    devShells = forEachSupportedSystem (
      {pkgs}: let
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in {
        default = pkgs.mkShell {
          nativeBuildInputs = [
            rust
            pkgs.cargo-audit
            pkgs.cargo-nextest
            pkgs.coreutils # for sha256sum
            pkgs.git
            pkgs.pkg-config
          ];
          shellHook = ''
            echo "Rust $(rustc --version)"
            echo "Cargo $(cargo --version)"
          '';
        };
      }
    );
  };
}
