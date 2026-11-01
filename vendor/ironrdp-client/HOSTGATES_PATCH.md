IronRDP client 0.1.0 from crates.io, MIT/Apache-2.0. Local modifications: upgrade ironrdp-tls to 0.3; strict TLS with explicit certificate-decision callback; original server name for SSH forwarding; bounded input channel (512 events). No invalid certificate accepted by default. Optional clipboard and DVC plugin features are not enabled or supported by this adaptation. Upstream: https://github.com/Devolutions/IronRDP

Gateway integration: ironrdp-mstsgu 0.1.0 local dependency, explicit strict outer TLS with the certificate callback, original target port, no smart-card credentials, redacted GatewayConfig Debug. Gateway feature is enabled alongside rustls.

RPCH integration: use the authenticated RpchStream (not the low-level unsigned RpchClient) with strict TLS, target port preservation and the original inner RDP certificate policy. Select directly with Config.gateway_rpc or fall back once after GwErrorKind::HttpConnectionClosed; no fallback after explicit auth or certificate failures.

RDS: Config::set_load_balance_info validates the imported routing string and
sets the connector request data using the local PDU encoder. This is shared
by direct, HTTP gateway and RPCH connection paths. RDS V4 enhanced-security redirection is handled via the same gateway, with fresh
TLS validation for the selected target, session ID and opaque cookie forwarding,
TSV collection consistency checks, a four-hop limit and loop detection.
Smart cards/RDSTLS redirection are explicitly unsupported. Fixed SSH tunnels do
not enable this feature.

Interactive latency: enable TCP_NODELAY before TLS on outgoing TCP sockets (direct RDP for client; gateway and proxy sockets for MSTSGU).
