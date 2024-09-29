{
  description = "Rust dev env";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
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
      packages.kmouse = 
          with pkgs; rustPlatform.buildRustPackage {
            pname = "kmouse";
            version = "0.0.1";

            src = ./.;

            cargoSha256 = "sha256-XKe0WZ6qaLdEdspEB+WZ9cGx7lAjd6adegVWKF559qI=";

            nativeBuildInputs = [ pkg-config mold];
            buildInputs = [
              openssl
            ];

            LIBCLANG_PATH = pkgs.lib.makeLibraryPath
              [ pkgs.llvmPackages_latest.libclang.lib ];

            meta = with lib; {
              description =
                "mouse simulator";
              homepage = "https://github.com/PanAeon/kmouse";
              license = licenses.mit;
            };
          };#) { };
        packages.default = packages.kmouse;
        apps.kmouse = flake-utils.lib.mkApp { drv = packages.kmouse; exePath = "/bin/kmouse"; };
        apps.default = apps.kmouse;
      });

}
