{
  description = "sl1-iced-gui flake";
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
          ];

          buildInputs = with pkgs; [
            cargo
            wayland
            libxkbcommon
            vulkan-loader
            pkg-config
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
