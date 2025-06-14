# Font Compatibility Guide for Lair Chat

## Overview

Lair Chat uses Unicode symbols and emojis to enhance the user experience with visual indicators for notifications, status, and navigation. This guide helps ensure optimal display across different terminals and operating systems.

## Symbols Used

### Emoji Characters
- 🔔 **Bell** - DM notifications and unread message indicators
- 💬 **Speech Bubble** - Direct message headers and notifications
- 🏠 **House** - Lobby chat mode indicator
- 📤 **Outbox Tray** - Debug logging (sent messages)
- 📥 **Inbox Tray** - Debug logging (received messages)

### Unicode Symbols
- ● **Black Circle** - Online status, connected state
- ○ **White Circle** - Offline/away status
- ◐ **Half Circle** - Idle status
- ✖ **Cross Mark** - Banned/error status
- ← **Left Arrow** - User left indicator
- • **Bullet** - List items and help text

## Recommended Fonts

### Linux
```bash
# Install Noto Color Emoji
sudo apt install fonts-noto-color-emoji

# Install JetBrains Mono Nerd Font
wget https://github.com/ryanoasis/nerd-fonts/releases/download/v3.0.2/JetBrainsMono.zip
unzip JetBrainsMono.zip -d ~/.local/share/fonts/
fc-cache -fv
```

### macOS
```bash
# Install via Homebrew
brew tap homebrew/cask-fonts
brew install --cask font-jetbrains-mono-nerd-font

# Apple Color Emoji is included by default
```

### Windows
```powershell
# Install via Chocolatey
choco install jetbrainsmono-nf

# Or download manually from:
# https://github.com/ryanoasis/nerd-fonts/releases
```

## Terminal Configuration

### Popular Terminals

#### Alacritty
```yaml
# ~/.config/alacritty/alacritty.yml
font:
  normal:
    family: "JetBrainsMono Nerd Font"
  size: 12.0
```

#### Kitty
```ini
# ~/.config/kitty/kitty.conf
font_family      JetBrainsMono Nerd Font
font_size        12.0
```

#### Windows Terminal
```json
{
    "profiles": {
        "defaults": {
            "fontFace": "JetBrainsMono Nerd Font"
        }
    }
}
```

#### iTerm2 (macOS)
1. Preferences → Profiles → Text
2. Font: Select "JetBrainsMono Nerd Font"
3. Enable "Use built-in Powerline glyphs"

## Testing Font Support

### Quick Test
Run this command to test symbol support:
```bash
echo "Status: ● ○ ◐ ✖ ← | DM: 🔔 💬 🏠"
```

### Expected Output
You should see:
- Filled and empty circles for status
- Half-circle for idle
- Cross and arrow symbols
- Bell, speech bubble, and house emojis

### If Symbols Don't Display
- **Squares or question marks**: Font lacks Unicode support
- **Missing emojis**: Terminal doesn't support color emojis
- **Overlapping characters**: Font spacing issues

## Fallback Behavior

Lair Chat remains fully functional without proper emoji support:

### What Still Works
- All messaging functionality
- DM conversations and switching
- Status bar information
- Navigation and controls
- User lists and room management

### Visual Fallbacks
- DM notifications show as text: "New DM from username"
- Status indicators use text: "ONLINE" / "OFFLINE"
- Mode headers show as: "LOBBY CHAT" / "DIRECT MESSAGE"
- Unread counts display as: "(3 unread)"

## Troubleshooting

### Common Issues

#### Emoji Not Displaying
```bash
# Check emoji support
fc-list | grep -i emoji

# Install emoji fonts
sudo apt install fonts-noto-color-emoji  # Ubuntu/Debian
sudo dnf install google-noto-emoji-fonts # Fedora
```

#### Incorrect Spacing
- Use monospace fonts with proper Unicode width support
- Try different Nerd Font variants
- Adjust terminal font size

#### Color Issues
- Ensure terminal supports 256 colors or true color
- Check `$TERM` environment variable
- Try: `export TERM=xterm-256color`

### Terminal-Specific Fixes

#### tmux
```bash
# Add to ~/.tmux.conf
set -g default-terminal "screen-256color"
set -ga terminal-overrides ",*256col*:Tc"
```

#### SSH Sessions
```bash
# Forward locale for proper Unicode support
ssh -o SendEnv=LANG,LC_* user@host
```

## Verification Script

Save this as `test_fonts.sh`:
```bash
#!/bin/bash
echo "=== Lair Chat Font Compatibility Test ==="
echo ""
echo "Unicode Symbols:"
echo "  Status: ● (online) ○ (offline) ◐ (idle)"
echo "  Actions: ✖ (banned) ← (left)"
echo ""
echo "Emojis:"
echo "  Notifications: 🔔 (bell)"
echo "  Chat: 💬 (speech) 🏠 (lobby)"
echo "  Messages: 📤 (sent) 📥 (received)"
echo ""
echo "If all symbols display correctly, your font setup is optimal!"
echo "If some appear as squares □ or question marks ?, install a Nerd Font."
```

## Font Recommendations by Use Case

### Development
- **JetBrains Mono Nerd Font** - Excellent code readability
- **Fira Code Nerd Font** - Great ligature support
- **Hack Nerd Font** - Clean and distinctive

### General Use
- **DejaVu Sans Mono** - Wide Unicode support
- **Liberation Mono** - Good compatibility
- **Inconsolata** - Readable and compact

### High DPI Displays
- **SF Mono** (macOS) - Optimized for Retina displays
- **Cascadia Code** (Windows) - Modern and crisp
- **Source Code Pro** - Adobe's professional choice

## Performance Considerations

### Emoji Rendering
- Color emojis may slightly impact performance
- Monochrome alternatives available in most fonts
- Terminal rendering speed varies by implementation

### Font Loading
- Nerd Fonts are larger files (~50MB+)
- May increase terminal startup time slightly
- Benefits outweigh minimal performance cost

## Best Practices

1. **Use a Nerd Font** for complete symbol coverage
2. **Test in your primary terminal** before deploying
3. **Set consistent font sizes** (10-14pt recommended)
4. **Enable Unicode support** in terminal settings
5. **Configure proper locale** (UTF-8 encoding)

## Support

If you encounter font-related issues:

1. Test with the verification script above
2. Check your terminal's font settings
3. Verify Unicode/UTF-8 support
4. Try a different Nerd Font variant
5. Report persistent issues on the GitHub repository

Remember: Lair Chat prioritizes functionality over visual flair. Even with basic fonts, all features work perfectly!