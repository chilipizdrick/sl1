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
    rpathWayland = lib.makeLibraryPath [
      wayland
      vulkan-loader
      libxkbcommon
    ];
  in ''
    rpath=$(patchelf --print-rpath $out/bin/sl1-desktop)
    patchelf --set-rpath "$rpath:${rpathWayland}" $out/bin/sl1-desktop
  '';

  postInstall = ''
    install -Dm644 assets/icons/hicolor/512x512/apps/xyz.chilipizdrick.sl1-desktop.png \
      $out/share/icons/hicolor/512x512/apps/xyz.chilipizdrick.sl1-desktop.png
    install -Dm644 assets/icons/hicolor/256x256/apps/xyz.chilipizdrick.sl1-desktop.png \
      $out/share/icons/hicolor/256x256/apps/xyz.chilipizdrick.sl1-desktop.png
    install -Dm644 assets/icons/hicolor/128x128/apps/xyz.chilipizdrick.sl1-desktop.png \
      $out/share/icons/hicolor/128x128/apps/xyz.chilipizdrick.sl1-desktop.png
    install -Dm644 assets/icons/hicolor/64x64/apps/xyz.chilipizdrick.sl1-desktop.png \
      $out/share/icons/hicolor/64x64/apps/xyz.chilipizdrick.sl1-desktop.png
  '';
}
