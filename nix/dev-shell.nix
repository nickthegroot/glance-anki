{ mkShell, rust-analyzer, clippy, rustfmt, glance-anki }:

mkShell {
  inputsFrom = [ glance-anki ];
  packages = [ rust-analyzer clippy rustfmt ];
}
