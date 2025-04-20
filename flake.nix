{
  description = "sl1 nix flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      perSystem = {pkgs, ...}: {
        packages = rec {
          sl1-desktop = pkgs.callPackage ./desktop/package.nix {};
          default = sl1-desktop;
        };

        devShells.default = let
          alias = pkgs.writeShellScriptBin;
          aliases = [
            (alias "run-esp32" ''ESP_LOG=TRACE cargo run --target xtensa-esp32-none-elf --features esp32'')
            (alias "run-esp32c3" ''ESP_LOG=TRACE cargo run --target riscv32imc-unknown-none-elf --features esp32c3'')

            (alias "build-esp32" ''ESP_LOG=WARN cargo build --target xtensa-esp32-none-elf --features esp32'')
            (alias "build-esp32c3" ''ESP_LOG=WARN cargo build --target riscv32imc-unknown-none-elf --features esp32c3'')

            (alias "build-release-esp32" ''ESP_LOG=WARN RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo build --release --target xtensa-esp32-none-elf --features esp32'')
            (alias "build-release-esp32c3" ''ESP_LOG=WARN RUSTFLAGS="-Zlocation-detail=none -Zfmt-debug=none" cargo build --release --target riscv32imc-unknown-none-elf --features esp32c3'')

            (alias "flash-esp32" ''espflash flash ./target/xtensa-esp32-none-elf/debug/sl1-firmware'')
            (alias "flash-esp32c3" ''espflash flash ./target/riscv32imc-unknown-none-elf/debug/sl1-firmware'')

            (alias "flash-release-esp32" ''espflash flash ./target/xtensa-esp32-none-elf/release/sl1-firmware'')
            (alias "flash-release-esp32c3" ''espflash flash ./target/riscv32imc-unknown-none-elf/release/sl1-firmware'')

            (alias "erase-flash" ''espflash erase-flash'')

            (alias "setup-rust-esp32" ''
              ${pkgs.espup}/bin/espup install
              ${pkgs.rustup}/bin/rustup override set esp
              chmod +x $HOME/export-esp.sh
              source $HOME/export-esp.sh
            '')
            (alias "setup-rust-esp32c3" ''
              ${pkgs.rustup}/bin/rustup toolchain install stable --component rust-src && \
              ${pkgs.rustup}/bin/rustup target add riscv32imc-unknown-none-elf
            '')
          ];
        in
          pkgs.mkShell rec {
            buildInputs = with pkgs;
              [
                rustup
                cargo
                cargo-udeps
                pkg-config

                espflash
                espup
                ldproxy
                git
                wget
                flex
                bison
                gperf
                python3
                python3Packages.pip
                python3Packages.virtualenv
                cmake
                ninja
                ccache
                libffi
                openssl
                dfu-util
                libusb1
                libz

                wayland
                libxkbcommon
                vulkan-loader
              ]
              ++ aliases;

            LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";

            shellHook = ''
              source $HOME/export-esp.sh
            '';
          };
      };
      imports = [];
      flake = {};
    };
}
