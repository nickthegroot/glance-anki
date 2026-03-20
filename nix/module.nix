{ self }:

{ config, lib, pkgs, ... }:

let
  cfg = config.services.glance-anki;
  pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
in
{
  options.services.glance-anki = {
    enable = lib.mkEnableOption "glance-anki Anki review heatmap widget";

    collectionPath = lib.mkOption {
      type = lib.types.str;
      description = "Absolute path to the Anki collection.anki2 SQLite file.";
      example = "/home/alice/.local/share/Anki2/User 1/collection.anki2";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = "TCP port the server listens on.";
    };

    defaultDays = lib.mkOption {
      type = lib.types.ints.positive;
      default = 30;
      description = "Default number of past days shown when no ?days= query param is given.";
    };

    user = lib.mkOption {
      type = lib.types.str;
      default = "glance-anki";
      description = "User account under which the service runs.";
    };

    group = lib.mkOption {
      type = lib.types.str;
      default = "glance-anki";
      description = "Group under which the service runs.";
    };
  };

  config = lib.mkIf cfg.enable {
    users.users.${cfg.user} = lib.mkIf (cfg.user == "glance-anki") {
      isSystemUser = true;
      group = cfg.group;
      description = "glance-anki service user";
    };

    users.groups.${cfg.group} = lib.mkIf (cfg.group == "glance-anki") { };

    systemd.services.glance-anki = {
      description = "glance-anki Anki review heatmap widget";
      wantedBy = [ "multi-user.target" ];
      after = [ "network.target" ];

      environment = {
        ANKI_COLLECTION_PATH = cfg.collectionPath;
        PORT = toString cfg.port;
        DEFAULT_DAYS = toString cfg.defaultDays;
        RUST_LOG = "info";
      };

      serviceConfig = {
        ExecStart = "${pkg}/bin/glance-anki";
        User = cfg.user;
        Group = cfg.group;
        Restart = "on-failure";
        RestartSec = "5s";

        # Hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = "read-only";
        PrivateTmp = true;
        PrivateDevices = true;
        ReadOnlyPaths = [ cfg.collectionPath ];
      };
    };

    networking.firewall.allowedTCPPorts = [ cfg.port ];
  };
}
