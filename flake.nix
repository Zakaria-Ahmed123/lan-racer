{
  description = "Lan Racer";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      dlopenLibraries = with pkgs; [
        lldb
        gnumake
        libxkbcommon

        # GPU backend
        # vulkan-loader
        libGL

        # Window system
        wayland
        xorg.libX11
        xorg.libXcursor
        xorg.libXi
      ];
      buildInputs = with pkgs; [
        lldb
        gnumake
      ];
    in {
      devShells.${system}.default = pkgs.mkShell {
        env.RUSTFLAGS = "-C link-arg=-Wl,-rpath,${nixpkgs.lib.makeLibraryPath dlopenLibraries}";
      };
    };
}
