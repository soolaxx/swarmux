# Swarmux setup

## tmux F8 popup

Use this binding:

```tmux
bind -n F8 display-popup -T "Swarmux" -w 90% -h 80% -E "sh -lc 'swarmux popup --once; printf \"\\nPress Enter to close...\"; read _'"
```

Reload tmux:

```bash
tmux source-file ~/.config/tmux/tmux.conf
```
