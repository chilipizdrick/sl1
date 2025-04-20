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
        packages = rec {
          sl1-desktop = pkgs.rustPlatform.buildRustPackage {
            name = "sl1-desktop";
            version = "0.1.0";
            cargoLock.lockFile = ./sl1-desktop/Cargo.lock;
            src = pkgs.lib.cleanSource ./sl1-desktop;

            buildCommand = ''
              mkdir -p $out/share/applications
              cp ${./sl1-desktop/assets/sl1-desktop.desktop} $out/share/applications
            '';

            postInstall = ''
              substituteInPlace $out/share/applications/sl1-desktop.desktop \
                --replace 'Exec=sl1-desktop' 'Exec=$out/bin/sl1-desktop'
            '';
          };
          default = sl1-desktop;
        };

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

            (alias "flash" ''espflash flash ./target/riscv32imc-unknown-none-elf/debug/sl1-firmware'')
            (alias "flash-release" ''espflash flash ./target/riscv32imc-unknown-none-elf/release/sl1-firmware'')
            (alias "setup-rust" ''
              ${pkgs.rustup}/bin/rustup toolchain install stable --component rust-src && \
              ${pkgs.rustup}/bin/rustup target add riscv32imc-unknown-none-elf
            '')
            (alias "erase-flash" ''espflash erase-flash'')
          ];

          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
        };
      };
      imports = [];
      flake = {};
    };
}
