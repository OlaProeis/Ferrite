//! Mermaid `gitGraph` source text → AST parsing.

use std::collections::HashMap;

use super::types::{
    GitBranch, GitCommit, GitCommitKind, GitGraph, GitGraphOrientation, GitGraphWarning,
};

/// Parse a git graph from source.
pub fn parse_git_graph(source: &str) -> Result<GitGraph, String> {
    let mut lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line))
        .collect();

    if lines.is_empty() {
        return Err("No commits found in git graph".to_string());
    }

    let (header_line_no, header_line) = lines
        .iter()
        .find(|(_, line)| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("%%")
        })
        .copied()
        .ok_or_else(|| "No commits found in git graph".to_string())?;

    let orientation = parse_orientation(header_line);

    // Body lines after the header.
    lines.retain(|(line_no, line)| {
        if *line_no <= header_line_no {
            return false;
        }
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with("%%")
    });

    let mut commits: Vec<GitCommit> = Vec::new();
    let mut branches: Vec<GitBranch> = vec![GitBranch {
        name: "main".to_string(),
        color_idx: 0,
        order: None,
    }];
    let mut current_branch = "main".to_string();
    let mut commit_counter = 0u32;
    let mut warnings: Vec<GitGraphWarning> = Vec::new();

    for (line_no, line) in lines {
        let trimmed = line.trim();
        let line_lower = trimmed.to_lowercase();

        if line_lower.starts_with("commit") {
            commit_counter += 1;
            let options = parse_options(trim_after_keyword(trimmed, "commit"));

            let id = options
                .get("id")
                .cloned()
                .unwrap_or_else(|| format!("c{commit_counter}"));

            let message = options.get("msg").cloned();
            let tag = options.get("tag").cloned();

            let kind = options
                .get("type")
                .and_then(|t| GitCommitKind::parse(t))
                .unwrap_or_default();

            if let Some(type_raw) = options.get("type") {
                if GitCommitKind::parse(type_raw).is_none() {
                    warnings.push(GitGraphWarning {
                        line: line_no,
                        message: format!("Unknown commit type: {type_raw}"),
                    });
                }
            }

            commits.push(GitCommit {
                id,
                branch: current_branch.clone(),
                message,
                tag,
                kind,
                is_merge: false,
                merge_from: None,
                is_cherry_pick: false,
                cherry_pick_from_id: None,
            });
        } else if line_lower.starts_with("branch") {
            let tail = trim_after_keyword(trimmed, "branch");
            let (name, order) = parse_branch_name_and_order(tail);
            if name.is_empty() {
                warnings.push(GitGraphWarning {
                    line: line_no,
                    message: "Branch statement missing branch name".to_string(),
                });
                continue;
            }
            ensure_branch(&mut branches, &name, order);
            current_branch = name;
        } else if line_lower.starts_with("checkout") || line_lower.starts_with("switch") {
            let keyword = if line_lower.starts_with("checkout") {
                "checkout"
            } else {
                "switch"
            };
            let name = strip_quotes(trim_after_keyword(trimmed, keyword));
            if name.is_empty() {
                warnings.push(GitGraphWarning {
                    line: line_no,
                    message: format!("{keyword} statement missing branch name"),
                });
                continue;
            }
            ensure_branch(&mut branches, &name, None);
            current_branch = name;
        } else if line_lower.starts_with("merge") {
            commit_counter += 1;
            let rest = trim_after_keyword(trimmed, "merge");
            let (merge_from, id) = if rest.to_ascii_lowercase().contains("id:") {
                let parts: Vec<&str> = rest.splitn(2, "id:").collect();
                let from = strip_quotes(parts[0].trim());
                let id = parts
                    .get(1)
                    .map(|s| parse_option_value(s.trim()).0)
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("m{commit_counter}"));
                (from, id)
            } else {
                (strip_quotes(rest), format!("m{commit_counter}"))
            };

            commits.push(GitCommit {
                id,
                branch: current_branch.clone(),
                message: Some(format!("Merge {merge_from}")),
                tag: None,
                kind: GitCommitKind::Normal,
                is_merge: true,
                merge_from: Some(merge_from),
                is_cherry_pick: false,
                cherry_pick_from_id: None,
            });
        } else if line_lower.starts_with("cherry-pick") || line_lower.starts_with("cherrypick") {
            commit_counter += 1;
            let options = parse_options(trim_after_keyword(
                trimmed,
                if line_lower.starts_with("cherry-pick") {
                    "cherry-pick"
                } else {
                    "cherrypick"
                },
            ));
            let source_id = options.get("id").cloned().unwrap_or_default();
            if source_id.is_empty() {
                warnings.push(GitGraphWarning {
                    line: line_no,
                    message: "cherry-pick statement missing id:".to_string(),
                });
                continue;
            }

            let known = commits.iter().any(|c| c.id == source_id);
            if !known {
                warnings.push(GitGraphWarning {
                    line: line_no,
                    message: format!("cherry-pick references unknown commit id: {source_id}"),
                });
            }

            commits.push(GitCommit {
                id: format!("cp{commit_counter}"),
                branch: current_branch.clone(),
                message: Some(format!("cherry-pick {source_id}")),
                tag: None,
                kind: GitCommitKind::Normal,
                is_merge: false,
                merge_from: None,
                is_cherry_pick: true,
                cherry_pick_from_id: Some(source_id),
            });
        } else {
            warnings.push(GitGraphWarning {
                line: line_no,
                message: format!("Unknown gitGraph statement: {trimmed}"),
            });
        }
    }

    if commits.is_empty() {
        return Err("No commits found in git graph".to_string());
    }

    Ok(GitGraph {
        orientation,
        commits,
        branches,
        warnings,
    })
}

