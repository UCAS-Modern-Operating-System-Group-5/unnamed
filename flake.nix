{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flakelight.url = "github:nix-community/flakelight";
  };
  
  outputs = { flakelight, ... }@inputs:
    flakelight ./. {
      devShell = pkgs: let
        runtimeLibs = with pkgs; [
          # apps/gui -----------
          libxkbcommon
          wayland
          
          # vulkan-loader
          libGL 
        ];
      in {
        env.LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath runtimeLibs}";
        # env.WGPU_BACKEND = "vulkan";

        packages = with pkgs; [
          pkg-config

          # apps/gui -----------
          fontconfig
        ];
      };
    };
}
