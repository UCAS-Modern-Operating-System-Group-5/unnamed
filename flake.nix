{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flakelight.url = "github:nix-community/flakelight";
  };
  
  outputs = { flakelight, ... }@inputs:
    flakelight ./. {
      devShell = pkgs: let
        runtimeLibs = with pkgs; [
          # apps/gui ==============
          libxkbcommon
          wayland
          
          # vulkan-loader
          libGL
        ];
      in {
        env.LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath runtimeLibs}";

        packages = with pkgs; [
          pkg-config

          # apps/gui ==============
          fontconfig
          
          # puffin_viewer ---------
          # (Though they are compile time needed)
          gtk3
          glib
          atk
        ];
      };
    };
}