fn parse_orientation(header: &str) -> GitGraphOrientation {
    let lower = header.trim().to_ascii_lowercase();
    let after = lower
        .strip_prefix("gitgraph")
        .unwrap_or("")
        .trim()
        .trim_start_matches(':')
        .trim();
    if after.starts_with("tb") {
        GitGraphOrientation::Tb
    } else {
        GitGraphOrientation::Lr
    }
}

fn trim_after_keyword<'a>(line: &'a str, keyword: &str) -> &'a str {
    let lower = line.to_ascii_lowercase();
    let Some(idx) = lower.find(keyword) else {
        return line;
    };
    line[idx + keyword.len()..].trim()
}

fn strip_quotes(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn parse_branch_name_and_order(tail: &str) -> (String, Option<u32>) {
    let lower = tail.to_ascii_lowercase();
    if let Some(idx) = lower.find("order:") {
        let name_part = tail[..idx].trim();
        let order_part = tail[idx + "order:".len()..].trim();
        let order = order_part.split_whitespace().next().and_then(|s| s.parse().ok());
        (strip_quotes(name_part), order)
    } else {
        (strip_quotes(tail.trim()), None)
    }
}

fn parse_option_value(input: &str) -> (String, &str) {
    let input = input.trim_start();
    if input.is_empty() {
        return (String::new(), input);
    }
    if let Some(q) = input.chars().next() {
        if q == '"' || q == '\'' {
            if let Some((value, rest)) = parse_quoted_value(input) {
                return (value, rest);
            }
        }
    }
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    (input[..end].to_string(), &input[end..])
}

fn parse_quoted_value(s: &str) -> Option<(String, &str)> {
    let quote = s.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let mut value = String::new();
    for (byte_idx, ch) in s.char_indices().skip(1) {
        if ch == quote {
            let rest_byte = byte_idx + ch.len_utf8();
            return Some((value, &s[rest_byte..]));
        }
        value.push(ch);
    }
    Some((value, ""))
}

fn parse_options(input: &str) -> HashMap<String, String> {
    let mut options = HashMap::new();
    let mut rest = input.trim();
    while !rest.is_empty() {
        let Some(colon) = rest.find(':') else {
            break;
        };
        let key = rest[..colon].trim().to_ascii_lowercase();
        rest = rest[colon + 1..].trim_start();
        let (value, after) = parse_option_value(rest);
        if !key.is_empty() {
            options.insert(key, value);
        }
        rest = after.trim_start();
    }
    options
}

fn ensure_branch(branches: &mut Vec<GitBranch>, name: &str, order: Option<u32>) {
    if let Some(existing) = branches.iter_mut().find(|b| b.name == name) {
        if order.is_some() {
            existing.order = order;
        }
    } else {
        branches.push(GitBranch {
            name: name.to_string(),
            color_idx: branches.len(),
            order,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::mermaid::git_graph::layout::{layout_git_graph, GitGraphLayoutConfig};

    fn parse(body: &str) -> GitGraph {
        parse_git_graph(body).expect("parse should succeed")
    }

    #[test]
    fn basic_commits_and_branches_still_parse() {
        let graph = parse(
            "gitGraph\n  commit\n  branch develop\n  commit id: \"d1\"\n  checkout main\n  merge develop",
        );
        assert_eq!(graph.commits.len(), 3);
        assert!(graph.commits[0].id.starts_with('c'));
        assert_eq!(graph.commits[1].id, "d1");
        assert!(graph.commits[2].is_merge);
        assert_eq!(graph.commits[2].merge_from.as_deref(), Some("develop"));
        assert!(graph.warnings.is_empty());
    }

    #[test]
    fn parse_commit_tag() {
        let graph = parse("gitGraph\n  commit id: \"a1\" tag: \"v1.0\"");
        assert_eq!(graph.commits[0].tag.as_deref(), Some("v1.0"));
    }

    #[test]
    fn parse_commit_type_variants() {
        let normal = parse("gitGraph\n  commit type: NORMAL");
        assert_eq!(normal.commits[0].kind, GitCommitKind::Normal);

        let reverse = parse("gitGraph\n  commit type: REVERSE");
        assert_eq!(reverse.commits[0].kind, GitCommitKind::Reverse);

        let highlight = parse("gitGraph\n  commit type: HIGHLIGHT");
        assert_eq!(highlight.commits[0].kind, GitCommitKind::Highlight);
    }

    #[test]
    fn parse_branch_order() {
        let graph = parse("gitGraph\n  branch develop order: 2\n  commit");
        let develop = graph.branches.iter().find(|b| b.name == "develop").unwrap();
        assert_eq!(develop.order, Some(2));
    }

    #[test]
    fn switch_is_checkout_alias() {
        let graph = parse("gitGraph\n  commit\n  branch feature\n  switch feature\n  commit id: \"f1\"");
        assert_eq!(graph.commits[1].branch, "feature");
        assert_eq!(graph.commits[1].id, "f1");
    }

    #[test]
    fn cherry_pick_known_id() {
        let graph = parse(
            "gitGraph\n  commit id: \"base\"\n  branch feature\n  cherry-pick id: \"base\"",
        );
        let cp = graph.commits.last().unwrap();
        assert!(cp.is_cherry_pick);
        assert_eq!(cp.cherry_pick_from_id.as_deref(), Some("base"));
        assert!(graph.warnings.is_empty());
    }

    #[test]
    fn cherry_pick_unknown_id_emits_warning() {
        let graph = parse("gitGraph\n  commit\n  cherry-pick id: \"missing\"");
        assert_eq!(graph.warnings.len(), 1);
        assert_eq!(graph.warnings[0].line, 3);
        assert!(graph
            .warnings[0]
            .message
            .contains("unknown commit id: missing"));
        let cp = graph.commits.last().unwrap();
        assert!(cp.is_cherry_pick);
    }

    #[test]
    fn parse_lr_and_tb_header() {
        assert_eq!(
            parse("gitGraph LR:\n  commit").orientation,
            GitGraphOrientation::Lr
        );
        assert_eq!(
            parse("gitGraph\n  commit").orientation,
            GitGraphOrientation::Lr
        );
        assert_eq!(
            parse("gitGraph TB:\n  commit").orientation,
            GitGraphOrientation::Tb
        );
    }

    #[test]
    fn quoted_branch_names_are_stripped() {
        let graph = parse("gitGraph\n  branch \"feat/x\"\n  commit id: \"x1\"");
        assert!(graph.branches.iter().any(|b| b.name == "feat/x"));
        assert_eq!(graph.commits[0].branch, "feat/x");
    }

    #[test]
    fn unknown_statement_emits_warning() {
        let graph = parse("gitGraph\n  commit\n  reset HEAD~1\n  commit id: \"after\"");
        assert_eq!(graph.warnings.len(), 1);
        assert_eq!(graph.warnings[0].line, 3);
        assert!(graph.warnings[0].message.contains("Unknown gitGraph statement"));
        assert_eq!(graph.commits.len(), 2);
        assert_eq!(graph.commits[1].id, "after");
    }

    #[test]
    fn empty_graph_is_error() {
        assert!(parse_git_graph("gitGraph\n").is_err());
    }

    // ── Fixture topology tests (test_md/test_git_graphs.md) ──────────────────

    const FIXTURE_FEATURE_MERGE: &str = r#"gitGraph
  commit id: "ROOT"
  branch develop
  commit id: "DEV1" msg: "Feature work"
  checkout main
  commit id: "MAIN1" msg: "Mainline fix"
  merge develop id: "MERGE1""#;

    const FIXTURE_MULTI_BRANCH_ORDER: &str = r#"gitGraph
  commit id: "BASE"
  branch hotfix order: 2
  commit id: "HF1"
  branch feature order: 1
  commit id: "FE1"
  checkout main
  commit id: "MAIN2""#;

    const FIXTURE_TAGS_CHERRY_PICK: &str = r#"gitGraph
  commit id: "abc" tag: "v1.0" type: NORMAL
  branch release
  commit id: "rel1" tag: "v1.1"
  checkout main
  cherry-pick id: "abc"
  commit type: HIGHLIGHT"#;

    #[test]
    fn fixture_feature_branch_merge_topology() {
        let graph = parse(FIXTURE_FEATURE_MERGE);
        assert_eq!(graph.commits.len(), 4);
        assert_eq!(graph.commits[0].id, "ROOT");
        assert_eq!(graph.commits[1].id, "DEV1");
        assert_eq!(graph.commits[1].branch, "develop");
        assert_eq!(graph.commits[2].id, "MAIN1");
        assert_eq!(graph.commits[3].id, "MERGE1");
        assert!(graph.commits[3].is_merge);
        assert_eq!(graph.commits[3].merge_from.as_deref(), Some("develop"));

        let layout = layout_git_graph(&graph, GitGraphLayoutConfig::default());
        assert_eq!(layout.branch_lanes.get("main"), Some(&0));
        assert_eq!(layout.branch_lanes.get("develop"), Some(&1));
        assert_eq!(layout.merge_connectors.len(), 1);
        assert_eq!(layout.merge_connectors[0].source_branch, "develop");
        assert_eq!(
            layout.merge_connectors[0].source_pos,
            layout.commits[1].pos
        );
        assert_eq!(
            layout.merge_connectors[0].target_pos,
            layout.commits[3].pos
        );
    }

    #[test]
    fn fixture_multi_branch_order_topology() {
        let graph = parse(FIXTURE_MULTI_BRANCH_ORDER);
        let layout = layout_git_graph(&graph, GitGraphLayoutConfig::default());

        assert_eq!(layout.branch_lanes.get("main"), Some(&0));
        assert_eq!(layout.branch_lanes.get("feature"), Some(&1));
        assert_eq!(layout.branch_lanes.get("hotfix"), Some(&2));

        // feature (lane 1) sits between main (0) and hotfix (2)
        let main_y = layout
            .commits
            .iter()
            .find(|c| graph.commits[c.commit_index].id == "MAIN2")
            .unwrap()
            .pos
            .y;
        let feature_y = layout
            .commits
            .iter()
            .find(|c| graph.commits[c.commit_index].id == "FE1")
            .unwrap()
            .pos
            .y;
        let hotfix_y = layout
            .commits
            .iter()
            .find(|c| graph.commits[c.commit_index].id == "HF1")
            .unwrap()
            .pos
            .y;
        assert!(main_y < feature_y);
        assert!(feature_y < hotfix_y);
    }

    #[test]
    fn fixture_tags_cherry_pick_topology() {
        let graph = parse(FIXTURE_TAGS_CHERRY_PICK);
        assert_eq!(graph.commits[0].tag.as_deref(), Some("v1.0"));
        assert_eq!(graph.commits[1].tag.as_deref(), Some("v1.1"));
        assert!(graph.commits[2].is_cherry_pick);
        assert_eq!(graph.commits[2].cherry_pick_from_id.as_deref(), Some("abc"));
        assert_eq!(graph.commits[3].kind, GitCommitKind::Highlight);

        let layout = layout_git_graph(&graph, GitGraphLayoutConfig::default());
        assert_eq!(layout.cherry_pick_connectors.len(), 1);
        assert_eq!(
            layout.cherry_pick_connectors[0].source_pos,
            layout.commits[0].pos
        );
        assert_eq!(
            layout.cherry_pick_connectors[0].target_pos,
            layout.commits[2].pos
        );
        assert_eq!(layout.branch_lanes.get("main"), Some(&0));
        assert_eq!(layout.branch_lanes.get("release"), Some(&1));
    }
}
