{self}: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.missiond;
  inherit (lib) types;

  toml = pkgs.formats.toml {};

  # Only the options the module has to reason about are typed. Tabs, playlists and overlay pass
  # through to the TOML generator, so a schema change does not have to be mirrored in Nix twice.
  passthrough = types.attrsOf types.anything;

  scheduleWindow = types.submodule {
    options = {
      days = lib.mkOption {
        type = types.listOf (types.enum ["mon" "tue" "wed" "thu" "fri" "sat" "sun"]);
        description = "Days this window applies to.";
      };
      from = lib.mkOption {
        type = types.strMatching "[0-9]{2}:[0-9]{2}";
        description = "Local time the screen turns on, as HH:MM.";
      };
      to = lib.mkOption {
        type = types.strMatching "[0-9]{2}:[0-9]{2}";
        description = "Local time the screen turns off, as HH:MM.";
      };
    };
  };

  displayOptions = types.submodule {
    options = {
      output = lib.mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Output name substituted into the power and brightness commands.";
      };
      power_on = lib.mkOption {
        type = types.listOf types.str;
        default = ["niri" "msg" "action" "power-on-monitors"];
        description = "Command that turns the screen on.";
      };
      power_off = lib.mkOption {
        type = types.listOf types.str;
        default = ["niri" "msg" "action" "power-off-monitors"];
        description = "Command that turns the screen off.";
      };
      brightness = lib.mkOption {
        type = types.listOf types.str;
        default = ["ddcutil" "setvcp" "10" "{percent}"];
        description = "Command that sets panel brightness. `{percent}` is substituted.";
      };
      schedule = lib.mkOption {
        type = types.listOf scheduleWindow;
        default = [];
        description = "Weekly windows during which the screen is on.";
      };
    };
  };

  # The daemon reads arrays; the module takes attribute sets so an entry can be named and
  # overridden the way the rest of a NixOS config is.
  namedList = key: attrs:
    lib.mapAttrsToList (name: value: {${key} = name;} // value) attrs;

  dropNulls = value:
    if builtins.isAttrs value
    then lib.filterAttrs (_: item: item != null) (lib.mapAttrs (_: dropNulls) value)
    else if builtins.isList value
    then map dropNulls value
    else value;

  settingsDir = pkgs.runCommand "missiond-config" {} ''
    mkdir -p "$out"
    ln -s ${toml.generate "device.toml" (dropNulls ({
        version = 1;
        name = cfg.settings.name;
        device_id = cfg.settings.device_id;
        http = {
          inherit (cfg) host port;
        };
        chromium = cfg.settings.chromium;
        mdns = cfg.settings.mdns;
        mpv = cfg.settings.mpv;
      }
      // lib.optionalAttrs (cfg.adminKeyFile != null) {
        admin_key.file = cfg.adminKeyFile;
      }
      // lib.optionalAttrs (cfg.controlKeyFile != null) {
        control_key.file = cfg.controlKeyFile;
      }
      // lib.optionalAttrs (cfg.settings.homeassistant != null) {
        homeassistant = cfg.settings.homeassistant;
      }))} "$out/device.toml"
    ln -s ${toml.generate "display.toml" (dropNulls ({version = 1;} // cfg.settings.display))} "$out/display.toml"
    ln -s ${toml.generate "tabs.toml" {
      version = 1;
      tabs = dropNulls (namedList "tab_id" cfg.settings.tabs);
    }} "$out/tabs.toml"
    ln -s ${toml.generate "playlists.toml" {
      version = 1;
      playlists = dropNulls (namedList "playlist_id" cfg.settings.playlists);
    }} "$out/playlists.toml"
    ln -s ${toml.generate "notifications.toml" (dropNulls ({version = 1;} // cfg.settings.notifications))} "$out/notifications.toml"
    ln -s ${toml.generate "calendars.toml" (dropNulls ({
        version = 1;
        calendars = dropNulls (namedList "calendar_id" cfg.settings.calendars);
      }
      // lib.optionalAttrs (cfg.settings.meetings != {}) {
        meetings = cfg.settings.meetings;
      }
      // cfg.settings.calendarDefaults))} "$out/calendars.toml"

    # A stinger clip is a build input like anything else, so the media directory is assembled
    # here rather than left for someone to copy onto the machine by hand.
    mkdir -p "$out/media"
    ${lib.concatStringsSep "\n" (lib.mapAttrsToList (name: file: ''
        ln -s ${file} "$out/media/${name}"
      '')
      cfg.settings.media)}
  '';

  configDir =
    if cfg.settings == null
    then cfg.configDir
    else settingsDir;
in {
  options.services.missiond = {
    enable = lib.mkEnableOption "missiond, the Mission Control display daemon";

    package = lib.mkPackageOption self.packages.${pkgs.stdenv.hostPlatform.system} "missiond" {};

    user = lib.mkOption {
      type = types.str;
      description = ''
        The user whose graphical session missiond joins. Unlike a network daemon, missiond has to
        start a browser on a compositor, so it runs as a session service rather than a system one.
      '';
      example = "display";
    };

    host = lib.mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address the web interface and API listen on.";
    };

    port = lib.mkOption {
      type = types.port;
      default = 3000;
      description = "Port the web interface and API listen on.";
    };

    openFirewall = lib.mkEnableOption "opening the missiond port in the firewall";

    adminKeyFile = lib.mkOption {
      type = types.nullOr types.path;
      default = null;
      example = "/run/secrets/missiond_admin_key";
      description = ''
        File holding the key every mutating request must present as a bearer token. Without one,
        anything that can reach the port can change what is on screen.
      '';
    };

    controlKeyFile = lib.mkOption {
      type = types.nullOr types.path;
      default = null;
      example = "/run/secrets/missiond_control_key";
      description = ''
        File holding a second bearer key that changes what is on screen and nothing else: playback,
        screen power and brightness, alerts, the rail and the agenda. It cannot edit configuration
        or power the machine off. Meant for a button panel. Only checked when adminKeyFile is set.
      '';
    };

    extraPackages = lib.mkOption {
      type = types.listOf types.package;
      default = [];
      example = lib.literalExpression "[config.programs.niri.package pkgs.ddcutil pkgs.grim pkgs.mpv]";
      description = ''
        Extra packages on the daemon's PATH. The display commands are run by name, and a systemd
        unit inherits no PATH of its own, so niri, ddcutil and grim have to be listed here for
        screen power, brightness and the screen capture to work, and mpv for a camera tab.
      '';
    };

    configDir = lib.mkOption {
      type = types.path;
      default = "/var/lib/missiond/config";
      description = "Directory holding the TOML documents, when `settings` is not used.";
    };

    settings = lib.mkOption {
      default = null;
      description = ''
        Configuration generated as TOML into the Nix store. The store path is read-only, so the
        web UI applies a change immediately and reports that it will not survive a restart.
      '';
      type = types.nullOr (types.submodule {
        options = {
          name = lib.mkOption {
            type = types.str;
            default = "Mission Control";
            description = "Display name, shown in the web UI and to Home Assistant.";
          };

          device_id = lib.mkOption {
            type = types.str;
            default = "missiond";
            description = "Stable identity, used for MQTT discovery topics and the mDNS host name.";
          };

          mdns = lib.mkOption {
            type = types.bool;
            default = true;
            description = ''
              Advertise the API as `_missiond._tcp` on the local network. Nothing is advertised
              while `host` is a loopback address.
            '';
          };

          chromium = lib.mkOption {
            type = passthrough;
            default = {};
            description = ''
              Browser settings. `fullscreen` defaults to true, which covers the output and hides
              the browser's own interface. Turning it off leaves a window the compositor tiles.
            '';
          };

          mpv = lib.mkOption {
            type = passthrough;
            default = {};
            example = lib.literalExpression ''
              {
                extra_args = ["--hwdec=auto-safe"];
              }
            '';
            description = ''
              The player that draws camera tabs. mpv has to be in `extraPackages` for a camera to
              work at all.
            '';
          };

          homeassistant = lib.mkOption {
            type = types.nullOr passthrough;
            default = null;
            description = "MQTT connection for Home Assistant discovery.";
          };

          display = lib.mkOption {
            type = displayOptions;
            default = {};
            description = "Screen power, brightness and schedule.";
          };

          tabs = lib.mkOption {
            type = passthrough;
            default = {};
            example = lib.literalExpression ''
              {
                overview.url = "https://grafana.example.com/d/overview?kiosk";
              }
            '';
            description = "Tabs, keyed by tab id.";
          };

          notifications = lib.mkOption {
            type = passthrough;
            default = {};
            example = lib.literalExpression ''
              {
                mode = "takeover";
                default_duration = "20s";
                stingers.doorbell = {
                  file = "doorbell.webm";
                  max_duration = "2s";
                };
              }
            '';
            description = "Alert behaviour and the named stinger clips.";
          };

          calendars = lib.mkOption {
            type = passthrough;
            default = {};
            example = lib.literalExpression ''
              {
                work = {
                  name = "Work";
                  url.file = config.sops.secrets.work_ics.path;
                  refresh = "15m";
                  window = "12h";
                  leads = ["5m" "0s"];
                };
              }
            '';
            description = ''
              iCalendar feeds, keyed by calendar id. Each one appears on the rail as it comes
              into `window`, and raises a toast at each of its `leads`.

              A feed's address is the credential for it, so `url` takes the same reference an
              RTSP camera does: `url.file` for a path, `url.env` for an environment variable, or
              a plain string when the link is not a secret.
            '';
          };

          meetings = lib.mkOption {
            type = passthrough;
            default = {};
            example = lib.literalExpression ''
              {
                providers.jitsi = ["meet.jit.si" "*.jitsi.example.org"];
              }
            '';
            description = ''
              Host patterns keyed by meeting provider, used to put an icon beside a calendar
              entry. `*.example.com` matches any subdomain and not `example.com` itself.

              Merged over the built-in providers by name, so naming one does not mean restating
              the rest. Zoom, Google Meet, Jitsi, Webex and Teams are recognised out of the box.
            '';
          };

          calendarDefaults = lib.mkOption {
            type = passthrough;
            default = {};
            example = lib.literalExpression ''
              {
                poll = "1m";
              }
            '';
            description = ''
              Values at the top of `calendars.toml`, which is `poll`: how often the rail is
              reconciled against what has already been fetched. Separate from a feed's `refresh`,
              which is how often the daemon goes back to the network.
            '';
          };

          media = lib.mkOption {
            type = types.attrsOf types.path;
            default = {};
            example = lib.literalExpression ''
              {
                "doorbell.webm" = ./media/doorbell.webm;
              }
            '';
            description = ''
              Files placed in the config directory's media, keyed by the name a stinger refers
              to. A path here can be a file in your configuration, or any derivation that
              produces one.
            '';
          };

          playlists = lib.mkOption {
            type = passthrough;
            default = {};
            example = lib.literalExpression ''
              {
                lobby = {
                  interval = "1m";
                  is_default = true;
                  tabs = ["overview"];
                };
              }
            '';
            description = "Playlists, keyed by playlist id. The tab list order is the play order.";
          };
        };
      });
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.settings == null || cfg.configDir == "/var/lib/missiond/config";
        message = "services.missiond.settings cannot be used with services.missiond.configDir.";
      }
      {
        assertion = cfg.adminKeyFile != null || cfg.host == "127.0.0.1";
        message = ''
          services.missiond listens on ${cfg.host} without an adminKeyFile, so anything that can
          reach port ${toString cfg.port} can change the display. Set adminKeyFile, or bind to
          127.0.0.1.
        '';
      }
    ];

    networking.firewall.allowedTCPPorts = lib.mkIf cfg.openFirewall [cfg.port];
    networking.firewall.allowedUDPPorts = lib.mkIf (cfg.openFirewall && (cfg.settings == null || cfg.settings.mdns)) [5353];

    systemd.user.services.missiond = {
      description = "Mission Control display daemon";
      wantedBy = ["graphical-session.target"];
      partOf = ["graphical-session.target"];
      after = ["graphical-session.target"];

      # The browser and the display commands are found here rather than assumed to be on the
      # user's PATH, which a systemd unit does not inherit.
      path = cfg.extraPackages;

      environment = {
        MISSIOND_CONFIG_DIR = toString configDir;
        MISSIOND_CONFIG_READ_ONLY = lib.boolToString (cfg.settings != null);
        MISSIOND_STATE_DIR = "%S/missiond";
        MISSIOND_CACHE_DIR = "%C/missiond";
      };

      serviceConfig = {
        ExecStart = lib.getExe cfg.package;
        Restart = "on-failure";
        RestartSec = 5;
        StateDirectory = "missiond";
        CacheDirectory = "missiond";
      };
    };

    systemd.user.services.missiond.unitConfig.ConditionUser = cfg.user;
  };
}
