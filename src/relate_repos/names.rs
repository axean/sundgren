//! Repositories named in commit messages: GitLab's `See merge request PATH!N` lines, and the
//! hosts of `http(s)://` links.

use std::collections::HashMap;

use serde::Serialize;

use crate::git::Node;

/// A repository path that merge requests were merged from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MergeRequests {
    pub path: String,
    /// Lines naming it.
    pub count: usize,
    /// Committer times of the first and last commit naming it, unix seconds.
    pub first: i64,
    pub last: i64,
    /// The lowest and highest merge request number.
    pub lowest: u64,
    pub highest: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Host {
    pub host: String,
    /// Links to it.
    pub count: usize,
}

/// What a repository's commit messages name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Names {
    /// Oldest first.
    pub merge_requests: Vec<MergeRequests>,
    /// Most linked first.
    pub hosts: Vec<Host>,
}

/// (path, number) of each `See merge request PATH!N` in `message`.
pub fn merge_requests(message: &str) -> Vec<(&str, u64)> {
    const LEAD: &str = "See merge request ";
    message
        .match_indices(LEAD)
        .filter_map(|(at, _)| {
            let rest = &message[at + LEAD.len()..];
            let word = rest.split_whitespace().next().filter(|_| {
                // The path follows on the same line.
                !rest.starts_with(char::is_whitespace)
            })?;
            word.match_indices('!').find_map(|(bang, _)| {
                let digits: String = word[bang + 1..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect();
                let number = digits.parse().ok()?;
                (bang > 0).then(|| (&word[..bang], number))
            })
        })
        .collect()
}

/// The host of each `http://` or `https://` link in `message`, lowercased. Only names with a
/// dot and a top-level label of two or more letters count (no IP addresses or `localhost`).
pub fn hosts(message: &str) -> Vec<String> {
    message
        .match_indices("http")
        .filter_map(|(at, _)| {
            let rest = &message[at + 4..];
            let rest = rest.strip_prefix('s').unwrap_or(rest);
            let rest = rest.strip_prefix("://")?;
            let host: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
                .collect();
            let host = host.trim_end_matches(['.', '-']).to_ascii_lowercase();
            let (_, top) = host.rsplit_once('.')?;
            (top.len() >= 2 && top.chars().all(|c| c.is_ascii_alphabetic())).then_some(host)
        })
        .collect()
}

/// What the messages of the commits `nodes` name.
pub fn names<'a>(nodes: impl IntoIterator<Item = &'a Node>) -> Names {
    let mut paths: HashMap<&str, MergeRequests> = HashMap::new();
    let mut hosts_seen: HashMap<String, usize> = HashMap::new();
    for node in nodes {
        for message in [node.subject.as_str(), node.body.as_str()] {
            for (path, number) in merge_requests(message) {
                let t = node.committed;
                let entry = paths.entry(path).or_insert_with(|| MergeRequests {
                    path: path.to_owned(),
                    count: 0,
                    first: t,
                    last: t,
                    lowest: number,
                    highest: number,
                });
                entry.count += 1;
                entry.first = entry.first.min(t);
                entry.last = entry.last.max(t);
                entry.lowest = entry.lowest.min(number);
                entry.highest = entry.highest.max(number);
            }
            for host in hosts(message) {
                *hosts_seen.entry(host).or_default() += 1;
            }
        }
    }
    let mut merge_requests: Vec<MergeRequests> = paths.into_values().collect();
    merge_requests.sort_by(|a, b| (a.first, &a.path).cmp(&(b.first, &b.path)));
    let mut hosts: Vec<Host> = hosts_seen
        .into_iter()
        .map(|(host, count)| Host { host, count })
        .collect();
    hosts.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.host.cmp(&b.host)));
    Names {
        merge_requests,
        hosts,
    }
}

#[cfg(test)]
mod tests;
