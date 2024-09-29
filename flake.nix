{
  description = "Rust dev env";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
  inputs.flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, flake-compat }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
      in rec {
        devShell = with pkgs;
          mkShell {
            name = "rust-env";
            buildInputs = [
              (rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" ];
              })
              rustfmt
              clippy
              mold
              rust-analyzer
              pkg-config
              cargo-generate
              rust-bindgen
              udev
              libinput
              libevdev
              #curl
            ];
            LIBCLANG_PATH = pkgs.lib.makeLibraryPath
              [ pkgs.llvmPackages_latest.libclang.lib ];
            PKG_CONFIG_PATH =
              "${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.libxml2.dev}/lib/pkgconfig";
            shellHook = "exec fish";
          };
      packages.transgression-tui = #pkgs.callPackage (with pkgs;
          with pkgs; rustPlatform.buildRustPackage rec {
            pname = "kmouse";
            version = "0.0.1";

            src = ./.;

            /*src = fetchFromGitHub {
              owner = "panaeon";
              repo = pname;
              rev = "4adc304f8d398934b80b42648e2b6b9414581a0c";
              sha256 = "sha256-ijwI5ujuGneThN6mcJSSb6CqMiKRkvsqvUv0/GyNBjs=";
              fetchSubmodules = true;
            };*/

            cargoSha256 = "sha256-XKe0WZ6qaLdEdspEB+WZ9cGx7lAjd6adegVWKF559qI=";

            nativeBuildInputs = [ pkg-config mold];
            buildInputs = [
              openssl
            ];

            meta = with lib; {
              description =
                "A transgressive way to manage your transmission torrents in the terminal";
              homepage = "https://github.com/PanAeon/transg-tui";
              license = licenses.mit;
              maintainers = [ ];
            };
          };#) { };
        packages.default = packages.transgression-tui;
        apps.transg-tui = flake-utils.lib.mkApp { drv = packages.transgression-tui; exePath = "/bin/transgression-tui"; };
        apps.default = apps.transg-tui;
      });

}
