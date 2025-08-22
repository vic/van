// Entry point: program main
// Handles --hook, --exe, --help, and runs the TUI
//
// TUI Docs: https://github.com/whit3rabbit/bubbletea-rs look for related crates there and examples on each of them.

use std::env;
use std::fs;
use std::path::Path;
use std::process::{self, Command, Stdio};
use van::ui::{Model as UiModel, initial_model, run as noninteractive_run};

use bubbletea_rs::{
    Program, event::KeyMsg, event::WindowSizeMsg, model::Model as TeaModel, window_size,
};
use crossterm::event::{KeyCode, KeyModifiers};

// Adapter type implementing bubbletea-rs Model trait by delegating to our UiModel
struct TeaAdapter {
    inner: UiModel,
}

impl TeaModel for TeaAdapter {
    fn init() -> (Self, Option<bubbletea_rs::command::Cmd>) {
        // preload carapace --list with descriptions so interactive UI shows top-level commands immediately
        let entries = van::carapace::list_with_desc().unwrap_or_default();
        let mut adapter = TeaAdapter {
            inner: initial_model(entries),
        };
        let (width, height) = crossterm::terminal::size().unwrap_or((80, 24));
        adapter.inner.update(van::ui::Msg::WindowSize {
            width: width as usize,
            height: height as usize,
        });
        let cmd = window_size();
        (adapter, Some(cmd))
    }

