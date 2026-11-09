{
  description = "Ball simulator: satisfying ball-bounce video generator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer
            ffmpeg
          ];
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          # The preview window (minifb, X11 backend) dlopens these at runtime.
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
            libX11
            libXcursor
          ]);
        };
      });
}
