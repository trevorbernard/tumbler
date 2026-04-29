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
      in
        {inherit default;}
        // pkgs.lib.optionalAttrs pkgs.stdenv.isLinux {
          dockerImage = pkgs.dockerTools.buildLayeredImage {
            name = "tumbler";
            tag = "latest";
            contents = [default];
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
