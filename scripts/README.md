# Development Scripts

This directory contains utility scripts for development, debugging, and maintenance of the lair-chat project.

## Available Scripts

### 🐛 `debug_client.sh`

**Purpose**: Debug script for comprehensive client logging and troubleshooting.

**Usage**:
```bash
./scripts/debug_client.sh
```

**Features**:
- Sets optimal debug environment variables (`RUST_LOG=trace`, `RUST_BACKTRACE=1`)
- Captures all output to `debug_output.log` for analysis
- Provides helpful grep commands for log analysis
- Automatically timestamps debug sessions

**Output**: Creates `debug_output.log` in the current directory with:
- Session metadata (timestamp, environment, build info)
- Complete client execution logs
- All debug traces and error information

**Example Usage**:
```bash
cd lair-chat
./scripts/debug_client.sh

# After session ends, analyze logs:
grep 'ERROR:' debug_output.log
grep 'transport' debug_output.log
```

### 📋 `show_real_logs.sh`

**Purpose**: Locates and displays actual lair-chat log files from the data directory.

**Usage**:
```bash
./scripts/show_real_logs.sh
```

**Features**:
- Automatically finds log files in `~/.config/the-lair-chat/data/`
- Shows recent log entries (last 50 lines)
- Filters for transport-related debug information
- Displays message queue operations
- Provides helpful commands for log analysis

**Log Locations**:
- Primary: `~/.config/the-lair-chat/data/lair-chat-client.log`
- Fallback: Searches common locations if primary not found

**Example Output Sections**:
- Recent log entries
- Transport debug logs
- Message queue operations
- Full log access commands

## Development Workflow

### For Debugging Client Issues

1. **Start Debug Session**:
   ```bash
   ./scripts/debug_client.sh
   ```

2. **Reproduce Issue**: Use the client normally to trigger the problem

3. **Analyze Logs**: Review `debug_output.log` for relevant information

4. **Check Persistent Logs**:
   ```bash
   ./scripts/show_real_logs.sh
   ```

### For Log Analysis

Common log analysis patterns:

```bash
# Find connection issues
grep -i "connect\|disconnect\|timeout" debug_output.log

# Find authentication problems
grep -i "auth\|login\|credential" debug_output.log

# Find message handling issues
grep -i "message\|send\|receive" debug_output.log

# Find transport layer issues
grep -i "transport\|tcp\|socket" debug_output.log
```

## Environment Variables

Both scripts respect and utilize these environment variables:

| Variable | Purpose | Default |
|----------|---------|---------|
| `RUST_LOG` | Logging level | `trace` (set by debug script) |
| `RUST_BACKTRACE` | Stack trace detail | `1` (set by debug script) |
| `HOME` | User home directory | System default |

## Log Levels

Understanding Rust log levels for effective debugging:

- **`error`**: Critical errors that prevent operation
- **`warn`**: Warning conditions that might indicate problems
- **`info`**: General information about program operation
- **`debug`**: Detailed information for debugging
- **`trace`**: Very detailed tracing information

## Troubleshooting

### No Logs Generated

If scripts don't find any logs:

1. **Check if client ran successfully**:
   ```bash
   cargo run --bin lair-chat-client
   ```

2. **Verify log directory exists**:
   ```bash
   ls -la ~/.config/the-lair-chat/data/
   ```

3. **Check environment variables**:
   ```bash
   echo $RUST_LOG
   echo $RUST_BACKTRACE
   ```

### Permission Issues

If you encounter permission errors:

```bash
# Make scripts executable
chmod +x scripts/*.sh

# Check directory permissions
ls -la ~/.config/the-lair-chat/
```

### Large Log Files

For managing large log files:

```bash
# Check log file size
du -h ~/.config/the-lair-chat/data/lair-chat-client.log

# Rotate logs (backup and truncate)
cp ~/.config/the-lair-chat/data/lair-chat-client.log lair-chat-backup.log
> ~/.config/the-lair-chat/data/lair-chat-client.log
```

## Adding New Scripts

When adding new development scripts:

1. **Place in this directory**: `scripts/`
2. **Make executable**: `chmod +x scripts/your_script.sh`
3. **Add documentation**: Update this README
4. **Follow naming convention**: Use descriptive, lowercase names with underscores
5. **Include help text**: Add usage information in script comments

### Script Template

```bash
#!/bin/bash
# Description of what this script does
# Usage: ./scripts/script_name.sh [arguments]

set -e  # Exit on error

echo "Script starting..."

# Your script logic here

echo "Script completed successfully"
```

## Related Documentation

- [Development Guide](../docs/development/DEVELOPMENT_GUIDE.md)
- [GitHub CI Guide](../docs/development/GITHUB_CI_GUIDE.md)
- [Architecture Documentation](../docs/architecture/README.md)
- [Troubleshooting Guide](../docs/guides/TROUBLESHOOTING.md)

---

**Maintenance**: Update this README when adding, removing, or modifying scripts.
**Support**: For issues with these scripts, check the main project documentation or open an issue.