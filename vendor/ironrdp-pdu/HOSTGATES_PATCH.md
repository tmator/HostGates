# HostGates routing patch

Source: ironrdp-pdu 0.9.0, Devolutions/IronRDP revision
11a0810cfbbabd8b8023875a05e3041216d4b01b. MIT OR Apache-2.0.

Adds an outgoing NegoRequestData::LoadBalanceInfo variant for the verbatim
loadbalanceinfo string from an RDP file (including tsv:// collection routes).
Unlike RoutingToken, it does not prepend Cookie: msts=. Adds exactly one CRLF.
Encoding rejects control characters, non-ASCII input and values over 200 bytes
to stay within the X.224 header length limit. Existing cookie encoding and
server-side decoding are unchanged. This is not server-redirection support.
HostGates tests verify import, persistence and outgoing bytes.
