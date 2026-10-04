# Autostart

## Short version

**You probably don't need autostart.** Claude Desktop and Claude Code spawn
selfloom themselves over stdio whenever they launch — the server exits with
them. That's by design and it's the right default.

The plist / service files in this folder exist for the day selfloom grows an
HTTP mode, or for the small number of people who want it running as a
long-lived daemon for other reasons.

## macOS (launchd)

See `com.selfloom.plist`. Edit the two placeholder paths, drop it in
`~/Library/LaunchAgents/`, and `launchctl load` it.

## Linux (systemd user service)

See `selfloom.service`. Edit the two placeholder paths, drop it in
`~/.config/systemd/user/`, and `systemctl --user enable --now` it. Run
`loginctl enable-linger "$USER"` so it survives logout.

## Windows

No service file yet. If you want it on boot, put a shortcut in
`shell:startup` that runs:

```
"C:\\path\\to\\loom.exe" --vault "C:\\path\\to\\vault" serve
```

Again — only if you actually need it running in the background.
