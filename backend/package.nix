{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "taiyaq-backend";
  version = "0.1.0";

  src =
    let
      fs = lib.fileset;
    in
    fs.toSource {
      root = ./.;
      fileset = fs.difference ./. (
        fs.unions [
          (fs.maybeMissing ./result)
        ]
      );
    };

  cargoDeps = rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };

  doCheck = false;

  meta = {
    license = with lib.licenses; [
      mit
      asl20
    ];
  };
}
