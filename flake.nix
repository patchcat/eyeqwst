{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
    fenix.url = "github:nix-community/fenix/monthly";
    fenix.inputs = { nixpkgs.follows = "nixpkgs"; };
    nixgl.url = "github:nix-community/nixGL";
  };

  outputs = { self, nixpkgs, systems, nixgl, fenix }:
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
      packages = forEachSystem (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ nixgl.overlay ];
          };
        in
        rec {
          default = pkgs.rustPlatform.buildRustPackage rec {
            pname = "eyeqwst";
            version = "0.0.1";

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            doCheck = false;

            postFixup = ''
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.wayland}/lib/libwayland-client.so
              # patchelf $out/bin/eyeqwst --add-needed ${pkgs.libxkbcommon}/lib/libxkbcommon.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.libxkbcommon}/lib/libxkbcommon-x11.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libX11}/lib/libX11.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libX11}/lib/libX11-xcb.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.libGL}/lib/libGL.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.libGL}/lib/libGLX.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.vulkan-loader}/lib/libvulkan.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libX11}/lib/libX11.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libXext}/lib/libXext.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libXrender}/lib/libXrender.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libXfixes}/lib/libXfixes.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libXau}/lib/libXau.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libXdmcp}/lib/libXdmcp.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libxcb}/lib/libxcb.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libXcursor}/lib/libXcursor.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libXrandr}/lib/libXrandr.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libXi}/lib/libXi.so
              patchelf $out/bin/eyeqwst --add-needed ${pkgs.xorg.libX11}/lib/libX11.so
            '';

            src = ./.;
          };
          eyeqwst-wrapped = pkgs.writeShellScriptBin "eyeqwst" ''
            exec ${builtins.trace pkgs.nixgl pkgs.nixgl.nixGLMesa}/bin/nixGLMesa ${default}/bin/eyeqwst "$@"
          '';
        }
      );
    };
}
