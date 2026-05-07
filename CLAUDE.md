# Chromancy — Claude-Specific Notes

Full project documentation is in [AGENTS.md](AGENTS.md). This file contains only Claude-specific configuration.

## Claude Desktop Integration

Start the MCP server for use with Claude Desktop:

```bash
cargo run
```

The server communicates via stdio. Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "chromancy": {
      "command": "/path/to/chromancy/target/release/chromancy",
      "args": []
    }
  }
}
```

## Claude Code

- `AGENTS.md` is the primary project reference — read it first
- `TASKS.md` tracks current work assignments
- `INTEGRATION_CONTRACTS.md` defines public API boundaries
- Real device config lives in `wled-config.toml` (gitignored); copy from `wled-config.toml.example`
