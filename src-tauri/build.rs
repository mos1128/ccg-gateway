use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const PROTOCOLS: &[&str] = &[
    "anthropic_messages",
    "openai_chat",
    "openai_responses",
    "gemini_generate_content",
];
fn required_string<'a>(value: &'a Value, key: &str, file: &Path) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("{}: `{}` must be a non-empty string", file.display(), key))
}

fn non_empty_array(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

fn validate_config_operations(operations: &[Value], label: &str, file: &Path) {
    let mut ids = HashSet::new();
    let mut file_formats = HashMap::new();
    for operation in operations {
        let id = required_string(operation, "id", file);
        if !ids.insert(id) {
            panic!(
                "{}: duplicate {} operation id `{}`",
                file.display(),
                label,
                id
            );
        }
        let target = required_string(operation, "file", file);
        let format = required_string(operation, "format", file);
        if let Some(existing) = file_formats.insert(target, format) {
            if existing != format {
                panic!(
                    "{}: {} file `{}` uses conflicting formats",
                    file.display(),
                    label,
                    target
                );
            }
        }
        let op = required_string(operation, "op", file);
        let path = operation
            .get("path")
            .and_then(Value::as_array)
            .filter(|path| !path.is_empty())
            .unwrap_or_else(|| {
                panic!(
                    "{}: {} operation `{}` needs a non-empty path",
                    file.display(),
                    label,
                    id
                )
            });
        let is_env = format == "env";
        if is_env && path.len() != 1 {
            panic!(
                "{}: {} operation `{}` has incompatible op, format or path",
                file.display(),
                label,
                id
            );
        }

        let writes_value = op == "set";
        let has_value = operation.get("value").is_some();
        if (writes_value && !has_value) || (!writes_value && has_value) {
            panic!(
                "{}: {} operation `{}` has invalid value",
                file.display(),
                label,
                id
            );
        }

        let path_text = path.iter().map(Value::to_string).collect::<String>();
        if target.contains("{target.")
            || target.contains("{agent.")
            || path_text.contains("{target.")
            || path_text.contains("{agent.")
        {
            panic!(
                "{}: {} operation `{}` can only use target/agent variables in value",
                file.display(),
                label,
                id
            );
        }
        let value_text = operation
            .get("value")
            .map(Value::to_string)
            .unwrap_or_default();
        if label == "provider_config"
            && (target.contains("{profile")
                || path_text.contains("{profile")
                || value_text.contains("{profile"))
        {
            panic!(
                "{}: {} operation `{}` cannot use Profile variables",
                file.display(),
                label,
                id
            );
        }
        if target.contains("{profile.absolute_path}")
            || path_text.contains("{profile.absolute_path}")
            || value_text.contains("{profile.relative_path}")
            || value_text.contains("{profile.absolute_path}")
        {
            panic!(
                "{}: {} operation `{}` uses a Profile path variable in an invalid position",
                file.display(),
                label,
                id
            );
        }
    }
}

fn validate_provider_config(feature: &Value, file: &Path) {
    let operations = feature
        .get("operations")
        .and_then(Value::as_array)
        .filter(|operations| !operations.is_empty())
        .unwrap_or_else(|| {
            panic!(
                "{}: enabled provider_config needs operations",
                file.display()
            )
        });
    validate_config_operations(operations, "provider_config", file);
}

fn validate_definition(value: &Value, file: &Path, ids: &mut HashSet<String>) -> String {
    if value.get("schema_version").and_then(Value::as_u64) != Some(1) {
        panic!("{}: only schema_version 1 is supported", file.display());
    }

    let id = required_string(value, "id", file).to_string();
    if !id.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
    }) {
        panic!("{}: invalid Agent id `{}`", file.display(), id);
    }
    if file.file_stem().and_then(|stem| stem.to_str()) != Some(id.as_str()) {
        panic!("{}: filename must match Agent id `{}`", file.display(), id);
    }
    if !ids.insert(id.clone()) {
        panic!("{}: duplicate Agent id `{}`", file.display(), id);
    }
    match value.get("sort_order").and_then(Value::as_i64) {
        Some(order) if order >= 0 => {}
        _ => panic!(
            "{}: `sort_order` must be a non-negative integer",
            file.display()
        ),
    }
    required_string(value, "name", file);
    required_string(value, "config_dir", file);

    let user_agents = value
        .get("user_agent")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| panic!("{}: user_agent must be a non-empty array", file.display()));
    for pattern in user_agents {
        if pattern
            .as_str()
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
            .is_none()
        {
            panic!("{}: user_agent cannot contain empty values", file.display());
        }
    }

    let protocols = value
        .get("protocols")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| panic!("{}: protocols cannot be empty", file.display()));
    let mut seen_protocols = HashSet::new();
    for protocol in protocols {
        let protocol = protocol
            .as_str()
            .unwrap_or_else(|| panic!("{}: protocol must be a string", file.display()));
        if !PROTOCOLS.contains(&protocol) {
            panic!("{}: unknown protocol `{}`", file.display(), protocol);
        }
        if !seen_protocols.insert(protocol) {
            panic!("{}: duplicate protocol `{}`", file.display(), protocol);
        }
    }

    let features = value
        .get("features")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("{}: features must be an object", file.display()));

    for (name, feature) in features {
        if !feature
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        match name.as_str() {
            "provider_config" => {
                validate_provider_config(feature, file);
            }
            "global_preset" => {
                required_string(feature, "file", file);
                if !matches!(
                    feature.get("format").and_then(Value::as_str),
                    Some("json" | "toml")
                ) {
                    panic!(
                        "{}: enabled global_preset needs a JSON or TOML target",
                        file.display()
                    );
                }
            }
            "profiles" => {
                let profile_file = required_string(feature, "profile_file", file);
                if !profile_file.contains("{profile}") {
                    panic!(
                        "{}: profiles.profile_file must contain {{profile}}",
                        file.display()
                    );
                }
                let operations = feature
                    .get("operations")
                    .and_then(Value::as_array)
                    .filter(|operations| !operations.is_empty())
                    .unwrap_or_else(|| {
                        panic!("{}: enabled profiles needs operations", file.display())
                    });
                validate_config_operations(operations, "profiles", file);
                if let Some(launch) = feature.get("launch") {
                    for key in ["default", "non_default"] {
                        let args = launch
                            .get(key)
                            .and_then(Value::as_array)
                            .unwrap_or_else(|| {
                                panic!(
                                    "{}: profiles.launch.{} must be an array",
                                    file.display(),
                                    key
                                )
                            });
                        if args.iter().any(|arg| {
                            arg.as_str()
                                .map(|arg| arg.trim().is_empty())
                                .unwrap_or(true)
                        }) {
                            panic!(
                                "{}: profiles.launch.{} cannot contain empty arguments",
                                file.display(),
                                key
                            );
                        }
                        if args.iter().any(|arg| {
                            arg.as_str().is_some_and(|arg| {
                                arg.contains("{target.") || arg.contains("{agent.")
                            })
                        }) {
                            panic!(
                                "{}: profiles.launch.{} cannot use target/agent variables",
                                file.display(),
                                key
                            );
                        }
                        if args.iter().any(|arg| {
                            arg.as_str()
                                .is_some_and(|arg| arg.contains("{profile.relative_path}"))
                        }) {
                            panic!(
                                "{}: profiles.launch.{} cannot use {{profile.relative_path}}",
                                file.display(),
                                key
                            );
                        }
                        if key == "default"
                            && args.iter().any(|arg| {
                                arg.as_str()
                                    .is_some_and(|arg| arg.contains("{profile.absolute_path}"))
                            })
                        {
                            panic!(
                                "{}: profiles.launch.default cannot use {{profile.absolute_path}}",
                                file.display()
                            );
                        }
                    }
                }
            }
            "official_login" => {
                if !non_empty_array(feature, "operations") {
                    panic!(
                        "{}: enabled official_login needs operations",
                        file.display()
                    );
                }
            }
            "skills" => {
                if feature
                    .get("directory")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|directory| !directory.is_empty())
                    .is_none()
                {
                    panic!("{}: enabled skills needs directory", file.display());
                }
            }
            "mcp" => {
                if feature.get("file").and_then(Value::as_str).is_none()
                    || feature.get("format").and_then(Value::as_str).is_none()
                    || !non_empty_array(feature, "servers_path")
                {
                    panic!(
                        "{}: enabled mcp needs file, format and servers_path",
                        file.display()
                    );
                }
            }
            "sessions" | "plugins" => {
                if feature.get("adapter").and_then(Value::as_str).is_none() {
                    panic!("{}: enabled {} needs an adapter", file.display(), name);
                }
            }
            "prompts" => {
                if feature.get("file").and_then(Value::as_str).is_none() {
                    panic!("{}: enabled prompts needs a file", file.display());
                }
            }
            _ => {}
        }
    }

    id
}

