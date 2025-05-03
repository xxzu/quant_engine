use quant_engine::sys_config;
use std::fs::File;
use std::io::Write;

#[test]
fn test_find_val() {
    // assert_eq!(config.database_url, "mysql://root:123456@localhost/db");
}

#[test]
fn test_load_config() {
    // 创建临时配置文件
    let test_config_path = "test_config.yaml";
    let content = r#"
database_url: "test_db_url"
port: 8080
debug: true
"#;

    let mut file = File::create(test_config_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();

    // 加载配置
    let config = sys_config::load_config(test_config_path).unwrap();

    // 验证配置内容
    assert_eq!(config.database_url, "test_db_url");
    assert_eq!(config.port, 8080);
    assert_eq!(config.debug, true);

    // 删除临时文件
    std::fs::remove_file(test_config_path).unwrap();
}
