{
  description = "Dactyl Remote Control";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs {
          inherit system;
          overlays = overlays;
        };
        rustToolchain = pkgs.rust-bin.stable.latest.default;
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in {
        packages.default = rustPlatform.buildRustPackage {
          pname = "dactyl-remote-control";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.systemd ]; # for libudev
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [ rustToolchain pkgs.pkg-config pkgs.systemd ];
        };

        nixosModules.dactyl-remote-control = { config, lib, pkgs, ... }:
          with lib;
          let cfg = config.services.dactyl-remote-control;
          in {
            options.services.dactyl-remote-control = {
              enable = mkEnableOption "Dactyl Remote Control";

              vid = mkOption {
                type = types.str;
                default = "0x4B41";
                description = "USB vendor ID";
              };

              pid = mkOption {
                type = types.str;
                default = "0x636D";
                description = "USB product ID";
              };

              configPath = mkOption {
                type = types.path;
                default = "/etc/dactyl-remote-control/config.yaml";
                description = "Path to the config.yaml file.";
              };

              package = mkOption {
                type = types.package;
                default = self.packages.${system}.default;
                description = "The dactyl-remote-control package to use.";
              };

              user = mkOption {
                type = types.str;
                default = "your-user"; # you may override this per host
                description = "User account to run the service under.";
              };
            };

            config = mkIf cfg.enable {
              # Ensure package is available
              environment.systemPackages = [ cfg.package ];

              systemd.user.services.dactyl-remote-control = {
                description = "Dactyl remote control";
                wantedBy = [ "graphical-session.target" ];
                partOf = [ "graphical-session.target" ];
                after = [ "default.target" ];
                wants = [ "default.target" ];
                serviceConfig = {
                  Type = "simple";
                  ExecStart = ''
                    ${cfg.package}/bin/dactyl-remote-control \
                      -vvv \
                      --vid ${cfg.vid} \
                      --pid ${cfg.pid} \
                      watch-window-focus \
                      --config ${cfg.configPath}
                  '';
                  Restart = "always";
                };
              };

              # Ensure user service is enabled
              systemd.user.services.dactyl-remote-control.Install.WantedBy =
                [ "graphical-session.target" ];
            };
          };
      });
}
