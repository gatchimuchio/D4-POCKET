# Agent Orchestration

This pure Dart package contains the shared Agent Task client, Task monitor,
MCP proposal controller, and result-dependent continuation flow. It has no
Flutter or `dart:ui` dependency. Desktop UI code adapts the host OS value and
existing authenticated Broker transport at the application boundary.

The package does not grant Permission or Approval, store credentials, execute
MCP tools directly, or finalize Audit. Sensitive operations are requests to
`BrokerTransport`; the Broker revalidates its current state and remains the
authority source. A fake transport can exercise the consumer contract but is
not evidence of a live Broker or installed product path.

An independent Dart host can depend on this package and provide its own
`BrokerTransport` implementation. That implementation must use an existing
authenticated Broker route; implementing this interface alone grants no
execution authority.
