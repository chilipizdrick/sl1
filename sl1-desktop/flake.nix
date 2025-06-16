{
  description = "sl1-desktop nix flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      perSystem = {pkgs, ...}: {
        packages = rec {
          sl1-desktop = pkgs.callPackage ./package.nix {};
          default = sl1-desktop;
        };

        devShells.default = pkgs.mkShell rec {
          buildInputs = with pkgs; [
            pkg-config
            wayland
            libxkbcommon
            vulkan-loader
          ];

          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
        };
      };
      imports = [];
      flake = {};
    };
}
