# RPC

This crate defines the RPC protocol used by `unnamed` (placeholder).

RPC is the protocol; IPC is the transport.

In practice, the server and client should communicate over Unix Socket (IPC) to achieve
high-performance communication.

It allows one server multiple clients (multiple users) architecture.

TODO Information like server status (currently indexed files/file name) should be exchanged
though shared memory (possibly using crate `memmap2`).
