# Keyboard Research Kit

My toolkit for keyboard layout analysis. Currently implement only keylogger for linux.

## Usage

Start keylogger (run as `sudo` or add user to `input` group):
```bash
# by device name
nix run . -- key-logger -d "Topre Corporation HHKB Professional"
# by full path
nix run . -- key-logger -d "/dev/input/event6"
# you can add output file by using -o option (by default "./keylogger.log" used)
nix run . -- key-logger -d "/dev/input/event6" -o "/tmp/keylogger.log"
```

