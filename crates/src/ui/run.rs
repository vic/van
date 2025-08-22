use crate::carapace;
use crate::ui::model::Model;
use crate::ui::model::initial_model;
use bubbletea_rs::{
    Program, command::Cmd, event::KeyMsg, event::WindowSizeMsg, model::Model as TeaModel,
};
use crossterm::event::{KeyCode, KeyModifiers};

// helper to build forms for a FlagDef
fn flag_forms(f: &crate::ast::FlagDef) -> Vec<String> {
    let mut forms = Vec::new();
    if !f.longhand.is_empty() {
        forms.push(format!("--{}", f.longhand));
    }
    if !f.shorthand.is_empty() {
        forms.push(format!("-{}", f.shorthand));
    }
    forms
}

// Keep the interactive runner and the non-interactive parsing behavior here.
pub fn run(initial_args: Vec<String>) -> Result<String, String> {
    // preload carapace --list with descriptions
    let entries = match carapace::list_with_desc() {
        Ok(e) => e,
        Err(err) => return Err(format!("carapace --list failed: {err}")),
    };
    let mut m = initial_model(entries);

    if !initial_args.is_empty() {
        // set root
        let root = &initial_args[0];
        match carapace::export(root) {
            Ok(def) => {
                m.ast.root = def.name.clone();
                if !m.ast.stack.is_empty() {
                    m.ast.stack[0].name = def.name.clone();
                }
                m.build_items_from_command(&def);
                m.current = Some(def);
            }
            Err(e) => return Err(format!("carapace {root} export failed: {e}")),
        }

        // parse remaining tokens
        let mut i = 1usize;
        while i < initial_args.len() {
            let tok = &initial_args[i];
            if tok.starts_with('-') {
                // flag form; find exact-form match among current.Flags
                let mut matched = false;
                if let Some(cur) = &m.current {
                    for f in &cur.flags {
                        for fm in flag_forms(f).iter() {
                            if fm == tok {
                                // add flag; if requires value and next arg exists and isn't a flag, consume it
                                let mut val = String::new();
                                if f.requires_value
                                    && i + 1 < initial_args.len()
                                    && !initial_args[i + 1].starts_with('-')
                                {
                                    val = initial_args[i + 1].clone();
                                    i += 1;
                                }
                                m.ast.add_flag(fm, &val);
                                matched = true;
                                break;
                            }
                        }
                        if matched {
                            break;
                        }
                    }
                }
                if !matched {
                    m.ast.add_positional(tok);
                }
                i += 1;
                continue;
            }
            // not a flag: could be subcommand or positional
            let mut found = false;
            if let Some(cur) = m.current.clone() {
                for sc in cur.subcommands.iter() {
                    if sc.name == *tok || sc.aliases.iter().any(|a| a == tok) {
                        m.ast.push_subcommand(&sc.name);
                        m.current = Some(sc.clone());
                        m.build_items_from_command(sc);
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                m.ast.add_positional(tok);
            }
            i += 1;
        }
    }

    // If initial_args were provided we are non-interactive: return the recorded preview (may be empty)
    if !initial_args.is_empty() {
        return Ok(m.exit_preview.clone());
    }

    // Interactive path: build a TeaAdapter that delegates to our Model and run the bubbletea-rs Program.
    struct TeaAdapter {
        inner: Model,
    }

    impl TeaModel for TeaAdapter {
        fn init() -> (Self, Option<Cmd>) {
            // Preload entries for interactive session (best-effort)
            let entries = carapace::list_with_desc().unwrap_or_default();
            let model = initial_model(entries);
            (TeaAdapter { inner: model }, None)
        }

        fn update(&mut self, msg: bubbletea_rs::event::Msg) -> Option<Cmd> {
            // Map bubbletea-rs Msg types to our ui::Msg and call update
            if let Some(km) = msg.downcast_ref::<KeyMsg>() {
                // Normalize and handle global quit keys first for reliability across terminals:
                match &km.key {
                    KeyCode::Esc => {
                        if !self.inner.in_value_mode {
                            return Some(bubbletea_rs::quit());
                        }
                        self.inner.update(crate::ui::Msg::KeyEsc);
                        return None;
                    }
                    KeyCode::Char(ch) => {
                        if *ch == '\u{1b}' {
                            if !self.inner.in_value_mode {
                                return Some(bubbletea_rs::quit());
                            }
                            self.inner.update(crate::ui::Msg::KeyEsc);
                            return None;
                        }
                        if *ch == '\u{03}' {
                            // Ctrl-C delivered as ETX
                            return Some(bubbletea_rs::quit());
                        }
                        if km.modifiers.contains(KeyModifiers::CONTROL)
                            && (*ch == 'c' || *ch == 'C')
                        {
                            return Some(bubbletea_rs::quit());
                        }
                    }
                    _ => {}
                }

                match &km.key {
                    KeyCode::Enter => {
                        self.inner.update(crate::ui::Msg::KeyEnter);
                        if !self.inner.exit_preview.is_empty() {
                            return Some(bubbletea_rs::quit());
                        }
                    }
                    KeyCode::Backspace => {
                        self.inner.update(crate::ui::Msg::KeyBackspace);
                    }
                    KeyCode::Esc => { /* handled above */ }
                    KeyCode::Up => {
                        self.inner.update(crate::ui::Msg::KeyUp);
                    }
                    KeyCode::Down => {
                        self.inner.update(crate::ui::Msg::KeyDown);
                    }
                    KeyCode::Char(ch) => {
                        if km.modifiers.contains(KeyModifiers::CONTROL) {
                            match ch {
                                'n' | 'N' => {
                                    self.inner.update(crate::ui::Msg::KeyDown);
                                }
                                'p' | 'P' => {
                                    self.inner.update(crate::ui::Msg::KeyUp);
                                }
                                _ => {}
                            }
                        } else if *ch == ' ' {
                            self.inner.update(crate::ui::Msg::KeySpace);
                        } else {
                            self.inner.update(crate::ui::Msg::Rune(*ch));
                        }
                    }
                    _ => {}
                }
                return None;
            }
            if let Some(ws) = msg.downcast_ref::<WindowSizeMsg>() {
                self.inner.update(crate::ui::Msg::WindowSize {
                    width: ws.width as usize,
                    height: ws.height as usize,
                });
                return None;
            }
            None
        }

        fn view(&self) -> String {
            self.inner.render_full()
        }
    }

    let builder = Program::<TeaAdapter>::builder()
        .alt_screen(true)
        .signal_handler(true);
    let program = match builder.build() {
        Ok(p) => p,
        Err(e) => return Err(format!("failed to build program: {e:?}")),
    };
    let final_adapter = match futures::executor::block_on(program.run()) {
        Ok(fa) => fa,
        Err(e) => return Err(format!("program error: {e:?}")),
    };

    Ok(final_adapter.inner.exit_preview.clone())
}
