use hostgates::{
    connections::rdp::Gateway,
    database::Database,
    importers::{DuplicatePolicy, rdp},
};

const FILE: &str = "full address:s:[2001:db8::1]:3390\r\nusername:s:alice\r\ndomain:s:LAB\r\ngatewayhostname:s:gateway.example:444\r\ngatewayusagemethod:i:1\r\ngatewayusername:s:LAB\\bob\r\ngatewaycredentialssource:i:4\r\npassword 51:b:secret-marker\r\nredirectclipboard:i:1\r\n";

#[test]
fn rdp_unicode_gateway_and_sanitized_launch() {
    for big in [false, true] {
        let bytes = std::iter::once(0xfeff)
            .chain(FILE.encode_utf16())
            .flat_map(|v| {
                if big {
                    v.to_be_bytes()
                } else {
                    v.to_le_bytes()
                }
            })
            .collect::<Vec<_>>();
        let preview = rdp::parse(&bytes, "Bureau").expect("import");
        let data = preview.rows[0].data.as_ref().expect("session");
        assert_eq!(data.hostname, "2001:db8::1");
        assert_eq!(data.port, 3390);
        assert_eq!(data.domain, "LAB");
        let gateway = preview.rows[0].rdp_gateway.as_ref().expect("gateway");
        let output = gateway.file_text(data).expect("safe file");
        assert!(output.contains("full address:s:[2001:db8::1]:3390"));
        assert!(output.contains("gatewayhostname:s:gateway.example:444"));
        assert!(!output.contains("secret-marker"));
        assert!(!output.contains("redirectclipboard:i:1"));
        assert!(output.contains("authentication level:i:1"));
        let args = gateway.arguments(data).expect("FreeRDP");
        assert!(args.contains(&"/g:gateway.example:444".into()));
        assert!(
            !args.iter().any(|s| s.starts_with("/p:")
                || s.starts_with("/gp:")
                || s.contains("cert:ignore"))
        );
    }
}

#[test]
fn invalid_or_unsupported_rdp_is_rejected() {
    for file in [
        "full address:s:/bad",
        "full address:s:host:0",
        "full address:i:123",
        "full address:s:host\nFULL ADDRESS:s:other",
        "full address:s:host\nremoteapplicationmode:i:1",
        "full address:s:host\nalternate shell:s:command",
        "full address:s:host\ngatewayusagemethod:i:1",
        "full address:s:host\ngatewayusagemethod:i:3",
        "full address:s:host\ngatewayhostname:s:gw/invalid",
    ] {
        assert!(rdp::parse(file.as_bytes(), "test").is_err(), "{file}");
    }
    assert!(rdp::parse(&[0xff, 0xfe, 0], "test").is_err());
    assert!(
        Gateway {
            host: "gw".into(),
            username: "a\npassword 51:b:bad".into(),
            ..Default::default()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn gateway_import_report_tracks_actual_ids_and_duplicate_decisions() {
    let mut db = Database::in_memory().expect("db");
    let mut preview = rdp::parse(FILE.as_bytes(), "Bureau").expect("preview");
    let report = db.import(&preview).expect("import");
    assert_eq!(report.rdp_gateways.len(), 1);
    let id = report.rdp_gateways[0].0.clone();
    assert_eq!(db.snapshot().expect("snapshot").sessions[0].uuid, id);
    assert!(db.import(&preview).expect("skip").rdp_gateways.is_empty());
    preview.rows[0].policy = DuplicatePolicy::Replace;
    preview.rows[0].rdp_gateway = None;
    let report = db.import(&preview).expect("replace");
    assert_eq!(report.rdp_gateways[0].0, id);
    assert!(report.rdp_gateways[0].1.is_none());
    preview.rows[0].rdp_configuration = false;
    assert!(
        db.import(&preview)
            .expect("other import keeps options")
            .rdp_gateways
            .is_empty()
    );
}

#[test]
fn extension_dispatch_and_disabled_gateway() {
    let dir = tempfile::tempdir().expect("dir");
    let file = dir.path().join("Bureau.RDP");
    std::fs::write(
        &file,
        "full address:s:host\nserver port:i:3391\ngatewayhostname:s:gw\ngatewayusagemethod:i:0",
    )
    .expect("write");
    let db = Database::in_memory().expect("db");
    let preview =
        hostgates::importers::exchange::read_preview(&file, &db.snapshot().expect("snapshot"))
            .expect("read");
    assert_eq!(preview.rows[0].name, "Bureau");
    assert_eq!(preview.rows[0].data.as_ref().expect("data").port, 3391);
    assert!(preview.rows[0].rdp_gateway.is_none());
}

#[test]
fn rds_routing_is_imported_replaced_and_validated() {
    let file = "full address:s:broker.example\nloadbalanceinfo:s:tsv://MS Terminal Services Plugin.1.collection\nuse redirection server name:i:1";
    let mut preview = rdp::parse(file.as_bytes(), "Collection").expect("parse");
    let routing = &preview.rows[0].rdp_routing;
    assert!(routing.use_redirection_server_name);
    assert_eq!(
        routing.load_balance_info,
        "tsv://MS Terminal Services Plugin.1.collection"
    );
    let mut db = Database::in_memory().expect("db");
    let report = db.import(&preview).expect("import");
    assert_eq!(report.rdp_routes[0].1, *routing);
    assert_eq!(report.rdp_routes[0].0, report.rdp_gateways[0].0);
    assert!(db.import(&preview).expect("skip").rdp_routes.is_empty());
    preview.rows[0].policy = DuplicatePolicy::Replace;
    preview.rows[0].rdp_routing = Default::default();
    assert!(
        db.import(&preview).expect("replace").rdp_routes[0]
            .1
            .load_balance_info
            .is_empty()
    );
    for value in ["x\rInjected", "é", &"x".repeat(201)] {
        let file = format!("full address:s:broker.example\nloadbalanceinfo:s:{value}");
        assert!(rdp::parse(file.as_bytes(), "invalid").is_err());
    }
    assert!(
        rdp::parse(
            b"full address:s:broker.example\nuse redirection server name:i:2",
            "invalid"
        )
        .is_err()
    );
}

#[test]
fn external_clients_keep_the_imported_collection_route() {
    let preview = rdp::parse(b"full address:s:broker.example\ngatewayhostname:s:gateway.example\nloadbalanceinfo:s:tsv://MS Terminal Services Plugin.1.collection\nuse redirection server name:i:1", "fixture").expect("valid test fixture");
    let row = &preview.rows[0];
    let gateway = row.rdp_gateway.as_ref().expect("valid test fixture");
    let data = row.data.as_ref().expect("valid test fixture");
    let file = gateway
        .file_text_with_routing(data, &row.rdp_routing)
        .expect("valid test fixture");
    assert!(file.contains("loadbalanceinfo:s:tsv://MS Terminal Services Plugin.1.collection\r\n"));
    assert!(file.contains("use redirection server name:i:1\r\n"));
    let args = gateway
        .arguments_with_routing(data, &row.rdp_routing)
        .expect("valid test fixture");
    assert!(
        args.contains(
            &"/load-balance-info:tsv://MS Terminal Services Plugin.1.collection".to_owned()
        )
    );
}
