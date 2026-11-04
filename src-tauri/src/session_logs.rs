//! Últimas sessões (7 dias) do Codex e do Copilot, lidas dos arquivos locais de cada ferramenta.
//!
//! O Copilot registra tokens por chamada; o Codex desta versão não registra tokens, então
//! para ele só se conta o número de mensagens do usuário.

use crate::providers::{parse_iso_secs, SessionUse};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const WEEK_SECS: i64 = 7 * 86400;

#[derive(Default)]
struct CopilotAcc {
    offset: u64,
    tokens: u64,
}

#[derive(Default)]
pub struct SessionScanner {
    /// arquivo -> (tamanho, data de modificação, resultado já lido)
    codex: HashMap<PathBuf, (u64, i64, SessionUse)>,
    copilot: HashMap<PathBuf, CopilotAcc>,
}

fn home() -> Option<PathBuf> {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()
        .map(PathBuf::from)
}

fn tail_name(path: &str) -> String {
    path.rsplit(['\\', '/'])
        .find(|s| !s.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn mtime_secs(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Valor de uma linha simples "chave: valor" (como no workspace.yaml do Copilot).
fn yaml_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|l| {
            l.strip_prefix(key)?
                .strip_prefix(':')
                .map(|v| v.trim().trim_matches('"'))
        })
        .filter(|v| !v.is_empty())
}

fn collect_jsonl(dir: &Path, depth: u8, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth < 4 {
                collect_jsonl(&p, depth + 1, out);
            }
        } else if p.extension().and_then(|x| x.to_str()) == Some("jsonl") {
            out.push(p);
        }
    }
}

fn parse_codex(path: &Path, modified: i64) -> SessionUse {
    let mut cwd = String::new();
    let mut turns: u32 = 0;
    let mut tokens: Option<u64> = None;
    if let Ok(file) = std::fs::File::open(path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let is_meta = line.contains("session_meta");
            if !is_meta && !line.contains("\"user\"") && !line.contains("token_count") {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let p = v.get("payload");
            if is_meta {
                if let Some(c) = p.and_then(|p| p.get("cwd")).and_then(|c| c.as_str()) {
                    cwd = c.to_string();
                }
            }
            match p.and_then(|p| p.get("type")).and_then(|t| t.as_str()) {
                Some("message") if p.and_then(|p| p.get("role")).and_then(|r| r.as_str()) == Some("user") => turns += 1,
                Some("token_count") => {
                    if let Some(t) = p
                        .and_then(|p| p.pointer("/info/total_token_usage/total_tokens"))
                        .and_then(|t| t.as_u64())
                    {
                        tokens = Some(t);
                    }
                }
                _ => {}
            }
        }
    }
    let project = if cwd.is_empty() {
        "Sessão".to_string()
    } else {
        tail_name(&cwd)
    };
    SessionUse {
        title: project.clone(),
        project: cwd,
        updated_ms: (modified.max(0) as u64) * 1000,
        tokens,
        turns: Some(turns),
    }
}

