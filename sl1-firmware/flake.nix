{
  description = "sl1-firmware nix flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      perSystem = {pkgs, ...}: {
        devShells.default = let
          aliases = import ./aliases.nix pkgs;
        in
          pkgs.mkShell rec {
            buildInputs = with pkgs;
              [
                rustup
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
              ]
              ++ aliases;

            LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath buildInputs}";
          };
      };
      imports = [];
      flake = {};
    };
}
