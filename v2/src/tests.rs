use super::*;
use serde_json::json;
#[test]
fn js_number_source_quirks() {
    for input in [
        "",
        "0",
        "-0",
        "  ",
        "0x0",
        "NaN",
        "abc",
        "inf",
        "+0x1",
        "1_0",
        "\u{0085}0.5",
    ] {
        assert!(validate_power(input).is_err(), "{input:?}");
    }
    for input in [
        "1",
        " .5 ",
        "+0.5",
        "1e-2",
        "0x1",
        "0b1",
        "0o1",
        "\u{feff}.25",
        "5e-324",
    ] {
        assert!(validate_power(input).is_ok(), "{input:?}");
    }
    for input in ["Infinity", "-Infinity", "1.01", "-0.1"] {
        assert_eq!(
            validate_power(input).unwrap_err(),
            "Power Cost must be between 0 and 1.00!"
        );
    }
}
#[test]
fn decimal_half_expand_and_negative_zero() {
    for (n, expect) in [
        (1.005, "$1.01"),
        (2.675, "$2.68"),
        (-0.0, "-$0.00"),
        (-0.004, "-$0.00"),
        (1234.567, "$1,234.57"),
        (1e-7, "$0.00"),
        (1e21, "$1,000,000,000,000,000,000,000.00"),
    ] {
        assert_eq!(core::usd(Some(&json!(n))), expect);
    }
}
#[test]
fn secret_free_and_schema_bounds() {
    assert!(normalize(
        "coins",
        json!([{ "coin":"SYNTHETIC-SECRET" }]),
        "SYNTHETIC-SECRET"
    )
    .is_err());
    assert!(normalize(
        "coins",
        json!({"title":"secret","detail":"secret"}),
        "secret"
    )
    .unwrap_err()
    .contains("reported an error"));
    assert!(normalize("coins", json!(null), "x").is_err());
    assert!(normalize("coins", json!(vec![json!({}); 513]), "x").is_err());
    assert!(normalize("gpu_estimates", json!({"test":{"device":{}}}), "x").is_err());
    assert!(normalize("coins", json!([{ "coin":"x".repeat(4097) }]), "key").is_err());
    assert!(validate_power(&"1".repeat(4097)).is_err());
    assert_ne!(cache_key("A", "coins", ""), cache_key("B", "coins", ""));
    assert_ne!(
        cache_key("A", "gpu_estimates", ".5"),
        cache_key("A", "gpu_estimates", "0.5")
    );
}
#[test]
fn javascript_object_enumeration() {
    let row = json!({"device":{},"profit":{},"profit24":{},"revenue":{},"revenue24":{}});
    let mut v = json!({});
    for k in ["z", "10", "2", "01", "4294967295", "0"] {
        v[k] = row.clone();
    }
    let data = normalize("gpu_estimates", v, "SYNTHETIC").unwrap();
    assert_eq!(
        data.as_array()
            .unwrap()
            .iter()
            .map(|x| x["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["0", "2", "10", "z", "01", "4294967295"]
    );
}
#[test]
fn actual_source_oracle() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/source-oracle.json");
    let oracle: Value = serde_json::from_slice(
        &std::fs::read(path).expect("run scripts/oracle.sh before cargo test"),
    )
    .unwrap();
    let source_date = local_date(oracle["epoch"].as_i64(), "");
    for case in oracle["views"].as_array().unwrap() {
        assert!(case["section"].as_str().unwrap().ends_with(&source_date));
    }
    for case in oracle["numeric"].as_array().unwrap() {
        assert_eq!(
            validate_power(case["input"].as_str().unwrap()).err(),
            case["error"].as_str().map(str::to_owned),
            "{}",
            case["input"]
        );
    }
    for case in oracle["mime"].as_array().unwrap() {
        assert_eq!(
            core::is_json_content_type(case["input"].as_str().unwrap()),
            case["expected"].as_bool().unwrap()
        );
    }
    for case in oracle["currency"].as_array().unwrap() {
        assert_eq!(
            core::usd(case.get("input")),
            case["expected"].as_str().unwrap(),
            "{}",
            case["input"]
        );
    }
    for case in oracle["views"].as_array().unwrap() {
        let command = case["command"].as_str().unwrap();
        let detail = case["detail"].as_bool().unwrap();
        let normalized = normalize(command, case["input"].clone(), "ORACLE-SYNTHETIC-KEY").unwrap();
        let rows = rows(
            command,
            &normalized,
            detail,
            &["test:toggle".into(), "test:refresh".into()],
        )
        .unwrap();
        let projection:Vec<Value>=rows.iter().map(|r|json!({"title":r["props"]["title"],"subtitle":r["props"]["subtitle"],"accessories":r["props"]["accessories"],"metadata":r["props"]["detail"]["children"][0]["children"].as_array().unwrap().iter().map(|n|if n["type"].as_str().unwrap().ends_with("Separator"){json!({"separator":true})}else{let mut p=n["props"].clone(); if p.get("text").is_none(){p["text"]=Value::Null;}p}).collect::<Vec<_>>()})).collect();
        assert_eq!(
            json!(projection),
            case["rows"],
            "{command} details={detail}"
        );
    }
}
#[tokio::test]
async fn stale_ids_cross_session_and_dispose() {
    let root = tempfile::tempdir_in(std::env::var("TMPDIR").unwrap()).unwrap();
    let ctx = tezbar_extension_sdk::TestContext::new(
        tezbar_extension_sdk::ExtensionPaths {
            package_dir: root.path().into(),
            data_dir: root.path().into(),
            temp_dir: root.path().into(),
        },
        &[],
    );
    let adapter = Adapter::production().unwrap();
    let key = cache_key("SYNTHETIC", "coins", "");
    let mut cache = json!({"version":1,"entries":{},"settings":{}});
    cache["entries"][key] = json!({"data":[{}],"date":"1/2/2026","serial":0});
    write_cache(root.path(), &cache).unwrap();
    let mut state = json!({});
    let p = json!({"sessionId":"one","preferences":{"api_key":"SYNTHETIC"},"argumentValues":{}});
    let view = adapter
        .handle(
            &ctx.context,
            "coins",
            "extension:run-command",
            &p,
            &mut state,
        )
        .await
        .unwrap();
    let id = view["actions"][0]["id"].clone();
    let mut action = p.clone();
    action["actionId"] = id;
    let next = adapter
        .handle(
            &ctx.context,
            "coins",
            "extension:invoke-action",
            &action,
            &mut state,
        )
        .await
        .unwrap();
    assert!(next["root"]["props"]["isShowingDetail"].as_bool().unwrap());
    assert_eq!(
        adapter
            .handle(
                &ctx.context,
                "coins",
                "extension:invoke-action",
                &action,
                &mut state
            )
            .await
            .unwrap()["mode"],
        "view"
    );
    action["sessionId"] = json!("another");
    action["actionId"] = next["actions"][0]["id"].clone();
    assert_eq!(
        adapter
            .handle(
                &ctx.context,
                "coins",
                "extension:invoke-action",
                &action,
                &mut state
            )
            .await
            .unwrap()["mode"],
        "view"
    );
    assert!(ctx.calls().is_empty());
    assert_eq!(
        adapter
            .handle(
                &ctx.context,
                "coins",
                "extension:refresh-session",
                &p,
                &mut state
            )
            .await
            .unwrap()["mode"],
        "unchanged"
    );
    adapter
        .handle(
            &ctx.context,
            "coins",
            "extension:dispose-session",
            &p,
            &mut state,
        )
        .await
        .unwrap();
    let other = json!({"sessionId":"another"});
    adapter
        .handle(
            &ctx.context,
            "coins",
            "extension:dispose-session",
            &other,
            &mut state,
        )
        .await
        .unwrap();
    assert!(adapter.owners.lock().await.is_empty());
}
#[test]
fn malformed_and_oversized_cache() {
    let dir = tempfile::tempdir_in(std::env::var("TMPDIR").unwrap()).unwrap();
    std::fs::write(dir.path().join("hashrate-cache.json"), b"{").unwrap();
    assert!(read_cache(dir.path()).is_err());
    std::fs::write(
        dir.path().join("hashrate-cache.json"),
        vec![b' '; MAX_CACHE + 1],
    )
    .unwrap();
    assert!(read_cache(dir.path()).is_err());
}
