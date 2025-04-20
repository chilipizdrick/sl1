{
  lib,
  rustPlatform,
  makeDesktopItem,
  copyDesktopItems,
  makeWrapper,
  pkg-config,
  wayland,
  libxkbcommon,
  vulkan-loader,
}:
rustPlatform.buildRustPackage rec {
  pname = "sl1-desktop";
  name = "sl1-desktop";
  version = "0.1.0";
  cargoLock.lockFile = ./Cargo.lock;
  src = lib.cleanSource ./.;

  nativeBuildInputs = [
    copyDesktopItems
    makeWrapper
    pkg-config
  ];

  buildInputs = [
    wayland
    libxkbcommon
    vulkan-loader
  ];

  desktopItems = [
    (makeDesktopItem {
      name = "xyz.chilipizdrick.sl1-desktop";
      desktopName = "Smart Lights (sl1)";
      comment = "Desktop app for controlling smart sl1 devices";
      icon = "xyz.chilipizdrick.sl1-desktop";
      exec = pname;
      terminal = false;
      keywords = [
        "sl1"
        "SL1"
      ];
      startupWMClass = "xyz.chilipizdrick.sl1-desktop";
    })
  ];

  postFixup = let
    libPathWayland = lib.makeLibraryPath [
      wayland
      vulkan-loader
      libxkbcommon
    ];
  in ''
    rpath=$(patchelf --print-rpath $out/bin/sl1-desktop)
    patchelf --set-rpath "$rpath:${libPathWayland}" $out/bin/sl1-desktop
  '';

  postInstall = ''
    install -Dm644 assets/icons/hicolor/128x128/apps/xyz.chilipizdrick.sl1-desktop.png \
      $out/assets/icons/hicolor/128x128/apps/xyz.chilipizdrick.sl1-desktop.png
  '';
}
