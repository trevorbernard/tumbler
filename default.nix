{
  pkgs,
  rustPlatform ? pkgs.rustPlatform,
}:
let
  cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
  src = pkgs.lib.cleanSource ./.;
  cargoLock = {
    lockFile = ./Cargo.lock;
  };
  nativeBuildInputs = [
    pkgs.pkg-config
  ];
  meta = with pkgs.lib; {
    description = "";
    homepage = "https://github.com/trevorbernard/tumbler";
    license = licenses.mit;
    maintainers = [
      {
        github = "trevorbernard";
        name = "Trevor Bernard";
        email = "trevor.bernard@pm.me";
      }
    ];
  };
}
