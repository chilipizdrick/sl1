{
  description = "sl1 nix flake";

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
          buildInputs = with pkgs; [
            rustup
            cargo
            cargo-udeps
            pkg-config

            espflash

            wayland
            libxkbcommon
            vulkan-loader

            (alias "flash" ''espflash flash ./target/riscv32imc-unknown-none-elf/debug/sl1'')
            (alias "flash-release" ''espflash flash ./target/riscv32imc-unknown-none-elf/release/sl1'')
            (alias "setup-rust" ''
              ${pkgs.rustup}/bin/rustup toolchain install stable --component rust-src && \
              ${pkgs.rustup}/bin/rustup target add riscv32imc-unknown-none-elf
            '')
            (alias "erase-flash" ''espflash erase-flash'')
          ];

          LD_LIBRARY_PATH = "${pkgs.stdenv.cc.cc.lib.outPath}/lib:${pkgs.lib.makeLibraryPath buildInputs}";
        };
      };
      imports = [];
      flake = {};
    };
}
