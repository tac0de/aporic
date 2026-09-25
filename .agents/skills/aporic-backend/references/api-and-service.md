# API and service contracts

Trace the actual route: MCP or CLI input → Hub service → Store and kernel boundary → response. Define the accepted request shape, bounds, idempotency key behavior, error cases, and output size before editing the adapter. Keep transport handling out of the domain model.

For a new or changed endpoint, inspect existing `domain.rs`, `hub.rs`, `mcp.rs`, and CLI conventions. Check that invalid and oversized inputs fail clearly, duplicate requests return the intended previous result, and workspace or task scope cannot be confused. Preserve existing clients' serialized names and response meaning unless a versioned change is deliberate. Update tool descriptions and examples when behavior changes.

Use a real MCP stdio or CLI integration test when wiring is the risk. Unit tests of the Hub alone do not show that the public adapter exposes the contract correctly. A test pass establishes that scenario, not general API quality.
