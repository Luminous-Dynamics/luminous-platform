{ pkgs ? import <nixpkgs> { } }:

pkgs.testers.runNixOSTest (import ./hearth-foundation.nix)
