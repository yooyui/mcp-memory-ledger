use agent_llm_mm::support::config::AppConfig;

#[test]
fn default_dashboard_config_is_disabled_and_safe() {
    let config = AppConfig::default();

    assert!(!config.dashboard.enabled);
    assert_eq!(config.dashboard.host, "127.0.0.1");
    assert_eq!(config.dashboard.port, 8787);
    assert_eq!(config.dashboard.base_path, "/");
    assert_eq!(config.dashboard.event_capacity, 2000);
    assert!(config.dashboard.sse_enabled);
    assert!(!config.dashboard.open_browser);
    assert!(!config.dashboard.required);
    assert!(config.dashboard.validate().is_ok());
}

#[test]
fn load_from_path_reads_dashboard_section() {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let config_path = temp_dir.path().join("agent-llm-mm.local.toml");
    std::fs::write(
        &config_path,
        r#"
transport = "stdio"
database_url = "sqlite:///tmp/agent-llm-mm-dashboard-config-test.sqlite"

[dashboard]
enabled = true
host = "0.0.0.0"
port = 9797
base_path = "/agent-llm-mm"
event_capacity = 123
sse_enabled = false
open_browser = true
required = true
"#,
    )
    .expect("write config");

    let config = AppConfig::load_from_path(&config_path).expect("load config");

    assert!(config.dashboard.enabled);
    assert_eq!(config.dashboard.host, "0.0.0.0");
    assert_eq!(config.dashboard.port, 9797);
    assert_eq!(config.dashboard.base_path, "/agent-llm-mm");
    assert_eq!(config.dashboard.event_capacity, 123);
    assert!(!config.dashboard.sse_enabled);
    assert!(config.dashboard.open_browser);
    assert!(config.dashboard.required);
    assert_eq!(
        config.dashboard.validate().unwrap_err(),
        "dashboard.host must be localhost or a loopback IP while dashboard authentication is unavailable"
    );
}

#[test]
fn dashboard_rejects_zero_event_capacity() {
    let mut config = AppConfig::default();
    config.dashboard.event_capacity = 0;

    assert_eq!(
        config.dashboard.validate().unwrap_err(),
        "dashboard.event_capacity must be greater than 0"
    );
}

#[test]
fn dashboard_rejects_base_path_without_leading_slash() {
    let mut config = AppConfig::default();
    config.dashboard.base_path = "agent-llm-mm".to_string();

    assert_eq!(
        config.dashboard.validate().unwrap_err(),
        "dashboard.base_path must start with /"
    );
}

#[test]
fn enabled_dashboard_rejects_non_loopback_hosts_without_authentication() {
    for host in ["0.0.0.0", "192.168.1.20", "example.test"] {
        let mut config = AppConfig::default();
        config.dashboard.enabled = true;
        config.dashboard.host = host.to_string();

        assert_eq!(
            config.dashboard.validate().unwrap_err(),
            "dashboard.host must be localhost or a loopback IP while dashboard authentication is unavailable"
        );
    }
}

#[test]
fn enabled_dashboard_accepts_localhost_and_loopback_ip_ranges() {
    for host in ["localhost", "LOCALHOST", "127.0.0.1", "127.12.34.56", "::1"] {
        let mut config = AppConfig::default();
        config.dashboard.enabled = true;
        config.dashboard.host = host.to_string();

        assert!(config.dashboard.validate().is_ok(), "host={host}");
    }
}

#[test]
fn disabled_dashboard_may_retain_non_loopback_config_without_exposure() {
    let mut config = AppConfig::default();
    config.dashboard.host = "0.0.0.0".to_string();

    assert!(config.dashboard.validate().is_ok());
}
