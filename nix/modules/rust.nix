{ inputs, ... }:
{
  imports = [
    inputs.rust-flake.flakeModules.default
    inputs.rust-flake.flakeModules.nixpkgs
    inputs.process-compose-flake.flakeModule
    inputs.cargo-doc-live.flakeModule
  ];
  perSystem = { config, self', pkgs, lib, ... }: {
    rust-project.crates."keyboard-research-kit".crane.args = {
      buildInputs = (lib.optionals pkgs.stdenv.isLinux (
        with pkgs; [
          autoconf
          automake
          libevdev
        ]
      )) ++ (lib.optionals pkgs.stdenv.isDarwin (
        with pkgs.darwin.apple_sdk.frameworks; [
          IOKit
        ]
      ));
    };
    packages.default = self'.packages.keyboard-research-kit;
  };
}