fn generate_agent_definitions() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let definitions_dir = manifest_dir.join("agent-definitions");
    let schema_path = definitions_dir.join("agent-definition.schema.json");
    println!("cargo:rerun-if-changed={}", definitions_dir.display());

    if !schema_path.is_file() {
        panic!("missing Agent definition schema: {}", schema_path.display());
    }
    let schema = serde_json::from_str::<Value>(&fs::read_to_string(&schema_path).unwrap())
        .unwrap_or_else(|error| panic!("invalid Agent JSON schema: {error}"));
    let schema_validator = jsonschema::validator_for(&schema)
        .unwrap_or_else(|error| panic!("invalid Agent JSON schema: {error}"));

    let mut files: Vec<PathBuf> = fs::read_dir(&definitions_dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", definitions_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && path.file_name().and_then(|name| name.to_str())
                    != Some("agent-definition.schema.json")
        })
        .collect();
    files.sort();
    if files.is_empty() {
        panic!("no built-in Agent definitions found");
    }

    let mut ids = HashSet::new();
    let mut entries = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()));
        let value: Value = serde_json::from_str(&source)
            .unwrap_or_else(|error| panic!("invalid JSON in {}: {error}", file.display()));
        if let Err(error) = schema_validator.validate(&value) {
            panic!("{}: JSON Schema validation failed: {error}", file.display());
        }
        let id = validate_definition(&value, &file, &mut ids);
        entries.push(format!("    ({:?}, {:?}),", id, source));
    }

    let generated = format!(
        "pub const BUILTIN_AGENT_DEFINITION_JSON: &[(&str, &str)] = &[\n{}\n];\n",
        entries.join("\n")
    );
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("agent_definitions.rs"), generated)
        .expect("failed to write embedded Agent definitions");
}

fn main() {
    generate_agent_definitions();
    tauri_build::build()
}