    fn update(&mut self, msg: bubbletea_rs::event::Msg) -> Option<bubbletea_rs::command::Cmd> {
        // Map bubbletea-rs Msg types to our ui::Msg and call update
        if let Some(km) = msg.downcast_ref::<KeyMsg>() {
            // Structured handling using crossterm types (KeyCode, KeyModifiers)
            match &km.key {
                KeyCode::Enter => {
                    // Enter -> perform ExecProcess semantics
                    self.inner.update(van::ui::Msg::KeyEnter);
                    let preview = &self.inner.exit_preview;
                    if preview.is_empty() {
                        return None;
                    }
                    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
                    let mut cmd = Command::new(shell);
                    cmd.arg("-c")
                        .arg(preview)
                        .stdin(Stdio::inherit())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit());
                    match cmd.status() {
                        Ok(status) => {
                            if let Some(code) = status.code() {
                                process::exit(code);
                            } else {
                                process::exit(0);
                            }
                        }
                        Err(e) => {
                            eprintln!("failed to execute command: {e}");
                            process::exit(1);
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.inner.update(van::ui::Msg::KeyBackspace);
                }
                KeyCode::Esc => {
                    // Quit immediately unless we're in value-input mode
                    if !self.inner.in_value_mode {
                        return Some(bubbletea_rs::quit());
                    }
                    self.inner.update(van::ui::Msg::KeyEsc);
                }
                KeyCode::Up => {
                    self.inner.update(van::ui::Msg::KeyUp);
                }
                KeyCode::Down => {
                    self.inner.update(van::ui::Msg::KeyDown);
                }
                KeyCode::Char(ch) => {
                    // Control-key handling
                    if km.modifiers.contains(KeyModifiers::CONTROL) {
                        match ch {
                            'n' | 'N' => {
                                self.inner.update(van::ui::Msg::KeyDown);
                            }
                            'p' | 'P' => {
                                self.inner.update(van::ui::Msg::KeyUp);
                            }
                            'c' | 'C' => {
                                return Some(bubbletea_rs::quit());
                            }
                            _ => {}
                        }
                    } else if *ch == ' ' {
                        self.inner.update(van::ui::Msg::KeySpace);
                    } else {
                        self.inner.update(van::ui::Msg::Rune(*ch));
                    }
                }
                _ => { /* ignore other keys */ }
            }

            return None;
        }
        if let Some(ws) = msg.downcast_ref::<WindowSizeMsg>() {
            self.inner.update(van::ui::Msg::WindowSize {
                width: ws.width as usize,
                height: ws.height as usize,
            });
            return None;
        }
        None
    }

    fn view(&self) -> String {
        // delegate to UiModel's styled renderer
        self.inner.render_full()
    }
}

fn print_help() {
    println!("van - interactive command completion preview tool");
    println!();
    println!("Usage:");
    println!("  van [<command> [args...]]");
    println!();
    println!("Options:");
    println!(
        "  --hook <shell>   Output shell hook code for <shell>. Supported: bash, zsh, fish, nushell. If <shell> omitted, auto-detects from $SHELL and falls back to bash."
    );
    println!(
        "  --exe <cmd>      Optional: override the executable string to embed in the hook (e.g. './target/debug/van')."
    );
    println!("  --help           Show this help message.");
    println!();
    println!("Description:");
    println!(
        "  When the hook is installed in your shell, your shell will invoke \"<exe> <command line>\" to produce completion candidates for the currently typed command line. For example, if you type 'jj commit' and press TAB, the shell will call '<exe> jj commit' to obtain completion items."
    );
    println!();
    println!("Installation example (bash):");
    println!("  van --hook bash > ~/.van_hook.sh");
    println!("  source ~/.van_hook.sh");
}

// shell_single_quote safely single-quotes s for embedding in POSIX shells.
fn shell_single_quote(s: &str) -> String {
    if s.is_empty() {
        return "''".to_string();
    }
    let escaped = s.replace('\'', "'\\''");
    format!("'{escaped}'")
}

// parse_run_from_parts tries to find a '<exe> run' invocation in parts and reconstruct the run command string
fn parse_run_from_parts(parts: &[String]) -> Option<String> {
    // look for a pair where the second token is "run" and then collect valid run args after it
    parts.windows(2).enumerate().find_map(|(i, pair)| {
        if pair[1] != "run" {
            return None;
        }
        let run_args: Vec<String> = parts
            .iter()
            .skip(i + 2)
            .take_while(|t| {
                if t.is_empty() || t.starts_with('-') {
                    return false;
                }
                if Path::new(t).exists() {
                    return true;
                }
                // Treat explicit paths or relative paths as run arguments
                if t.contains('/') || t.starts_with("./") || t.starts_with("../") {
                    return true;
                }
                false
            })
            .cloned()
            .collect();

        if run_args.is_empty() {
            None
        } else {
            Some(format!("run {}", run_args.join(" ")))
        }
    })
}

// detect_exec_from_parent: attempts to determine the original executable string used to invoke this program.
fn detect_exec_from_parent() -> String {
    // default to argv[0]
    let default_exe = env::args().next().unwrap_or_default();

    // try to determine parent pid via ps -p <pid> -o ppid=
    let pid = process::id();
    let ppid_out = Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("ppid=")
        .output();
    if let Ok(out) = ppid_out {
        if out.status.success() {
            if let Ok(ppid_str) = String::from_utf8(out.stdout) {
                if let Ok(ppid) = ppid_str.trim().parse::<u32>() {
                    // platform-specific detection
                    if cfg!(target_os = "linux") {
                        // linux: try reading /proc/<ppid>/cmdline
                        let proc_cmd = format!("/proc/{ppid}/cmdline");
                        if let Ok(data) = fs::read(&proc_cmd) {
                            // cmdline is NUL-separated; convert bytes to UTF-8 and split on NULs
                            if let Ok(raw) = String::from_utf8(data) {
                                let parts: Vec<String> = raw
                                    .split('\0')
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string())
                                    .collect();
                                if let Some(r) = parse_run_from_parts(&parts) {
                                    return r;
                                }
                            }
                        }
                        // fallback: use ps -p <ppid> -o command=
                        if let Some(cmdline) = get_ps_command(ppid) {
                            let cmdline = cmdline.trim();
                            if let Some(idx) = cmdline.find("run ") {
                                let rest = cmdline[idx + "run ".len()..].trim();
                                if !rest.is_empty() {
                                    return format!("run {rest}");
                                }
                            }
                        }
                    } else if cfg!(target_os = "macos") {
                        if let Some(cmdline) = get_ps_command(ppid) {
                            let cmdline = cmdline.trim();
                            if let Some(idx) = cmdline.find("run ") {
                                let rest = cmdline[idx + "run ".len()..].trim();
                                if !rest.is_empty() {
                                    return format!("run {rest}");
                                }
                            }
                        }
                    } else if cfg!(target_os = "windows") {
                        // windows: query via PowerShell
                        let ps_cmd = format!(
                            "Get-CimInstance Win32_Process -Filter \"ProcessId={ppid}\" | Select-Object -ExpandProperty CommandLine"
                        );
                        let out = Command::new("powershell")
                            .arg("-NoProfile")
                            .arg("-Command")
                            .arg(ps_cmd)
                            .output();
                        if let Ok(o2) = out {
                            if o2.status.success() {
                                if let Ok(cmdline) = String::from_utf8(o2.stdout) {
                                    let cmdline = cmdline.trim();
                                    if !cmdline.is_empty() {
                                        if let Some(idx) = cmdline.to_lowercase().find("run ") {
                                            // preserve original-case remainder
                                            let rest = cmdline[idx + "run ".len()..].trim();
                                            if !rest.is_empty() {
                                                return format!("run {rest}");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(cmdline) = get_ps_command(ppid) {
                        let cmdline = cmdline.trim();
                        if let Some(idx) = cmdline.find("run ") {
                            let rest = cmdline[idx + "run ".len()..].trim();
                            if !rest.is_empty() {
                                return format!("run {rest}");
                            }
                        }
                    }
                }
            }
        }
    }

    // If we didn't detect a wrapper like 'run', return the invocation actually used.
    default_exe
}

// hook_script returns a shell-specific hook that will invoke exec_cmd to obtain completion items.
fn hook_script(shell: &str, exec_cmd: &str) -> String {
    let s = shell.to_lowercase();
    // single-quoted exec_cmd for safe embedding
    let esc = shell_single_quote(exec_cmd);
    // Use template placeholders {{EXEC}} then replace to avoid Rust format! interpreting shell braces
    match s.as_str() {
        "bash" => {
            let tpl = r#"# van bash hook
EXEC_CMD={{EXEC}}
_van_completion() {
  local cur compword i
  cur="${COMP_WORDS[COMP_CWORD]}"
  # build args: skip the command itself
  local args=()
  for ((i=1;i<${#COMP_WORDS[@]};i++)); do
    args+=("${COMP_WORDS[i]}")
  done
  local IFS=$'\n'
  local out
  out=$(eval "$EXEC_CMD \"${args[@]}\"") || return
  COMPREPLY=($(compgen -W "$out" -- "$cur"))
}
# Register _van_completion for all commands found in PATH (may be slow on very large PATHs)
for cmd in $(compgen -c); do
  complete -F _van_completion -o default "$cmd" 2>/dev/null || true
done
"#;
            tpl.replace("{{EXEC}}", &esc)
        }
        "zsh" => {
            let tpl = r#"# van zsh hook
EXEC_CMD={{EXEC}}
_van_completion() {
  # words array contains all words; remove the command itself
  local -a reply
  reply=("${(@f)$(eval "$EXEC_CMD ${words[1,-1]}")}")
  if [[ -n ${reply} ]]; then
    compadd -- "${reply[@]}"
  fi
}
# Register for all commands available in this shell
for cmd in ${(k)commands}; do
  compdef _van_completion $cmd 2>/dev/null || true
done
"#;
            tpl.replace("{{EXEC}}", &esc)
        }
        "fish" => {
            let tpl = r#"# van fish hook
set -l VAN_EXEC {{EXEC}}
function __van_completion
  # get full commandline
  set -l cmdline (commandline -cp)
  # split into tokens by space (basic split)
  set -l tokens (string split ' ' -- $cmdline)
  # drop the leading command name
  set -e tokens[1]
  # call $VAN_EXEC with remaining tokens and print each candidate on its own line
  for item in (eval "$VAN_EXEC $tokens")
    printf "%s\n" "$item"
  end
end
# Register completion for every executable in $PATH (may be slow)
for p in (string split : $PATH)
  for cmd in (ls $p 2>/dev/null)
    complete -c $cmd -f -a '(__van_completion)'
  end
end
"#;
            tpl.replace("{{EXEC}}", &esc)
        }
        "nushell" | "nu" => {
            let tpl = r#"# van nushell hook
# Nushell custom completion support varies by version. The following provides a simple helper function
# you can call from your nushell config to get completions for the current command line.
# Example (in your config):
#   def van-complete [] { {{EXEC_RAW}} ($nu.env.CMDLINE | split ' ' | skip 1) }
# Consult nushell docs for registering completion functions in your version.
"#;
            // nushell example uses unquoted raw exec_cmd; provide raw (not shell-single-quoted) replacement
            tpl.replace("{{EXEC_RAW}}", exec_cmd)
        }
        _ => {
            let tpl = r#"# van (default=bash) hook
EXEC_CMD={{EXEC}}
_van_completion() {
  local cur compword i
  cur="${COMP_WORDS[COMP_CWORD]}"
  # build args: skip the command itself
  local args=()
  for ((i=1;i<${#COMP_WORDS[@]};i++)); do
    args+=("${COMP_WORDS[i]}")
  done
  local IFS=$'\n'
  local out
  out=$(eval "$EXEC_CMD \"${args[@]}\"") || return
  COMPREPLY=($(compgen -W "$out" -- "$cur"))
}
# Register _van_completion for all commands found in PATH (may be slow on very large PATHs)
for cmd in $(compgen -c); do
  complete -F _van_completion -o default "$cmd" 2>/dev/null || true
done
"#;
            tpl.replace("{{EXEC}}", &esc)
        }
    }
}

fn detect_shell_from_env() -> String {
    env::var("SHELL")
        .ok()
        .and_then(|p| {
            Path::new(&p)
                .file_name()
                .and_then(|s| s.to_str().map(|s| s.to_string()))
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "bash".to_string())
}

fn get_ps_command(ppid: u32) -> Option<String> {
    let out = Command::new("ps")
        .arg("-p")
        .arg(ppid.to_string())
        .arg("-o")
        .arg("command=")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout)
        .ok()
        .map(|s| s.trim().to_string())
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    // simple flag handling for --help and --hook
    if !args.is_empty() {
        if args[0] == "--help" || args[0] == "-h" {
            print_help();
            return;
        }
        // support: --hook [shell] and optional --exe <cmd> (can appear before or after)
        let mut hook_idx: isize = -1;
        let mut exe_val = String::new();
        let mut i = 0usize;
        while i < args.len() {
            if args[i] == "--hook" {
                hook_idx = i as isize;
                // if next arg exists and doesn't start with '-', treat as shell token
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    i += 1;
                }
                i += 1;
                continue;
            }
            if args[i] == "--exe" && i + 1 < args.len() {
                exe_val = args[i + 1].to_owned();
                i += 2;
                continue;
            }
            i += 1;
        }
        if hook_idx != -1 {
            // determine shell param if provided
            let shell = if (hook_idx as usize) + 1 < args.len()
                && !args[(hook_idx as usize) + 1].starts_with('-')
            {
                args[(hook_idx as usize) + 1].to_owned()
            } else {
                detect_shell_from_env()
            };
            let mut exe_cmd = exe_val;
            if exe_cmd.is_empty() {
                exe_cmd = detect_exec_from_parent();
            }
            if exe_cmd.is_empty() {
                exe_cmd = Path::new(&env::args().next().unwrap_or_default())
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
            }
            print!("{}", hook_script(&shell, &exe_cmd));
            return;
        }
    }

    // If args provided, use non-interactive parsing similar to tooling (<cmd> args), else run interactive TUI
    if !args.is_empty() {
        match noninteractive_run(args) {
            Ok(out) => {
                if !out.is_empty() {
                    println!("{out}");
                }
                process::exit(0);
            }
            Err(e) => {
                eprintln!("{e}");
                process::exit(2);
            }
        }
    }

    // Run interactive program
    let builder = Program::<TeaAdapter>::builder();
    let program = match builder.build() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("failed to build program: {e:?}");
            process::exit(2);
        }
    };
    match program.run().await {
        Ok(_final_model) => {
            // Interactive run does not print preview; simply exit successfully
            process::exit(0);
        }
        Err(e) => {
            eprintln!("program error: {e:?}");
            process::exit(2);
        }
    }
}
