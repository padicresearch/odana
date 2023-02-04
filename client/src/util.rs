use anyhow::bail;
use json_dotpath::DotPaths;
use serde_json::Value;

pub(crate) fn parse_cli_args_to_json(
    iter: impl IntoIterator<Item=impl Into<std::ffi::OsString>>,
) -> anyhow::Result<Value> {
    let mut parsed_args = Vec::new();
    let args = clap_lex::RawArgs::new(iter);
    let mut cursor = args.cursor();
    while let Some(arg) = args.next(&mut cursor) {
        let (key, value) = if let Some((Ok(key), value)) = arg.to_long() {
            if let Some(value) = value {
                (
                    key.to_string(),
                    value.to_str().unwrap_or_default().to_string(),
                )
            } else {
                let Some(value) = args.next(&mut cursor) else {
                    bail!(r#"[{key}] doesn't have a value assigned, eg. --{key}="something" or --{key} something"#)
                };
                let Ok(value) = value.to_value() else {
                    bail!(r#"[{key}] doesn't have a value assigned, eg. --{key}="something" or --{key} something"#)
                };
                (key.to_string(), value.to_string())
            }
        } else {
            bail!(r#"args must be in the long format eg. --some="value" or --some value"#)
        };
        parsed_args.push((key, value));
    }

    let mut json_value = Value::Null;

    for (path, value) in parsed_args {
        let jvalue: Value = match serde_json::from_str(&value) {
            Ok(v) => v,
            Err(_) => Value::from(value),
        };
        json_value.dot_set(&path, &jvalue)?;
    }

    Ok(json_value)
}
