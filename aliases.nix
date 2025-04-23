pkgs: let
  alias = pkgs.writeShellScriptBin;
in [
  (alias "run-esp32" ''
    cargo run --target xtensa-esp32-none-elf --no-default-features --features esp32
  '')
  (alias "run-esp32c3" ''
    cargo run --target riscv32imc-unknown-none-elf --no-default-features --features esp32c3
  '')

  (alias "build-esp32" ''
    cargo build --target xtensa-esp32-none-elf --no-default-features --features esp32
  '')
  (alias "build-esp32c3" ''
    cargo build --target riscv32imc-unknown-none-elf --no-default-features --features esp32c3
  '')

  (alias "build-release-esp32" ''
    RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo build \
      --release --target xtensa-esp32-none-elf --no-default-features --features esp32
  '')
  (alias "build-release-esp32c3" ''
    RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo build \
      --release --target riscv32imc-unknown-none-elf --no-default-features --features esp32c3
  '')

  (alias "flash-esp32" ''espflash flash ./target/xtensa-esp32-none-elf/debug/sl1-firmware'')
  (alias "flash-esp32c3" ''espflash flash ./target/riscv32imc-unknown-none-elf/debug/sl1-firmware'')

  (alias "flash-release-esp32" ''espflash flash ./target/xtensa-esp32-none-elf/release/sl1-firmware'')
  (alias "flash-release-esp32c3" ''espflash flash ./target/riscv32imc-unknown-none-elf/release/sl1-firmware'')

  (alias "erase-flash" ''espflash erase-flash'')

  (alias "setup-rust-esp32" ''
    espup install
    rustup override set esp
    chmod +x $HOME/export-esp.sh
    source $HOME/export-esp.sh
  '')
  (alias "setup-rust-esp32c3" ''
    rustup toolchain install nightly --component rust-src rust-analyzer \
      && rustup target add riscv32imc-unknown-none-elf \
      && rustup override set nightly
  '')
]
