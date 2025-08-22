use crate::ast::{CommandDef, FlagDef};
use std::process::Command;

fn run_carapace_cmd(args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("carapace");
    for a in args {
        cmd.arg(a);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("carapace {args:?} failed to run: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        return Err(format!("carapace {:?} failed: {}", args, stderr.trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn list() -> Result<Vec<String>, String> {
    let s = run_carapace_cmd(&["--list"])?;
    Ok(s.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .filter(|name| which::which(name).is_ok())
        .map(|s| s.to_string())
        .collect())
}

pub fn list_with_desc() -> Result<Vec<(String, String)>, String> {
    let s = run_carapace_cmd(&["--list"])?;
    let out: Vec<(String, String)> = s
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            line.split_whitespace()
                .next()
                .and_then(|name| {
                    if which::which(name).is_ok() {
                        let short = if line.len() > name.len() {
                            line[name.len()..].trim().to_string()
                        } else {
                            String::new()
                        };
                        Some((name.to_string(), short))
                    } else {
                        None
                    }
                })
        })
        .collect();
    Ok(out)
}

pub fn export(cmd_name: &str) -> Result<CommandDef, String> {
    if cmd_name.trim().is_empty() {
        return Err("empty command name".to_string());
    }
    let s = run_carapace_cmd(&[cmd_name, "export"])?;

    let r: serde_json::Value = serde_json::from_str(&s)
        .map_err(|e| format!("failed to parse carapace export JSON: {e}"))?;

    fn map_raw(r: &serde_json::Value) -> CommandDef {
        let name = r
            .get("Name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let short = r
            .get("Short")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let aliases = r
            .get("Aliases")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let mut flags = Vec::new();
        if let Some(local) = r.get("LocalFlags").and_then(|v| v.as_array()) {
            for f in local {
                let long = f
                    .get("Longhand")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let shortf = f
                    .get("Shorthand")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let usage = f
                    .get("Usage")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let typ = f.get("Type").and_then(|v| v.as_str()).unwrap_or("bool");
                let fd = FlagDef {
                    longhand: long,
                    shorthand: shortf,
                    usage,
                    requires_value: typ != "bool",
                };
                flags.push(fd);
            }
        }
        let mut subs = Vec::new();
        if let Some(cmds) = r.get("Commands").and_then(|v| v.as_array()) {
            for c in cmds {
                subs.push(map_raw(c));
            }
        }
        CommandDef {
            name,
            short,
            aliases,
            flags,
            subcommands: subs,
        }
    }

    Ok(map_raw(&r))
}
