use toml::Value;

/// 获取输出目录
pub(crate) fn pick_field<'a>(
    toml_value: &'a Value,
    field: &'a str,
    sub_field: &'a str,
) -> Option<&'a str> {
    toml_value
        .get(field)
        .and_then(|v| v.get(sub_field))
        .and_then(Value::as_str)
}
