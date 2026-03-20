{ lib, rustPlatform, pkg-config }:

rustPlatform.buildRustPackage {
  pname = "glance-anki";
  version = "0.1.0";

  src = ../.;

  cargoLock.lockFile = ../Cargo.lock;

  # rusqlite's "bundled" feature compiles SQLite from source — only a C
  # compiler is needed, not a system libsqlite3.
  nativeBuildInputs = [ pkg-config ];

  meta = {
    description = "Glance widget showing Anki review activity as a contribution-style heatmap";
    homepage = "https://github.com/nickthegroot/glance-anki";
    license = lib.licenses.mit;
    mainProgram = "glance-anki";
  };
}
