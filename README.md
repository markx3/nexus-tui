# Nexus

Cyberpunk TUI session manager for [Claude Code](https://docs.anthropic.com/en/docs/claude-code).

## Install

```sh
cargo install --path .
```

Requires [tmux](https://github.com/tmux/tmux) for embedded session management.

## Terminal Configuration (macOS)

Nexus uses **Alt+key** shortcuts for navigation and commands. On macOS, the Option key produces special characters by default (e.g., Option+M types `µ`) instead of the Alt/Meta sequences that terminal applications expect.

You **must** configure your terminal to treat Option as Meta/Alt, or Alt keybindings will not work.

### Ghostty

Add to `~/.config/ghostty/config`:

```
macos-option-as-alt = true
```

Restart Ghostty after changing this setting.

### iTerm2

Settings > Profiles > Keys > General > Left Option key: **Esc+**

(Repeat for Right Option key if desired.)

### Terminal.app

Settings > Profiles > Keyboard > **Use Option as Meta key**

### Kitty

Add to `~/.config/kitty/kitty.conf`:

```
macos_option_as_alt yes
```

### Alacritty

No configuration needed -- Alacritty treats Option as Alt by default on macOS.

### WezTerm

Add to `~/.config/wezterm/wezterm.lua`:

```lua
config.send_composed_key_when_left_alt_is_pressed = false
config.send_composed_key_when_right_alt_is_pressed = false
```
