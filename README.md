# missiond

missiond is the daemon that owns an information display. It holds the content, the screen power,
and one control surface that a browser, Home Assistant, or anything that can make an HTTP request
all drive.

It runs on a Wayland session, starts Chromium, and rotates a playlist of tabs. What is on screen
is decided here rather than by the pages themselves.

Features:

- Playlists of tabs, rotated on a wall-clock aligned interval
- Screen power and DDC brightness, on a weekly schedule
- Poweroff, reboot and suspend over logind
- Home Assistant MQTT discovery for the screen, the playlist and the tab
- iCalendar feeds on a rail beside the content, with a toast before each meeting
- Webhooks that raise a preset alert, such as a doorbell press that puts the camera on screen
- A web UI with live previews, driven by a server sent event stream
- An OpenAPI-documented REST API
- A NixOS module that declares the whole configuration

## Configuration

missiond reads a directory of TOML documents. See [docs/configuration.md](docs/configuration.md)
for the schema.

| Directory | Resolution order | Holds |
| --- | --- | --- |
| config | `MISSIOND_CONFIG_DIR`, `$XDG_CONFIG_HOME/missiond`, `~/.config/missiond` | The TOML documents. The only directory worth backing up. |
| state | `MISSIOND_STATE_DIR`, `$XDG_STATE_HOME/missiond`, `~/.local/state/missiond` | `runtime.sqlite3`. Deleting it costs one restart. |
| cache | `MISSIOND_CACHE_DIR`, `$XDG_CACHE_HOME/missiond`, `~/.cache/missiond` | The Chromium profile. |

## NixOS

```nix
{
  inputs.missiond.url = "github:v3xlabs/missiond";

  # in your configuration
  imports = [inputs.missiond.nixosModules.default];

  services.missiond = {
    enable = true;
    user = "display";
    host = "0.0.0.0";
    port = 3000;
    openFirewall = true;
    adminKeyFile = config.sops.secrets.missiond_admin_key.path;

    # The daemon runs commands and players by name, and a systemd unit inherits no PATH.
    extraPackages = [config.programs.niri.package pkgs.ddcutil pkgs.grim pkgs.mpv];

    settings = {
      name = "Lobby Display";
      device_id = "lobby-display";

      display.output = "DP-1";
      display.schedule = [
        {
          days = ["mon" "tue" "wed" "thu" "fri"];
          from = "07:30";
          to = "23:00";
        }
      ];

      tabs.overview.url = "https://grafana.example.com/d/overview?kiosk";

      # Any RTSP stream can be a tab. A browser has no rtsp:// handler, so mpv plays it outside
      # the browser, which is why mpv is in extraPackages.
      tabs.entrance-camera = {
        rtsp.file = config.sops.secrets.entrance_camera_url.path;
        stinger = "doorbell";
      };

      # A stinger names a file, and `media` is what puts that file in the config directory.
      notifications.stingers.doorbell = {
        file = "doorbell.webm";
        max_duration = "1500ms";
      };

      media."doorbell.webm" = ./media/doorbell.webm;

      # Any iCalendar feed. Its link is a bearer credential, so it takes a reference like the
      # camera above. Entries appear on a rail beside the content, and a toast goes up five
      # minutes before each one and again as it starts.
      calendars.work = {
        name = "Work";
        url.file = config.sops.secrets.work_ics.path;
        window = "12h";
        leads = ["5m" "0s"];
      };

      playlists.lobby = {
        interval = "1m";
        is_default = true;
        tabs = ["overview"];
      };
    };
  };
}
```

`settings` generates the config directory into the Nix store, which is read-only. The web UI can
still change anything, and says so: a change applies to the running display immediately and does
not survive a restart. `GET /api/config/export` returns the effective configuration as TOML, so a
change worth keeping can be moved back into your Nix.

Leave `settings` unset to let missiond own its config directory and write to it.

## Development

Everything is in the flake devshell.

```bash
nix develop
just          # list the recipes
just dev      # run the daemon
just web      # run the web UI against it on :5173
just kiosk    # run the daemon inside a nested cage session
just lab      # run it inside a nested niri, in one window on your desktop
just check    # clippy, tests, typecheck, lint
just build    # build the web UI, embed it, build the release binary
```

`just lab` is the one to reach for when a change touches a window: the rail, the toast overlay and
the camera all need a compositor the daemon can drive, and a nested niri gives you one without
touching the session you are working in. It keeps its state under `.tmp/lab`, and `just lab-ics`
writes a calendar feed with events a few minutes out so the agenda can be watched end to end.

`just schema` regenerates `web/src/api/schema.gen.ts` from a running daemon.
