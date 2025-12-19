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
        env = {
          LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath runtimeLibs}";
          
          # Needed for cross compilation by `cross`
          CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = "gcc";
          CROSS_CONTAINER_UID = "0";
          CROSS_CONTAINER_GID = "0";
        };

        packages = with pkgs; [
          pkg-config

          # apps/gui -----------
          fontconfig
        ];
      };
    };
}