/// Lê só o que foi acrescentado ao events.jsonl do Copilot e soma os tokens das chamadas.
fn read_copilot_events(path: &Path, acc: &mut CopilotAcc) {
    let Ok(meta) = std::fs::metadata(path) else { return };
    if meta.len() < acc.offset {
        *acc = CopilotAcc::default(); // arquivo recriado
    }
    if meta.len() == acc.offset {
        return;
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return;
    };
    if file.seek(SeekFrom::Start(acc.offset)).is_err() {
        return;
    }
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    loop {
        line.clear();
        let Ok(n) = reader.read_line(&mut line) else { break };
        if n == 0 || !line.ends_with('\n') {
            break;
        }
        acc.offset += n as u64;
        if !line.contains("session.usage_record") {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(u) = v.pointer("/data/usage") {
            acc.tokens += ["inputTokens", "outputTokens"]
                .iter()
                .filter_map(|k| u.get(*k).and_then(|x| x.as_u64()))
                .sum::<u64>();
        }
    }
}

fn finish(mut list: Vec<SessionUse>) -> Vec<SessionUse> {
    list.retain(|s| s.tokens.unwrap_or(0) > 0 || s.turns.unwrap_or(0) > 0);
    list.sort_by_key(|s| std::cmp::Reverse(s.updated_ms));
    list.truncate(6);
    list
}

impl SessionScanner {
    pub fn scan_codex(&mut self, now_secs: i64) -> Vec<SessionUse> {
        let Some(root) = home().map(|h| h.join(".codex").join("sessions")) else {
            return vec![];
        };
        let mut files = Vec::new();
        collect_jsonl(&root, 0, &mut files);
        let mut out = Vec::new();
        for path in files {
            let Ok(meta) = std::fs::metadata(&path) else { continue };
            let modified = mtime_secs(&meta);
            if now_secs - modified > WEEK_SECS {
                continue;
            }
            let len = meta.len();
            let cached = self
                .codex
                .get(&path)
                .filter(|(l, m, _)| *l == len && *m == modified)
                .map(|(_, _, s)| s.clone());
            let session = cached.unwrap_or_else(|| {
                let s = parse_codex(&path, modified);
                self.codex.insert(path.clone(), (len, modified, s.clone()));
                s
            });
            out.push(session);
        }
        finish(out)
    }

    pub fn scan_copilot(&mut self, now_secs: i64) -> Vec<SessionUse> {
        let Some(base) = home().map(|h| h.join(".copilot").join("session-state")) else {
            return vec![];
        };
        let Ok(rd) = std::fs::read_dir(&base) else {
            return vec![];
        };
        let mut out = Vec::new();
        for e in rd.flatten() {
            let dir = e.path();
            if !dir.is_dir() || e.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let ws = std::fs::read_to_string(dir.join("workspace.yaml")).unwrap_or_default();
            let dir_time = std::fs::metadata(&dir).map(|m| mtime_secs(&m)).unwrap_or(0);
            let updated = yaml_value(&ws, "updated_at")
                .and_then(parse_iso_secs)
                .unwrap_or(dir_time);
            if now_secs - updated > WEEK_SECS {
                continue;
            }
            let events = dir.join("events.jsonl");
            let acc = self.copilot.entry(events.clone()).or_default();
            read_copilot_events(&events, acc);
            let cwd = yaml_value(&ws, "cwd").unwrap_or("");
            let project = if cwd.is_empty() { String::new() } else { tail_name(cwd) };
            let title = yaml_value(&ws, "name")
                .map(String::from)
                .unwrap_or_else(|| project.clone());
            out.push(SessionUse {
                title: if title.is_empty() { "Sessão".into() } else { title },
                project: cwd.to_string(),
                updated_ms: (updated.max(0) as u64) * 1000,
                tokens: if acc.tokens > 0 { Some(acc.tokens) } else { None },
                turns: None,
            });
        }
        finish(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, content: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("limitgadget-test-{}-{name}", std::process::id()));
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn yaml_le_chave_exata() {
        let y = "id: abc\nuser_named: true\nname: Minha sessão\ncwd: C:\\Projetos\\app\n";
        assert_eq!(yaml_value(y, "name"), Some("Minha sessão"));
        assert_eq!(yaml_value(y, "cwd"), Some("C:\\Projetos\\app"));
        assert_eq!(yaml_value(y, "nome"), None);
    }

    #[test]
    fn nome_do_projeto_vem_do_fim_do_caminho() {
        assert_eq!(tail_name("C:\\Users\\Bruno\\Projetos\\limit-gadget"), "limit-gadget");
        assert_eq!(tail_name("/home/x/app/"), "app");
    }

    #[test]
    fn copilot_soma_tokens_e_le_so_o_acrescentado() {
        let linha = |i: u64, o: u64| {
            format!("{{\"type\":\"session.usage_record\",\"data\":{{\"usage\":{{\"inputTokens\":{i},\"outputTokens\":{o}}}}}}}\n")
        };
        let p = temp_file(
            "events.jsonl",
            &format!("{}{}{{\"type\":\"outro\"}}\n", linha(100, 20), linha(5, 1)),
        );
        let mut acc = CopilotAcc::default();
        read_copilot_events(&p, &mut acc);
        assert_eq!(acc.tokens, 126);
        // sem mudanças: não conta de novo
        read_copilot_events(&p, &mut acc);
        assert_eq!(acc.tokens, 126);
        // acrescenta uma chamada
        std::fs::write(
            &p,
            format!(
                "{}{}{{\"type\":\"outro\"}}\n{}",
                linha(100, 20),
                linha(5, 1),
                linha(10, 4)
            ),
        )
        .unwrap();
        read_copilot_events(&p, &mut acc);
        assert_eq!(acc.tokens, 140);
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn codex_conta_mensagens_do_usuario() {
        let conteudo = concat!(
            "{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"C:\\\\Projetos\\\\clipo\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"assistant\"}}\n",
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\"}}\n",
        );
        let p = temp_file("rollout.jsonl", conteudo);
        let s = parse_codex(&p, 1_000);
        assert_eq!(s.title, "clipo");
        assert_eq!(s.turns, Some(2));
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn lista_final_ordena_e_limita() {
        let mk = |t: u64| SessionUse {
            title: "x".into(),
            updated_ms: t,
            turns: Some(1),
            ..Default::default()
        };
        let out = finish((1..=9).map(mk).collect());
        assert_eq!(out.len(), 6);
        assert_eq!(out[0].updated_ms, 9);
    }
}
