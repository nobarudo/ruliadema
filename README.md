# Ruliadema

A simple, fast, and beautiful HTTP monitoring daemon & TUI dashboard written in Rust.

## Features

- **Daemon Mode**: Continuously monitors target URLs in the background.
- **SLA Tracking**: Define acceptable latency limits per URL.
- **Smart Logging**: 
  - Maintains a real-time state snapshot (`status.json`).
  - Appends permanent history in JSON Lines format (`ruliadema.log`).
- **TUI Dashboard**: A beautiful, real-time terminal user interface to visualize response times, SLA diffs, and inspect configurations.

## Build

To build the project for production, run:

```bash
cargo build --release
```

The optimized binaries will be available in the `target/release/` directory.

## Configuration

Create a `config.toml` file in the same directory as the binaries.

```toml
interval_seconds = 30
timeout_seconds = 5
max_concurrency = 2

[[targets]]
url = "[https://example.com](https://example.com)"
acceptable_latency_ms = 500

[[targets]]
url = "[https://google.com](https://google.com)"
acceptable_latency_ms = 200

```

## Usage

### 1. Start the Daemon

Run the monitoring daemon in the background (or in a separate terminal/tmux pane).

```bash
./target/release/ruliadema

```

### 2. Start the TUI Viewer

Open the real-time dashboard.

```bash
./target/release/view

```

### Viewer Keybindings

* `j` / `k` or `↓` / `↑` : Select Target URL
* `c` : View `config.toml` contents
* `Esc` : Close config view and return to dashboard
* `q` : Quit

## Generated Files

* `status.json`: Contains the latest 50 check results for the TUI viewer. Overwritten automatically.
* `ruliadema.log`: Append-only JSONL log file for permanent record and external analysis.
