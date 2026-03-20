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

### Graph

Centered review heatmap.

```yml
- type: extension
  url: http://localhost:8080/graph
  allow-potentially-dangerous-html: true
  parameters:
    deck: Japanese         # omit to show all decks
    days: 30               # number of past days to display
    background-color: "#1d2025"  # cell background
    primary-color: "#f3afaf"     # cell foreground
    svg-height: 150
    font-size: 9
    transition-hue: false  # if true, interpolates hue between bg and primary
```

### Stats

Statistics summary.

```yml
- type: extension
  url: http://localhost:8080/stats
  allow-potentially-dangerous-html: true
  parameters:
    deck: Japanese
    days: 30
    show_quartiles: true
```

### Graph SVG

Just the raw SVG graph.

```yml
- type: extension
  url: http://localhost:8080/graph_svg
  allow-potentially-dangerous-html: true
  parameters:
    ... # accepts all the same parameters as /graph
```
