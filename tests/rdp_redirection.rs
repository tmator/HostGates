use ironrdp_client::config::{Config, ConfigBuilder, Destination, Transport, TransportKind};
use ironrdp_connector::{ClientConnector, ClientConnectorState, ConnectorErrorKind, Sequence};
use ironrdp_core::{WriteBuf, encode_vec};
use ironrdp_pdu::{mcs::SendDataIndication, x224::X224};

fn config() -> Config {
    let mut config = ConfigBuilder::new()
        .with_destination(Destination::new("broker.example:3391").expect("valid test fixture"))
        .with_username("alice")
        .with_domain("LAB")
        .with_password("desktop-secret")
        .with_autologon(true)
        .with_client_build(26000)
        .with_client_dir("HostGates")
        .with_client_name("HostGates")
        .with_platform(ironrdp_pdu::rdp::capability_sets::MajorPlatformType::WINDOWS)
        .with_transport(TransportKind::Gateway {
            endpoint: "gateway.example:444".into(),
        })
        .with_gateway_username("GW\\bob")
        .with_gateway_password("gateway-secret")
        .build()
        .expect("valid test fixture");
    config.enable_server_redirection(true);
    config.use_redirection_server_name = true;
    config.gateway_rpc = true;
    config
        .set_load_balance_info("tsv://MS Terminal Services Plugin.1.collection")
        .expect("valid test fixture");
    config
}
fn field(data: &mut Vec<u8>, value: &[u8]) {
    data.extend_from_slice(&(value.len() as u32).to_le_bytes());
    data.extend_from_slice(value);
}
fn text(data: &mut Vec<u8>, value: &str) {
    field(
        data,
        &value
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
}
fn body() -> Vec<u8> {
    let mut data = vec![0, 0, 10, 0, 0xea, 3, 0, 0, 0, 4, 0, 0];
    data.extend_from_slice(&42u32.to_le_bytes());
    data.extend_from_slice(&0x11fu32.to_le_bytes()); // address, routing, user, domain, cookie, FQDN
    text(&mut data, "192.0.2.10");
    field(&mut data, b"Cookie: msts=123.456.0000\r\n");
    text(&mut data, "alice");
    text(&mut data, "LAB");
    field(&mut data, &[0xff, 0x80, 0x11, 0x22, 0, 0]); // opaque, not valid UTF-8
    text(&mut data, "desktop.example");
    let n = data.len();
    data[..2].copy_from_slice(&(n as u16).to_le_bytes());
    data[10..12].copy_from_slice(&((n - 8) as u16).to_le_bytes());
    data
}
fn packet(data: &[u8], channel: u16) -> Vec<u8> {
    encode_vec(&X224(SendDataIndication {
        initiator_id: 1002,
        channel_id: channel,
        user_data: data.into(),
    }))
    .expect("valid test fixture")
}
fn redirect(config: &Config) -> ironrdp_connector::redirection::ServerRedirection {
    let mut connector = ClientConnector::new(
        config.connector().clone(),
        "127.0.0.1:1234".parse().expect("valid test fixture"),
    );
    connector.state = ClientConnectorState::ConnectTimeAutoDetection {
        io_channel_id: 1003,
        user_channel_id: 1002,
    };
    let error = connector
        .step(&packet(&body(), 1003), &mut WriteBuf::new())
        .expect_err("broker redirection must interrupt the connector handshake");
    assert!(!format!("{error:?}").contains("alice"));
    let ConnectorErrorKind::ServerRedirection(redir) = error.kind() else {
        panic!("{error}");
    };
    (**redir).clone()
}

#[test]
fn broker_redirect_keeps_gateway_and_nla_and_updates_target_session_and_certificate_name() {
    let mut cfg = config();
    let redir = redirect(&cfg);
    cfg.apply_server_redirection(&redir)
        .expect("valid test fixture");
    assert_eq!(cfg.destination().name(), "desktop.example");
    assert_eq!(cfg.destination().port(), 3391);
    assert_eq!(cfg.tls_server_name.as_deref(), Some("desktop.example"));
    assert_eq!(cfg.connector().redirected_session_id, Some(42));
    let ironrdp_connector::Credentials::UsernamePassword { password, .. } =
        &cfg.connector().credentials
    else {
        panic!("password credentials expected")
    };
    assert_eq!(password, "desktop-secret");
    assert!(cfg.gateway_rpc);
    let Transport::Gateway(gateway) = cfg.transport() else {
        panic!("gateway lost")
    };
    assert_eq!(gateway.endpoint, "gateway.example:444");
    assert_eq!(gateway.username, "GW\\bob");
    assert_eq!(gateway.password, "gateway-secret");
    let request = cfg
        .connector()
        .request_data
        .as_ref()
        .expect("valid test fixture");
    let mut encoded = vec![0; request.size()];
    request
        .write(&mut ironrdp_core::WriteCursor::new(&mut encoded))
        .expect("valid test fixture");
    assert_eq!(encoded, b"Cookie: msts=123.456.0000\r\n");

    let mut connector = ClientConnector::new(
        cfg.connector().clone(),
        "127.0.0.1:1234".parse().expect("valid test fixture"),
    );
    connector.state = ClientConnectorState::SecureSettingsExchange {
        io_channel_id: 1003,
        user_channel_id: 1002,
    };
    let mut out = WriteBuf::new();
    connector
        .step_no_input(&mut out)
        .expect("valid test fixture");
    let pdu: X224<ironrdp_pdu::mcs::SendDataRequest<'_>> =
        ironrdp_core::decode(out.filled()).expect("valid test fixture");
    let data = &pdu.0.user_data;
    let domain_len = u16::from_le_bytes([data[12], data[13]]) as usize;
    let user_len = u16::from_le_bytes([data[14], data[15]]) as usize;
    let start = 26 + domain_len + user_len;
    assert_eq!(&data[16..18], &4u16.to_le_bytes());
    assert_eq!(&data[start..start + 6], &[0xff, 0x80, 0x11, 0x22, 0, 0]);
    let secret: Vec<_> = "desktop-secret"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    assert!(!data.windows(secret.len()).any(|w| w == secret));
}

#[test]
fn malformed_and_wrong_channel_redirections_cannot_change_destination() {
    let cfg = config();
    let original = body();
    for end in 0..original.len() {
        let mut data = original[..end].to_vec();
        if end >= 2 {
            data[..2].copy_from_slice(&(end as u16).to_le_bytes());
        }
        assert!(!matches!(
            ironrdp_connector::redirection::decode(&packet(&data, 1003), 1003),
            Ok(Some(_))
        ));
    }
    assert!(
        ironrdp_connector::redirection::decode(&packet(&original, 1004), 1003)
            .expect("valid test fixture")
            .is_none()
    );
    let redir = redirect(&cfg);
    for bad in [
        "attacker.example:443",
        "https://host",
        "host/path",
        "user@host",
        "-host",
    ] {
        let mut cfg = config();
        let mut r = redir.clone();
        r.fqdn = Some(bad.into());
        assert!(cfg.apply_server_redirection(&r).is_err());
        assert_eq!(cfg.destination().name(), "broker.example");
    }
    let mut cfg = config();
    let mut r = redir;
    r.tsv_url = Some(b"different collection".to_vec());
    assert!(cfg.apply_server_redirection(&r).is_err());
    cfg.enable_server_redirection(false);
    r.tsv_url = None;
    assert!(cfg.apply_server_redirection(&r).is_err());
}

#[test]
fn cluster_capability_and_redirected_session_id_are_sent() {
    let mut cfg = config();
    let redir = redirect(&cfg);
    cfg.apply_server_redirection(&redir)
        .expect("valid test fixture");
    let mut connector = ClientConnector::new(
        cfg.connector().clone(),
        "127.0.0.1:1234".parse().expect("valid test fixture"),
    );
    connector.state = ClientConnectorState::BasicSettingsExchangeSendInitial {
        selected_protocol: ironrdp_pdu::nego::SecurityProtocol::SSL,
    };
    let mut out = WriteBuf::new();
    connector
        .step_no_input(&mut out)
        .expect("valid test fixture");
    // GCC CS_CLUSTER header, V4 + supported + valid session, redirected session 42.
    let cluster = [4, 0xc0, 12, 0, 15, 0, 0, 0, 42, 0, 0, 0];
    assert!(out.filled().windows(cluster.len()).any(|w| w == cluster));
}
