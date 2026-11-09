# Broker Contract

This pure Dart package defines the request port shared by Broker consumers and
the canonical JSON request envelope used by the Flutter client. The payload
hash binds data for consistency; it is not authentication and does not grant
Permission, Approval, or authority.

Each host must use an existing authenticated Broker transport. This package
does not read endpoint files or credentials and does not open sockets, launch
processes, or decide whether an operation is authorized. The Broker remains
responsible for authentication, current Permission and Approval checks, Audit,
and operation execution.
