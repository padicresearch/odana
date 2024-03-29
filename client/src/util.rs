use crate::parser::{parse_cmd_str, Command, CommandError};
use anyhow::bail;
use base64::engine::general_purpose;
use base64::Engine;
use json_dotpath::DotPaths;
use serde_json::{json, Value};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub fn handle_cmd_string(cmd: &Command) -> Result<Vec<u8>, CommandError> {
    match cmd.op {
        "hex" => {
            hex::decode(cmd.data).map_err(|e| CommandError::FailedToParseNom(format!("{}", e)))
        }

        "file" => {
            let file_path = PathBuf::new().join(cmd.data);
            if !file_path.is_file() {
                return Err(CommandError::FailedToParseNom("path not file".to_string()));
            }
            let mut file = File::open(file_path.as_path())
                .map_err(|e| CommandError::FailedToParseNom(format!("{}", e)))?;
            let mut out = Vec::with_capacity(
                file.metadata()
                    .map_err(|e| CommandError::FailedToParseNom(format!("{}", e)))?
                    .len() as usize,
            );
            let _read_len = file
                .read_to_end(&mut out)
                .map_err(|e| CommandError::FailedToParseNom(format!("{}", e)));
            Ok(out)
        }
        _ => Err(CommandError::FailedToParse),
    }
}

pub(crate) fn convert_command_strings(value: &mut Value) -> anyhow::Result<()> {
    match value {
        Value::Null => {
            return Ok(());
        }
        Value::Bool(_) => {
            return Ok(());
        }
        Value::Number(_) => {
            return Ok(());
        }
        Value::String(str) => {
            if let Some(bytes) = parse_cmd_str(str)
                .map(|cmd| handle_cmd_string(&cmd).ok())
                .ok()
                .flatten()
            {
                *value = json!({
                    "value" : Value::String(general_purpose::STANDARD.encode(bytes))
                })
            }
            return Ok(());
        }
        Value::Array(val) => {
            for i in val {
                convert_command_strings(i)?;
            }
        }
        Value::Object(val) => {
            for v in val.iter_mut().map(|(_, v)| v) {
                convert_command_strings(v)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn parse_cli_args_to_json(
    iter: impl IntoIterator<Item = impl Into<std::ffi::OsString>>,
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
    convert_command_strings(&mut json_value)?;
    Ok(json_value)
}

pub struct RpcMethod<'a> {
    service_name: &'a str,
    method_name: &'a str,
}

impl<'a> RpcMethod<'a> {
    pub fn parse(method_name: &'a str) -> anyhow::Result<Self> {
        let method_name = method_name.trim_start_matches('/');
        let split: Vec<&str> = method_name.splitn(2, '/').collect();
        if split.len() == 2 {
            Ok(RpcMethod {
                service_name: split[0],
                method_name: split[1],
            })
        } else {
            bail!("Invalid method name")
        }
    }

    pub fn service_name(&self) -> &'a str {
        self.service_name
    }

    pub fn method_name(&self) -> &'a str {
        self.method_name
    }

    pub fn full_name(&self) -> String {
        format!("/{}/{}", self.service_name, self.method_name)
    }

    pub fn service_id(&self) -> u64 {
        rune_framework::prelude::Hashing::twox_64_hash(self.service_name().as_bytes())
    }

    pub fn method_id(&self) -> u64 {
        rune_framework::prelude::Hashing::twox_64_hash(self.full_name().as_bytes())
    }
}

#[test]
fn test_rpc_method() {
    // Valid method name
    let method_name = "/myapp.myservice/myMethod";
    let rpc_method = RpcMethod::parse(method_name).unwrap();
    assert_eq!(rpc_method.service_name(), "myapp.myservice");
    assert_eq!(rpc_method.method_name(), "myMethod");
    assert_eq!(rpc_method.full_name(), method_name);

    // Invalid method name (missing service name)
    let invalid_method_name = "myMethod";
    let result = RpcMethod::parse(invalid_method_name);
    assert!(result.is_err());

    // Invalid method name (missing method name)
    let invalid_method_name = "/myapp.myservice/";
    let result = RpcMethod::parse(invalid_method_name);
    assert!(result.is_err());
}
