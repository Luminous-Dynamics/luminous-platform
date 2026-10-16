{
  description = "Pinned NixOS/QEMU test harness for Luminous Hearth Node pre-hardware contracts";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
      hearthFoundation = pkgs.testers.runNixOSTest (import ./hearth-foundation.nix);
    in
    {
      checks.${system}.hearth-foundation = hearthFoundation;
      packages.${system}.hearth-foundation = hearthFoundation;
      packages.${system}.default = hearthFoundation;
    };
}
