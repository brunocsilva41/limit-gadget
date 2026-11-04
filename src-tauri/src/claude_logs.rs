//! Uso do Claude Code por projeto, lido dos logs locais (~/.claude/projects/*/*.jsonl).
//! Só soma tokens (entrada, saída e criação de cache) das mensagens do assistente.

use crate::providers::{parse_iso_secs, ProjectUse};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const RECENT_SECS: i64 = 8 * 86400;

#[derive(Default)]
pub struct LogScanner {
    /// Até onde cada arquivo já foi lido.
    offsets: HashMap<PathBuf, u64>,
    /// Mensagens já contadas (o log repete a mesma mensagem em alguns casos).
    seen: HashSet<String>,
    /// (pasta do projeto, dia local) -> tokens
    days: HashMap<(String, i64), u64>,
    /// pasta do projeto -> nome para exibir
    names: HashMap<String, String>,
}

fn projects_dir() -> Option<PathBuf> {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok()?;
    Some(PathBuf::from(home).join(".claude").join("projects"))
}

fn short_name(folder: &str) -> String {
    let n: Vec<char> = folder.chars().collect();
    if n.len() <= 26 {
        folder.to_string()
    } else {
        n[n.len() - 26..].iter().collect()
    }
}

/// Soma de tokens de uma mensagem (entrada + saída + criação de cache).
fn tokens_of(v: &Value) -> u64 {
    let Some(u) = v.pointer("/message/usage") else { return 0 };
    ["input_tokens", "output_tokens", "cache_creation_input_tokens"]
        .iter()
        .filter_map(|k| u.get(*k).and_then(|x| x.as_u64()))
        .sum()
}

impl LogScanner {
    /// Lê só o que foi acrescentado desde a última varredura e devolve os projetos mais usados.
    pub fn scan(&mut self, now_secs: i64, tz: i64) -> Vec<ProjectUse> {
        let today = (now_secs + tz).div_euclid(86400);
        if let Some(root) = projects_dir() {
            if let Ok(rd) = std::fs::read_dir(&root) {
                for entry in rd.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        self.scan_project(&path, now_secs, tz);
                    }
                }
            }
        }
        self.days.retain(|(_, d), _| *d >= today - 8);

        // junta pastas diferentes que apontam para o mesmo nome de projeto
        let mut per: HashMap<String, (u64, u64)> = HashMap::new();
        for ((folder, day), tokens) in &self.days {
            if *day >= today - 6 {
                let name = self.names.get(folder).cloned().unwrap_or_else(|| short_name(folder));
                let e = per.entry(name).or_default();
                e.0 += tokens;
                if *day == today {
                    e.1 += tokens;
                }
            }
        }
        let mut out: Vec<ProjectUse> = per
            .into_iter()
            .filter(|(_, (week, _))| *week > 0)
            .map(|(name, (week, day))| ProjectUse {
                name,
                tokens_7d: week,
                tokens_today: day,
            })
            .collect();
        out.sort_by_key(|p| std::cmp::Reverse(p.tokens_7d));
        out.truncate(6);
        out
    }

    fn scan_project(&mut self, dir: &Path, now_secs: i64, tz: i64) {
        let folder = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for f in rd.flatten() {
            let path = f.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Ok(meta) = f.metadata() else { continue };
            let modified = meta
                .modified()
                .ok()
                .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if now_secs - modified > RECENT_SECS {
                continue;
            }
            let len = meta.len();
            let mut off = *self.offsets.get(&path).unwrap_or(&0);
            if len < off {
                off = 0; // arquivo foi recriado
            }
            if len == off {
                continue;
            }
            let Ok(mut file) = std::fs::File::open(&path) else {
                continue;
            };
            if file.seek(SeekFrom::Start(off)).is_err() {
                continue;
            }
            let mut reader = BufReader::new(file);
            let mut line = String::new();
            loop {
                line.clear();
                let Ok(n) = reader.read_line(&mut line) else { break };
                if n == 0 || !line.ends_with('\n') {
                    break; // fim do arquivo ou linha ainda incompleta
                }
                off += n as u64;
                if !line.contains("\"usage\"") {
                    continue;
                }
                let Ok(v) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if let Some(cwd) = v.get("cwd").and_then(|c| c.as_str()) {
                    if let Some(last) = cwd.rsplit(['\\', '/']).find(|s| !s.is_empty()) {
                        self.names.insert(folder.clone(), last.to_string());
                    }
                }
                let tokens = tokens_of(&v);
                if tokens == 0 {
                    continue;
                }
                if let (Some(id), Some(req)) = (
                    v.pointer("/message/id").and_then(|i| i.as_str()),
                    v.get("requestId").and_then(|r| r.as_str()),
                ) {
                    if !self.seen.insert(format!("{id}:{req}")) {
                        continue;
                    }
                }
                let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()).and_then(parse_iso_secs) else {
                    continue;
                };
                *self
                    .days
                    .entry((folder.clone(), (ts + tz).div_euclid(86400)))
                    .or_default() += tokens;
            }
            self.offsets.insert(path, off);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soma_entrada_saida_e_criacao_de_cache() {
        let v: Value = serde_json::from_str(
            r#"{"message":{"usage":{"input_tokens":10,"output_tokens":5,"cache_creation_input_tokens":3,"cache_read_input_tokens":900}}}"#,
        )
        .unwrap();
        assert_eq!(tokens_of(&v), 18);
    }

    #[test]
    fn mensagem_sem_uso_vale_zero() {
        let v: Value = serde_json::from_str(r#"{"message":{"role":"user"}}"#).unwrap();
        assert_eq!(tokens_of(&v), 0);
    }

    #[test]
    fn nome_curto_limita_o_tamanho() {
        assert_eq!(short_name("abc"), "abc");
        assert_eq!(
            short_name("C--Users-Bruno-Silva-Documents-Projetos-limit-gadget")
                .chars()
                .count(),
            26
        );
    }
}
