# Keyboard Research Kit

My toolkit for keyboard layout analysis. Currently implement only keylogger for linux.

## Usage

Start keylogger (run as `sudo` or add user to `input` group):
```bash
# by device name
nix run . -- \
  key-logger start \
  --device "Topre Corporation HHKB Professional"
# by full path
nix run . -- \
  key-logger start \
  --device "/dev/input/event6"
# you can specify output directory
nix run . -- \
  key-logger start \
  --device "/dev/input/event6" \
  --output-directory "./output"
# and specify output file format (support binary and json, binary is default)
nix run . -- \
  key-logger start \
  --device "/dev/input/event6" \
  --output-format "json"
```

Convert log format:
```bash
# binary to json
nix run . -- \
  key-logger convert-log \
  --input-format "binary" "output/2024-11-04T14:48:05.log" \
  --output-format "json" "output/2024-11-04T14:48:05.log.json"
```
