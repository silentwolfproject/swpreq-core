use crate::error::{Result, SwpreqError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct NetrcEntry {
    pub machine: String,
    pub login: String,
    pub password: String,
    pub account: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct Netrc {
    entries: HashMap<String, NetrcEntry>,
    default: Option<NetrcEntry>,
}

impl Netrc {
    pub fn load() -> Result<Option<Self>> {
        match Self::default_path() {
            Some(path) if path.exists() => Self::from_file(&path).map(Some),
            _ => Ok(None),
        }
    }

    pub fn default_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("NETRC") {
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }

        dirs_home().map(|home| home.join(".netrc"))
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SwpreqError::Io(e))?;

        Self::parse(&content)
    }

    pub fn parse(content: &str) -> Result<Self> {
        let mut entries = HashMap::new();
        let mut default: Option<NetrcEntry> = None;

        let tokens = tokenize(content);
        let mut i = 0;

        while i < tokens.len() {
            let keyword = &tokens[i];

            if keyword == "machine" {
                if i + 1 >= tokens.len() {
                    break;
                }
                let machine = tokens[i + 1].clone();
                i += 2;

                let mut entry = NetrcEntry {
                    machine: machine.clone(),
                    login: String::new(),
                    password: String::new(),
                    account: None,
                    port: None,
                };

                while i < tokens.len() {
                    let k = &tokens[i];
                    if k == "machine" || k == "default" {
                        break;
                    }
                    match k.as_str() {
                        "login" if i + 1 < tokens.len() => {
                            entry.login = tokens[i + 1].clone();
                            i += 2;
                        }
                        "password" if i + 1 < tokens.len() => {
                            entry.password = tokens[i + 1].clone();
                            i += 2;
                        }
                        "account" if i + 1 < tokens.len() => {
                            entry.account = Some(tokens[i + 1].clone());
                            i += 2;
                        }
                        "port" if i + 1 < tokens.len() => {
                            entry.port = tokens[i + 1].parse().ok();
                            i += 2;
                        }
                        _ => i += 1,
                    }
                }

                entries.insert(machine, entry);
            } else if keyword == "default" {
                i += 1;

                let mut entry = NetrcEntry {
                    machine: "default".to_string(),
                    login: String::new(),
                    password: String::new(),
                    account: None,
                    port: None,
                };

                while i < tokens.len() && tokens[i] != "machine" && tokens[i] != "default" {
                    let k = &tokens[i];
                    match k.as_str() {
                        "login" if i + 1 < tokens.len() => {
                            entry.login = tokens[i + 1].clone();
                            i += 2;
                        }
                        "password" if i + 1 < tokens.len() => {
                            entry.password = tokens[i + 1].clone();
                            i += 2;
                        }
                        _ => i += 1,
                    }
                }

                default = Some(entry);
            } else {
                i += 1;
            }
        }

        Ok(Self { entries, default })
    }

    pub fn lookup(&self, host: &str) -> Option<&NetrcEntry> {
        self.entries.get(host).or(self.default.as_ref())
    }

    pub fn has_entry(&self, host: &str) -> bool {
        self.lookup(host).is_some()
    }
}

fn tokenize(content: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;

    for c in content.chars() {
        match in_quote {
            Some(q) => {
                if c == q {
                    in_quote = None;
                } else {
                    current.push(c);
                }
            }
            None => {
                if c == '"' || c == '\'' {
                    in_quote = Some(c);
                } else if c == '#' {
                    for next in content.chars().skip_while(|x| *x != '\n') {
                        let _ = next;
                        break;
                    }
                } else if c.is_whitespace() {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                } else {
                    current.push(c);
                }
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn dirs_home() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.is_empty() {
            return Some(PathBuf::from(profile));
        }
    }
    None
}