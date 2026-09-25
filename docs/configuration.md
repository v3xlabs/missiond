# Configuration

This is the schema reference for every file missiond reads. What the daemon writes back, what the
web UI exports, and what the NixOS module generates are the same documents.

## Locations

| Path | Contents |
| --- | --- |
| `$MISSIOND_CONFIG_DIR`, else `$XDG_CONFIG_HOME/missiond`, else `~/.config/missiond` | configuration |
| `$MISSIOND_STATE_DIR`, else `$XDG_STATE_HOME/missiond`, else `~/.local/state/missiond` | volatile runtime state |
| `$MISSIOND_CACHE_DIR`, else `$XDG_CACHE_HOME/missiond`, else `~/.cache/missiond` | the Chromium profile |

```text
~/.config/missiond/
  device.toml
  display.toml
  tabs.toml
  playlists.toml
```

A document that does not exist loads its defaults, so a fresh install starts with no tabs rather
than with an error. Every document carries `version`, and the daemon writes the current version
back.

State and cache are both reconstructible. `runtime.sqlite3` holds what was on screen and whether
the display was on; deleting it costs one restart. Configuration is the only directory worth
backing up.

Writes are atomic: the daemon writes `<name>.toml.tmp` and renames.

## Read-only mode

The config directory is the source of truth. When the NixOS module generates it, the directory is
a Nix store path and every write fails, so missiond treats it as read-only. It detects this two
ways: `MISSIOND_CONFIG_READ_ONLY=1`, which the module sets, or a directory with no write bits.

In read-only mode a change from the web UI still applies. The display reacts at once, and the API
answers `{"persisted": false}` so the UI can say the change lasts until the next restart. Use
`GET /api/config/export` to get the effective configuration back as TOML.

This is the deliberate trade for a declarative setup, and it is why the UI shows a badge rather
than failing a save.

## device.toml

```toml
version = 1
name = "Lobby Display"
device_id = "lobby-display"

[http]
host = "0.0.0.0"
port = 3000

[admin_key]
file = "/run/secrets/missiond_admin_key"

[control_key]
file = "/run/secrets/missiond_control_key"

[chromium]
enabled = true
fullscreen = true
binary_path = "/run/current-system/sw/bin/chromium"
extra_args = ["--force-dark-mode"]

[mpv]
binary_path = "/run/current-system/sw/bin/mpv"
extra_args = ["--hwdec=auto-safe"]

[homeassistant]
mqtt_url = "mqtt://broker.example:1883"
username = "missiond"
password = { env = "MISSIOND_MQTT_PASSWORD" }
```

