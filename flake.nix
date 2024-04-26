{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs = {
      nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, systems, fenix }:
    let
      forEachSystem = nixpkgs.lib.genAttrs (import systems);
    in {
      devShells = forEachSystem (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          lib = nixpkgs.lib;
          fenixpkgs = fenix.packages.${system};
        in
        {
          default = pkgs.mkShell rec {
            buildInputs = with pkgs; [ 
              wayland 
              libxkbcommon 
              libGL 
              vulkan-loader 
              xorg.libX11 
              xorg.libxcb 
              xorg.libXcursor
              xorg.libXrandr
              xorg.libXi
              xorg.libX11
              wasm-pack
              wasm-bindgen-cli
              binaryen
              simple-http-server
            ];
            LD_LIBRARY_PATH = "${lib.makeLibraryPath buildInputs}";
            nativeBuildInputs = with pkgs; [
              pkg-config
              fenixpkgs.rust-analyzer
              (with fenixpkgs; combine [
                (fenixpkgs.stable.withComponents [
                  "cargo" "clippy" "rust-src" "rustc" "rustfmt"
                ])
                fenixpkgs.targets.wasm32-unknown-unknown.stable.rust-std
              ])
              gdbgui
            ];
          };
        });
    };
}
