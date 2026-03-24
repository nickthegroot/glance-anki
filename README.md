# glance-anki

A lightweight [Glance](https://github.com/glanceapp/glance) extension that displays Anki review activity as a contribution-style heatmap.

## Installation

### NixOS module

```nix
# flake.nix
inputs.glance-anki.url = "github:nickthegroot/glance-anki";

# configuration.nix
imports = [ inputs.glance-anki.nixosModules.default ];

services.glance-anki = {
  enable = true;
  collectionPath = "/home/alice/.local/share/Anki2/User 1/collection.anki2";
};
```

### Nix run

```bash
ANKI_COLLECTION_PATH=~/.local/share/Anki2/"User 1"/collection.anki2 nix run github:nickthegroot/glance-anki
```

### Cargo

```bash
ANKI_COLLECTION_PATH=~/.local/share/Anki2/"User 1"/collection.anki2 cargo run --release
```

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `ANKI_COLLECTION_PATH` | `collection.anki2` | Path to your `collection.anki2` file. |
| `DEFAULT_DAYS` | `30` | Fallback window size when no `?days=` param is given. |
| `RUST_LOG` | *(unset)* | Log level, e.g. `info` or `debug`. |

## Glance configuration

> [!TIP]
> All query parameters are optional, with sane defaults.

All parameters use `kebab-case`.

### Shared parameters

| Parameter | Default | Description |
|---|---|---|
| `deck` | all decks | Deck name to filter by. Includes sub-decks. |
| `days` | `30` | Number of past days to display. |
| `timezone` | server timezone | IANA timezone string (e.g. `America/New_York`). Used to compute the daily rollover boundary. |

### Graph

Review heatmap. Cell colors are driven by your Glance theme's primary color automatically.

```yml
- type: extension
  url: http://localhost:8080/graph
  allow-potentially-dangerous-html: true
  parameters:
    deck: Japanese
    days: 30
```