| Field | Type | Notes |
| --- | --- | --- |
| `name` | string | Display name, shown in the web UI and to Home Assistant. |
| `device_id` | string | Stable identity. MQTT discovery topics are built from it. |
| `http.host`, `http.port` | string, integer | Where the API and web UI listen. Defaults are `0.0.0.0` and `3000`. |
| `admin_key` | secret, optional | Required on every mutating request as `authorization: Bearer <key>`. Without one, anything that can reach the port can change the display. |
| `control_key` | secret, optional | A second bearer key for a button panel. It drives what is on screen and cannot edit configuration. See [The API](#the-api). Only checked when `admin_key` is set. |
| `mdns` | boolean | Defaults to `true`. Advertises the API as `_missiond._tcp` with `device_id`, `name` and `version` in the TXT record. Nothing is advertised while `http.host` is a loopback address. |
| `chromium.enabled` | boolean | A disabled browser leaves the API and web UI running, which is how the daemon is tested without a compositor. |
| `chromium.fullscreen` | boolean | Defaults to `true`. See below. |
| `chromium.binary_path` | string, optional | Falls back to `$CHROMIUM_BINARY`, then to `chromium` on `PATH`. |
| `chromium.extra_args` | array | Appended to the argument list. |
| `mpv.binary_path` | string, optional | Falls back to `$MPV_BINARY`, then to `mpv` on `PATH`. Only camera tabs use it. |
| `mpv.extra_args` | array | Appended to the argument list. |
| `homeassistant` | table, optional | Omit it and MQTT stays off. |

### fullscreen

Defaults to `true`. The browser covers the output and draws none of its own interface: no tab
strip, no omnibox, no profile button.

This takes two things, not one. `--kiosk` covers the output, but a tab created over CDP needs a
tab strip to live in and drags the window back to its decorated form, so the daemon also puts the
window itself into fullscreen through `Browser.setWindowBounds`. That is the presentation change
`F11` makes, and it is what actually hides the browser interface.

Leave it on. The sidebar does not need it off: while the rail is open the daemon puts the browser
into the compositor's windowed fullscreen, so Chromium keeps believing it is fullscreen and keeps
drawing no interface, while the compositor tiles it beside the rail. See
[How the rail gets its column](#how-the-rail-gets-its-column).

Setting it to `false` starts a maximised window the compositor tiles instead, and the browser draws
its tab strip and its omnibox on the display for as long as it runs. That is only worth doing to
see what the page looks like at a size the display never uses. If you do turn it off, a tiled
window on niri opens at its column width, which is half the screen, so add a window rule:

```kdl
window-rule {
    match app-id="chromium-browser"
    open-maximized true
    draw-border-with-background false
}
```

## display.toml

```toml
version = 1
output = "DP-1"

power_on  = ["niri", "msg", "action", "power-on-monitors"]
power_off = ["niri", "msg", "action", "power-off-monitors"]
brightness = ["ddcutil", "setvcp", "10", "{percent}", "--display", "1"]
screenshot = ["grim", "-t", "jpeg", "-q", "80", "-"]

[[schedule]]
days = ["mon", "tue", "wed", "thu", "fri"]
from = "07:30"
to   = "23:00"
```

The commands live here rather than in a match arm inside the daemon, so a compositor missiond has
never heard of needs a config line rather than a new code path. `{percent}` and `{output}` are
substituted. The defaults are the niri commands above.

| Field | Type | Notes |
| --- | --- | --- |
| `output` | string, optional | Substituted for `{output}`. |
| `power_on`, `power_off` | array | Argument vectors, not shell strings. |
| `brightness` | array | Receives `{percent}` from 0 through 100. |
| `screenshot` | array | Writes the output's current contents to stdout. The default uses `grim`, which speaks wlr-screencopy. |
| `schedule` | array of tables | Weekly windows during which the screen is on. |

`days` names the days a window *starts* on. A window whose `to` is earlier than its `from` runs past
midnight and ends on the next morning, which belongs to the evening that opened it:

```toml
[[schedule]]
days = ["mon", "tue", "wed", "thu", "fri"]
from = "09:00"
to   = "04:00"
```

is on from nine each weekday morning until four the next morning, Saturday morning included, and
does not come on at midnight on Sunday because Sunday is not a day it starts on.

A window whose `from` and `to` are equal is the whole day, which `HH:MM` cannot otherwise say. A day
with no window is off. With no windows at all the schedule has no opinion and never touches the
screen. A time that is not `HH:MM` stops the daemon at load rather than leaving the screen dark all
day for a reason nothing reports.

The schedule sets the baseline. A command from the API or from MQTT overrides it, and the override
holds until the next schedule boundary rather than forever, so turning the screen on at midnight
does not keep it on until Monday.

## tabs.toml

```toml
version = 1

[[tabs]]
tab_id = "overview"
name = "Overview"
url = "https://grafana.example.com/d/overview?kiosk"
persist = true
scale = 1.25
```

| Field | Type | Notes |
| --- | --- | --- |
| `tab_id` | string | The identity. A playlist references it, and changing it orphans those references. |
| `name` | string, optional | Web UI label. Defaults to `tab_id`. |
| `url` | string | What the page loads. A tab has this or `rtsp`, never both. |
| `rtsp` | secret, optional | A camera stream. The whole url is treated as a secret, because the credential is part of it. |
| `persist` | boolean | Whether the page stays loaded when the playlist moves on. Defaults to `true`. |
| `scale` | float, optional | Device scale factor for this page only, so a dashboard and a departure board can differ on the same panel. |
| `stinger` | string, optional | A clip played while this tab loads, by name from `notifications.toml`. |

### Cameras

A browser has no `rtsp://` handler, so a camera is not a page. missiond plays it with mpv in its
own window, and the compositor puts that window over the browser. Everything else about a camera
is an ordinary tab: it sits in a playlist, it rotates, and an alert can take over with it.

```toml
[[tabs]]
tab_id = "entrance-camera"
name = "Entrance"
stinger = "doorbell"

[tabs.rtsp]
file = "/run/secrets/entrance-camera-url"
```

The file holds the whole url on one line, credential included:

```
rtsp://admin:hunter2@10.0.0.40:554/stream1
```

`env = "NAME"` reads it from the environment instead. An inline string works and is what a test
uses, but it puts a credential in the config file.

missiond hands the url to mpv over a control socket rather than as an argument, so it does not
appear in the process list. It never reaches the web UI, Home Assistant or a log line either: the
API reports a camera by leaving `url` out.

Two consequences follow from a camera being outside the browser. `GET /api/preview/:tab_id` has
no frame for one, because previews come from the browser's own protocol, while `GET /api/screen`
still shows it, because that reads the compositor. And `POST /api/tabs` cannot create one: a
stream url is a credential, and the API writes what it is given back to disk.

mpv has to be on the daemon's PATH. Under the NixOS module that means `extraPackages`. The window
carries the app id `missiond-camera`, which is how the daemon finds it again and what a compositor
rule matches on.

A camera is a second window, and which window reaches the panel is the compositor's decision. niri
holds every window in one scrolling layout, and it hands the focus to a new window only when the
client can show that a person opened it. A player started by a daemon cannot, so the camera window
arrives in a column beside the browser and is never seen. missiond focuses it itself, over the
socket niri names in `NIRI_SOCKET`. `niri --session` puts that variable in the user manager's
environment, so a user service inherits it and nothing has to be configured.

The way back needs nothing. Stopping the camera destroys its window, and the browser is the only
window the compositor has left to focus.

On a compositor that is not niri the daemon logs that it could not bring the window forward, and
where that window lands is up to that compositor.

A camera that drops leaves its last frame on the wall rather than closing its window, which would
put whatever page is behind it on screen instead.

## playlists.toml

```toml
version = 1

[[playlists]]
playlist_id = "lobby"
name = "Lobby"
interval = "1m"
hold = "5m"
is_default = true
tabs = ["overview", "uptime", "departures"]
disabled_tabs = ["departures"]
```

| Field | Type | Notes |
| --- | --- | --- |
| `playlist_id` | string | The identity. |
| `name` | string, optional | Defaults to `playlist_id`. |
| `interval` | duration | How long each tab holds the screen. |
| `hold` | duration, optional | How long a tab chosen by hand suspends rotation. Without it, pressing a Stream Deck key can be undone by the rotation timer a second later. |
| `is_default` | boolean | Which playlist starts at boot. With none set, the first one starts. |
| `tabs` | array of `tab_id` | **The list order is the play order.** Reordering in the web UI reorders this array. |
| `disabled_tabs` | array of `tab_id` | Skipped without being removed, so re-enabling keeps the position. |

A `tab_id` with no tab behind it is skipped rather than treated as an error, so deleting a tab
cannot break a playlist.

Rotation aligns to the wall clock: a one minute interval changes on the minute.

## Durations

`750ms`, `30s`, `5m`, `1h`. A bare number is seconds. These are what the daemon writes back, so a
round trip through the web UI does not turn `1m` into `60`.

## Secrets

Any secret field accepts three forms:

```toml
admin_key = { env = "MISSIOND_ADMIN_KEY" }    # read from the environment at start
admin_key = { file = "/run/secrets/key" }     # read from disk at start, trimmed
admin_key = "inline-is-permitted"             # discouraged
```

Resolution happens once, at start. A missing variable or an unreadable file stops the daemon with
a readable message rather than starting it with an empty credential, because a display daemon that
silently accepts every request is worse than one that does not start. A webhook token is the
exception: it is read on each call, and a call whose token cannot be read is refused.

**An export never contains an inline secret.** `GET /api/config/export` replaces an inline value
with a reference placeholder derived from the field, so the exported document is directly usable
given the environment variable rather than merely redacted.

## The API

`GET` is open. Every other method requires a key when an admin key is configured. The admin key
opens every route. The control key opens the routes marked *control*: they change what is on
screen, and none of them edits configuration or powers the machine off.

| Method | Path | Key | Does |
| --- | --- | --- | --- |
| GET | `/api/status` | | What is on screen, plus `config_read_only` and the `access` the request's key has. |
| GET | `/api/events` | | Server sent events. See [The event stream](#the-event-stream). |
| GET | `/api/playlists` | | Every playlist. |
| GET | `/api/playlists/:playlist_id/tabs` | | A playlist's tabs, in play order. |
| GET | `/api/tabs` | | Every configured tab. |
| GET | `/api/preview/:tab_id` | | One JPEG frame. |
| GET | `/api/preview_live/:tab_id` | | An MJPEG stream. |
| GET | `/api/config/export` | | Every document as TOML. |
| POST | `/api/playlists/:playlist_id/activate` | control | Put a playlist on screen. |
| POST | `/api/playlists/:playlist_id/tabs/:tab_id/activate` | control | Put a tab on screen, and hold it. |
| POST | `/api/tabs/:tab_id/activate` | control | Put a tab on screen, and hold it, within the current playlist when that has the tab and otherwise within the first playlist that does. |
| POST | `/api/playback/{next,previous,pause,resume}` | control | Drive the rotation. |
| POST | `/api/playback/toggle` | control | Pause or resume. Answers `{"auto_rotate": ...}`. |
| POST | `/api/tabs/:tab_id/{refresh,recreate}` | control | Reload a tab's page, or close and reopen it. |
| PUT | `/api/playlists/:playlist_id/reorder` | admin | Reorder, with the full tab list. |
| PUT | `/api/playlists/:playlist_id/tabs/:tab_id/enabled` | admin | Include or exclude a tab. |
| PUT | `/api/tabs/:tab_id` | admin | Create a tab, or replace one with that id. |
| DELETE | `/api/tabs/:tab_id` | admin | Remove a tab and every reference to it. |
| POST | `/api/display/power/:on` | control | Turn the screen on or off. |
| POST | `/api/display/power/toggle` | control | Turn the screen off or on. Answers `{"screen_on": ...}`. |
| PUT | `/api/display/brightness` | control | Set panel brightness over DDC. |
| POST | `/api/notify`, DELETE `/api/notifications/:id` | control | Raise or dismiss an alert. |
| POST | `/api/webhooks/:name` | control, or the webhook's token | Raise the alert a webhook describes. See [Webhooks](#webhooks). |
| POST | `/api/sidebar/toggle`, `/api/calendar/toggle` | control | Toggle the rail or the full-screen agenda. |
| PUT | `/api/sidebar` | control | Pin the rail open or closed, or hand it back to the notifications. |
| POST | `/api/system/{poweroff,reboot,suspend}` | admin | Power actions over logind. |

Subscribing to a preview starts the capture, and dropping the subscription stops it, so a display
nobody is watching does not encode JPEG in the background. The tab on screen keeps a slow capture
running so `/api/status` always has something recent.

Failures answer with a status code and `{"message": "..."}`. Nothing returns HTTP 200 carrying an
error.

Swagger UI is at `/docs`, and the OpenAPI document at `/docs/spec`.

### The event stream

`GET /api/events` sends two kinds of named frame. Each is sent once on connect, and again
whenever it changes, so a client that holds the stream open never needs to poll.

`event: state` is what is on screen:

```json
{
  "device_id": "lobby-display",
  "device_name": "Lobby Display",
  "current_playlist_id": "lobby",
  "current_tab_id": "overview",
  "auto_rotate": true,
  "next_rotation_at": 1790286710,
  "screen_on": true,
  "brightness": 80,
  "requires_auth": true,
  "access": "control"
}
```

`next_rotation_at` is when rotation next steps, as Unix seconds, and accounts for a hold. It is
`null` while rotation is paused. `access` is what the key the stream was opened with allows:
`admin`, `control` or `none`. A browser `EventSource` sends no header, so it always reads `none`
when a key is configured.

`event: catalogue` is every playlist and its tabs, in play order, and follows every edit:

```json
{
  "playlists": [
    {
      "playlist_id": "lobby",
      "name": "Lobby",
      "tabs": [{ "tab_id": "overview", "name": "Overview", "enabled": true }]
    }
  ]
}
```

## Previews

A preview is the tab's own rendered output, captured over CDP. Two properties follow from that
and are worth knowing before you read a blank thumbnail as a bug.

A page has no frame until it has been rendered at least once, so a tab that has never been on
screen shows nothing. The rotation fixes that within one cycle.

A tab that is not on screen and that no browser is watching stops capturing. Opening the web UI
starts it again, so an unattended display does no JPEG encoding for tabs nobody can see.

## The screen, as opposed to a tab

`GET /api/screen` returns what the compositor is putting on the panel, captured through its own
screencopy protocol rather than through the browser. It includes anything drawn over the page,
which a tab preview cannot show.

It runs the `screenshot` command from `display.toml` and nothing else: no background process, no
stream. Repeated requests inside one second reuse the last capture, so holding the page open
cannot spawn a capture per frame.

The default command is `grim`, which needs to be on the daemon's PATH. Under the NixOS module,
add it to `extraPackages`.

## notifications.toml

```toml
version = 1
mode = "takeover"
default_duration = "20s"
sidebar_width = 480
toast_width = 420
toast_height = 180

[stingers.doorbell]
file = "doorbell.webm"
max_duration = "2s"
```

| Field | Type | Notes |
| --- | --- | --- |
| `mode` | `takeover`, `sidebar` or `toast` | What an alert does by default. A single alert can override it. |
| `default_duration` | duration | How long an alert stays up when the caller does not say. |
| `sidebar_width` | integer | How wide the rail is, in logical pixels. The daemon narrows the display by this much and gives the column the remainder. |
| `toast_width`, `toast_height` | integer | The size of the toast window, in logical pixels. |
| `stingers` | table | Named clips, keyed by the name a tab or an alert refers to. |
| `webhooks` | table | Named alerts a caller raises by URL. See [Webhooks](#webhooks). |

### takeover, sidebar and toast

**Takeover** makes the alert the thing on screen. Rotation stops, and when the alert expires the
display returns to the tab it interrupted rather than to wherever rotation would have reached.

**Sidebar** opens the alert on a rail beside the content and leaves the playlist running.

**Toast** opens it over a corner of the content. Nothing is resized and nothing is interrupted,
which is what a meeting five minutes away is worth. The toast holds the focus while it is up,
because a floating window sits behind a fullscreen one otherwise.

All three are pages the daemon serves to itself. The rail and the toast are second browser windows
in Chromium's app mode rather than layer-shell surfaces, because the compositor is already a tiling
one and speaks an IPC the daemon can drive.

#### How the rail gets its column

Three things have to be true, and the daemon does all of them through niri's IPC. None of them
needs a window rule in your niri configuration.

niri scrolls its columns rather than shrinking them, so a rail that is merely opened lands off the
side of the output. The daemon sets both widths itself: the display gets the output width minus
`sidebar_width`, and the rail gets the rest.

A window that is really fullscreen covers the output rather than sharing it, and Chromium hides its
tab strip and its omnibox only while it believes it is fullscreen. The daemon puts the display into
niri's windowed fullscreen for as long as the rail is open, so the browser keeps drawing no
interface while the compositor keeps tiling it. That is why `chromium.fullscreen` can stay on.

Chromium ignores `--class` on an `--app` window and derives an app id from the URL, so neither
window carries a name you chose. The daemon finds them by the process it started instead, which is
exact and needs nothing configured.

#### Opening and closing the rail by hand

The rail has a mode. `auto` is the default: the rail is up while a sidebar notification is active.
`open` and `closed` pin it, whatever arrives, until the mode is set back to `auto`. The mode is not
saved, so a restart returns to `auto`.

```bash
curl -X PUT http://display.example:3000/api/sidebar \
  -H "authorization: Bearer $MISSIOND_ADMIN_KEY" \
  -H "content-type: application/json" \
  -d '{"mode": "closed"}'
```

`POST /api/sidebar/toggle` pins the rail to the opposite of what is on screen, so a button somewhere
else needs no state of its own. Both calls answer `{"open": ..., "mode": ...}`, and `/api/status`
and the `state` event carry the same object as `sidebar`.

### Stingers

A stinger is a clip played while the screen changes. It is not decoration. A camera feed takes
seconds to connect, and a viewer watching a blank page reads that as broken. The clip goes up
first, the target comes to life behind it, and the clip is taken away once the target is there.

The clip is a window of its own, played by mpv and floated over the display, rather than a page in
the browser. A page has a background where the stream should be, and the browser cannot draw over
a window it does not own.

```toml
[[tabs]]
tab_id = "entrance-camera"
url = "http://camera.example/stream"
stinger = "doorbell"
```

Files live in `media` inside the config directory. That directory can be a Nix store path, which
makes a clip a build input like anything else. `max_duration` cuts the clip off even if it has not
ended, so a mis-encoded file cannot strand the display mid-transition.

Under the NixOS module, `media` is an attribute set keyed by the name a stinger refers to. The
value is any path: a file next to your configuration, or a derivation that produces one.

```nix
services.missiond.settings = {
  notifications.stingers.doorbell = {
    file = "doorbell.webm";
    max_duration = "1500ms";
  };

  media."doorbell.webm" = ./media/doorbell.webm;
};
```

The file has to be in the git tree of the flake it is referenced from, or Nix cannot see it.

#### Transparency

A clip with an alpha channel shows the display coming up through it, which is the point of playing
one over a camera rather than in front of it. Two things have to be true for that to work.

The clip has to carry alpha. VP9 keeps its alpha channel outside the frame and only libvpx's
decoder reads it, which is why the daemon asks mpv for that decoder. `ffprobe` reports
`TAG:alpha_mode=1` on a file that has it. Encoding one:

```bash
ffmpeg -i source.mov -c:v libvpx-vp9 -pix_fmt yuva420p -auto-alt-ref 0 doorbell.webm
```

An encode without `-pix_fmt yuva420p` drops the alpha channel silently, and the clip then covers
the display rather than sitting over it.

The compositor has to leave the window alone. niri draws the focus ring behind a window rather
than around it, so a transparent window shows the ring instead of what is underneath. The clip
window holds the focus while it plays, because that is what keeps it above a fullscreen camera, so
niri needs one rule to stop drawing on it:

```kdl
window-rule {
    match app-id="missiond-stinger"
    focus-ring {
        off
    }
    border {
        off
    }
}
```

Without the rule the clip still plays and still covers the transition; its transparent parts show
niri's ring colour rather than the camera.

### Raising an alert

```bash
curl -X POST http://display.example:3000/api/notify \
  -H "authorization: Bearer $MISSIOND_ADMIN_KEY" \
  -H 'content-type: application/json' \
  -d '{
        "title": "Entrance",
        "body": "Someone is at the door",
        "level": "warning",
        "tab_id": "entrance-camera",
        "stinger": "doorbell",
        "duration": "30s"
      }'
```

| Field | Notes |
| --- | --- |
| `title`, `body` | What the alert card shows. `body` is optional. |
| `level` | `info`, `warning` or `critical`. Colours one edge of the card. |
| `mode` | Overrides `mode` for this alert. |
| `duration` | Overrides `default_duration`. |
| `tab_id` | Show this tab instead of a card, so an alert about a camera puts the stream on screen rather than a sentence describing it. |
| `stinger` | A clip to cover the change. |
| `key` | Two calls carrying one key are one alert said twice: the second replaces the first rather than stacking beside it. An automation that fires on every door event gets this for free. |
| `starts_at`, `ends_at` | RFC 3339. What the alert is about, so the card shows a time and counts down to it. |
| `location` | A room, shown under the title. |

The call returns as soon as the alert is queued. A transition can take seconds and the caller is
usually an automation, so it is not held open while the screen changes.

### Webhooks

A webhook is an alert written down in advance, raised by a request that carries nothing but its
name. It is for a service that can call a URL when something happens but cannot shape the
request: a doorbell, an alarm panel, a monitoring system.

```toml
[webhooks.doorbell]
token = { file = "/run/secrets/doorbell-webhook" }
title = "Someone is at the door"
level = "warning"
mode = "takeover"
tab_id = "entrance-camera"
stinger = "doorbell"
duration = "30s"
```

```bash
curl -X POST http://display.example:3000/api/webhooks/doorbell \
  -H "authorization: Bearer $DOORBELL_WEBHOOK_TOKEN"
```

| Field | Notes |
| --- | --- |
| `token` | A [secret](#secrets). The bearer this webhook accepts. Optional. |
| `title`, `body`, `level`, `mode`, `tab_id`, `stinger`, `duration` | As for [raising an alert](#raising-an-alert). |

The request body is never read, so a caller that posts its own payload works unchanged. The call
answers once the alert is queued, as `/api/notify` does.

A webhook with a `token` accepts that token as its bearer and nothing else, not even the admin
key, and it does so whether or not an admin key is configured. That gives the caller one
credential that raises one alert. A webhook without a `token` is a control route: it needs the
admin or control key when an admin key is configured, and nothing otherwise.

Each webhook's alert carries the key `webhook:<name>`, so a doorbell pressed twice replaces its
alert and restarts its duration rather than stacking a second one.

An export replaces an inline token with `MISSIOND_WEBHOOK_<NAME>`, the name upper-cased with `-`
as `_`.

| Method | Path | Does |
| --- | --- | --- |
| POST | `/api/notify` | Raise an alert. |
| POST | `/api/webhooks/:name` | Raise the alert a webhook describes. |
| GET | `/api/notifications` | What is currently showing. |
| GET | `/api/notifications/stream` | The same list as a server sent event stream. The alert pages read this. |
| DELETE | `/api/notifications/:notification_id` | Clear one early. |
| POST | `/api/sidebar/toggle` | Pin the rail open if it is closed, closed if it is open. |
| PUT | `/api/sidebar` | Set the mode: `{"mode": "auto" \| "open" \| "closed"}`. |
| GET | `/api/sidebar` | Whether the rail is up, and its mode. |
| POST | `/api/calendar/toggle` | Put the full-screen agenda on the display, or take it away. |
| GET | `/api/calendar/agenda` | The entries the feeds put in their window. |
| GET | `/api/stingers` | The configured clips, so the transition page can resolve a name. |
| GET | `/api/media/:name` | A file from the config directory's `media`. |

## calendars.toml

```toml
version = 1
poll = "1m"

[[calendars]]
calendar_id = "work"
name = "Work"
refresh = "15m"
window = "12h"
leads = ["5m", "0s"]
toast_duration = "45s"

[calendars.url]
file = "/run/secrets/work-ics"
```

| Field | Type | Notes |
| --- | --- | --- |
| `poll` | duration | How often the rail is reconciled against what has already been fetched. |
| `calendar_id` | string | Names the feed, and is part of every key its entries carry. |
| `name` | string | What a message about the feed calls it. Defaults to the id. |
| `url` | secret | Where the `.ics` is. See [Secrets](#secrets). |
| `refresh` | duration | How often the daemon goes back to the network. |
| `window` | duration | How far ahead the rail reaches, and how far recurrence expansion runs. |
| `leads` | list of durations | How long before an entry starts to raise a toast. One toast per lead. |
| `toast_duration` | duration | How long each of those toasts stays up. |

A calendar's `.ics` link is a bearer credential: anyone holding it reads the calendar. So `url`
takes the same reference an RTSP camera does, and `GET /api/config/export` replaces an inline one
with `MISSIOND_CALENDAR_<ID>` rather than printing it.

`poll` and `refresh` are separate on purpose. The rail has to stay correct to the minute, because
"in 3 minutes" is wrong a minute later, and asking a calendar server that often to learn something
it has not changed would be rude and slower than reading what is already in memory.

### Meeting links

An entry that names a video call carries its provider, and the rail draws that provider's icon
beside the title.

The link is looked for in three places, in this order, because they are not equally trustworthy. A
dedicated conference property, `X-GOOGLE-CONFERENCE` or the `CONFERENCE` of RFC 7986, holds a link
and nothing else. `LOCATION` holds one often enough to be worth reading, but a real feed also puts
prose there, and "Find the Zoom link on Discord" is a location rather than a url. `DESCRIPTION` is
mostly prose that happens to contain links, and the first one is as likely to be an issue tracker
as a meeting, so a link found there counts only when a provider claims it.

A provider is a name and a list of host patterns:

```toml
[meetings.providers]
jitsi = ["meet.jit.si", "*.jitsi.example.org"]
```

`*.example.com` matches any subdomain and not `example.com` itself, which is why a provider that
answers on both lists both. The wildcard is anchored to a label boundary, so `*.zoom.us` does not
claim `evilzoom.us`.

These are recognised without any configuration:

| Provider | Hosts |
| --- | --- |
| `zoom` | `zoom.us`, `*.zoom.us` |
| `meet` | `meet.google.com` |
| `jitsi` | `meet.jit.si`, `*.jit.si` |
| `webex` | `webex.com`, `*.webex.com` |
| `teams` | `teams.microsoft.com`, `teams.live.com` |

`providers` is merged over that table by name, so naming one does not mean restating the rest.
That is what makes a custom domain a single line: a self-hosted Jitsi, or a Zoom vanity domain
that is not a `zoom.us` subdomain, replaces only its own entry. Setting a name to `[]` turns that
provider off.

A name missiond has no icon for still reaches the page and renders with a generic camera, so a
provider can be added here alone. A link no pattern claims keeps its url and gets no icon: the
entry is still a meeting, missiond just cannot say whose.

### What reaches the screen

An entry is on the rail from the moment it falls inside `window` until it ends, which makes this an
agenda rather than a stack of reminders. Each entry drops itself the minute its meeting finishes,
whether or not a poll runs first.

One meeting that two feeds both carry is one entry. Two people sharing a display are invited to the
same thing often, and the copies do not agree on a `UID` once a calendar has exported them, so what
a viewer can see decides it: the title, the start, the end, the location and the join link. The feed
listed first in `calendars.toml` keeps the entry, and the copy is dropped along with its toasts.

The rail draws the day on a time axis rather than as a list. Vertical position is the clock and
height is the length of the meeting, at one scale from the top of the rail to the bottom, so an hour
of waiting takes an hour of room and the gap before the afternoon is drawn rather than described.
The axis starts at the current time and reaches the end of the last entry, and never covers less
than two hours, so one short meeting does not fill the panel on its own.

Meetings that overlap share the width, one lane each, which is what makes a clash read as a clash. A
meeting too short to hold a line of text is given one anyway, so a ten minute slot keeps its title
and loses its second line.

A meeting that has started is lifted off the axis to the top of the rail, with a bar for how much of
it is left. Its start is behind us, so how far away it is has stopped being the question.

A toast goes up at each of `leads`. With the default `["5m", "0s"]` that is one five minutes out and
one as the meeting begins. The window a toast is up for is absolute, `start - lead` to
`toast_duration` later, rather than a crossing the daemon has to remember, so a restart at four
minutes past arrives at the same answer as a daemon that has been running all morning.

Recurrence is expanded in the feed's own timezone. A rule expanded in UTC and converted afterwards
drifts by an hour at each daylight saving boundary, so a standup that is 09:00 all year would start
reading as 08:00 from late October.

### When a feed stops answering

The last body that parsed is kept under the cache directory, one file per `calendar_id`, and a
failed fetch falls back to it. A display is on a wall and a network blip is common: an agenda that
blanks itself for a minute reads as broken in a way that a fifteen minute old one does not.

While the daemon is serving a cached body it puts one row on the rail saying so. A feed that has
never fetched successfully has nothing to fall back to, and an empty rail is indistinguishable from
an empty afternoon, so that case says so too.

### The full-screen agenda

```bash
curl -X POST http://display.example:3000/api/calendar/toggle \
  -H "authorization: Bearer $MISSIOND_ADMIN_KEY"
```

The first call puts the whole day on the display and holds it there. The hold has no end, so it
stays until the second call rather than for a duration the caller had to guess. Ending it returns
to the tab the playlist was showing, the same way a takeover does.
