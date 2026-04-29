# Project Chromancy
A WLED client written in Rust. 
For general questions about WLED, see https://kno.wled.ge
For technical questions about the code behind WLED, see https://github.com/WLED/WLED
My favorite hardware platform to run LED code 

A Rust client library designed to be used to implement things such as (in priority order):
1. LLM tools for agents e.g. MCP or just plain old command line
2. LLM SDK for agnets
3. Human command lines

## Core Design Goals

1. Mirror WLED's JSON API closely (makes docs/debugging easier)
2. Rust-idiomatic (Result types, builder pattern where useful)
3. Async-first (all network calls are non-blocking)
4. Clear error types (distinguish network vs. API vs. config errors)
5. Extensible (new WLED features = new methods, not rewrites)

## MCP Tools Design

Here are the tools your MCP server would expose to Claude (or any MCP client):

Tool Name	Description	Parameters
---- ----   ----------- ----------
get_device_info	Get WLED device capabilities & status	device_id
get_state	Get current light state	device_id
set_power	Turn lights on/off	device_id, on: bool
set_brightness	Set brightness level	device_id, brightness: 0-255
set_color	Set primary color	device_id, r, g, b
set_effect	Change active effect	device_id, effect_id
set_palette	Change color palette	device_id, palette_id
set_segment	Configure LED segment	device_id, segment_config
list_effects	List available effects	device_id
list_palettes	List available palettes	device_id

## Summary: API Surface

Category	Methods
Construction	new(), builder()
Queries	get_state(), get_info(), get_full_state(), list_effects(), list_palettes(), get_palette_colors(), ping()
Mutations	set_power(), set_brightness(), set_color(), set_effect(), set_palette(), set_transition(), set_state()
Escape Hatch	raw_request()

### Why This API Design Works for MCP 
These are assumptions worth testing later on through LLM usage

One method per MCP tool → Clean 1:1 mapping
Consistent error types → Easy to translate to McpError
Partial updates via WledStateRequest → Efficient, mirrors WLED's API
raw_request() as escape hatch → Future WLED features don't require client rewrites
Builder pattern for config → API keys, timeouts, etc. without constructor bloat

## Security Considerations

Since this may touch your home network, which we should be very security conscious about:

- No external exposure: Server runs locally, only talks to your LAN
- Config file permissions: Keep config.toml readable only by you
- Optional auth: WLED supports optional API keys—could add that layer
- Read-only mode: Could run in a mode that only queries state, no writes

