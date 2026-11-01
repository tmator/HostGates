# HostGates connector patch

Source: ironrdp-connector 0.10.0, upstream revision
11a0810cfbbabd8b8023875a05e3041216d4b01b (Devolutions/IronRDP).
MIT OR Apache-2.0; original license files are included.

During capabilities exchange, process four-byte Status Info notifications and
Set Error Info (no error) while continuing to wait for Demand Active. Report
nonzero server errors by their symbolic code and description. Reject other
unexpected Data PDUs with their specific type, without logging their payload.
This does not bypass authentication, certificate validation or server refusals.
Unit tests enabled for these cases. The root Cargo patch applies to all users
of the connector, including the session reactivation sequence.

RDS broker support: optional GCC cluster V4 advertisement and redirected session
ID. Recognize enhanced-security redirection on the negotiated I/O channel during
licensing, capabilities and finalization. A typed, redacted error carries the
bounded decoded fields to the transport layer. Zeroizing opaque broker cookies
replace only Client Info password bytes, with their exact binary representation;
CredSSP retains the login password. Unsupported encrypted-password / smart-card
redirection modes fail explicitly. Tests live in tests/rdp_redirection.rs at the
HostGates root and exercise connector state transitions and wire bytes.
