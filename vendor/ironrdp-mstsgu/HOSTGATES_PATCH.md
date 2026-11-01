IronRDP MSTSGU 0.1.0 from crates.io, upstream revision b13befa0db86f48f523a0b2b007e399f9811e0ec, MIT/Apache-2.0.
Local adaptations: SSPI 0.21 and picky rc.25 to match the existing IronRDP connector dependency graph; redact GwConnectTarget debug output; relocate upstream public test certificates into tests/certs. These certificates and their public fixture key are for loopback tests only, never production credentials.
Smart-card feature is not enabled. Upstream protocol/auth/TLS tests are used to validate the SSPI compatibility adaptation.

Compatibility fix (2026-11-01): when the gateway closes an HTTP authentication exchange during the initial WebSocket transport attempt, retry once on fresh strictly validated TLS channels without the WebSocket upgrade. Authentication state is recreated. Explicit HTTP authentication refusals and TLS validation errors do not trigger this retry. A loopback regression reproduces the abrupt close and verifies the successful dual-HTTP retry.

Interactive latency: enable TCP_NODELAY before TLS on outgoing TCP sockets (direct RDP for client; gateway and proxy sockets for MSTSGU).
