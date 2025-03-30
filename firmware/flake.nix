{
  description = "sl1-firmware flake";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };
  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      perSystem = {pkgs, ...}: let
        alias = pkgs.writeShellScriptBin;
      in {
        devShells.default = pkgs.mkShell rec {
          nativeBuildInputs = [
            (alias "run" ''cargo run'')
            (alias "build" ''cargo build'')
            (alias "build-release" ''cargo build --release'')
            (alias "flash" ''espflash flash ./target/riscv32imc-unknown-none-elf/debug/sl1'')
            (alias "flash-release" ''espflash flash ./target/riscv32imc-unknown-none-elf/release/sl1'')
            (alias "setup-rust" ''${pkgs.rustup}/bin/rustup toolchain install stable --component rust-src && ${pkgs.rustup}/bin/rustup target add riscv32imc-unknown-none-elf'')
          ];

          buildInputs = with pkgs; [
            rustup
            cargo
            cargo-udeps

            espflash
          ];

          shellHook =
            # sh
            ''
              export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib.outPath}/lib:$LD_LIBRARY_PATH"
              export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath buildInputs}:$LD_LIBRARY_PATH"
              echo "Set up environment."
            '';
        };
      };
      imports = [];
      flake = {};
    };
}
