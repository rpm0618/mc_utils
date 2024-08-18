{
    description = "Minecraft Utilities";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        rust-overlay.url = "github:oxalica/rust-overlay";
        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
        flake-utils.lib.eachDefaultSystem (system:
            let
                overlays = [ (import rust-overlay) ];
                pkgs = import nixpkgs {
                    inherit system overlays;
                };
                rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
                rustPlatform = pkgs.makeRustPlatform {
                    cargo = rustToolchain;
                    rustc = rustToolchain;
                };
                buildInputs = with pkgs; [
                    alsa-lib.dev
                    udev.dev

                    xorg.libX11
                    xorg.libXcursor
                    xorg.libXrandr
                    xorg.libXi
                    kdialog

                    libxkbcommon
                    libGL

                    makeWrapper
                ];
                nativeBuildInputs = with pkgs; [
                    pkg-config
                ];
                libraryPath = pkgs.lib.makeLibraryPath buildInputs;
            in rec {
                packages = {
                    viewer = rustPlatform.buildRustPackage {
                        name = "viewer";
                        src = self;
                        nativeBuildInputs = nativeBuildInputs;
                        buildInputs = buildInputs;
                        cargoLock = { lockFile = ./Cargo.lock; };
                        dontWrapQtApps = true;
                        postInstall = ''
                            wrapProgram $out/bin/viewer --set LD_LIBRARY_PATH "${libraryPath}"
                        '';
                    };
                };
                defaultPackage = packages.viewer;
                devShells.default = pkgs.mkShell rec {
                    name = "mc_utils";
                    packages = buildInputs ++ nativeBuildInputs ++ [ rustToolchain ];
                    LD_LIBRARY_PATH = libraryPath;
               };
            }
        );
}
