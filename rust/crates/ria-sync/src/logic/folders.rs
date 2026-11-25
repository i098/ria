// Ported from ardia packages/sync/src/folders.ts at 551d3c13c.
// Copyright 2026 Mantra Patel, Apache-2.0.
// This file is a Rust port of that file, with changes.

//! The children of each folder, after a folder that is its own ancestor moves to the fallback.
//!
//! Changes from Ardia: none.

use indexmap::IndexMap;
use std::collections::HashSet;

/// Port of `folderChildren` (`packages/sync/src/folders.ts:1`). It moves each key whose chain of
/// parents loops to `fallback` in `parent_of`, then lists the children of each parent.
pub fn folder_children(
    parent_of: &mut IndexMap<String, String>,
    keys: &[String],
    fallback: &str,
) -> IndexMap<String, Vec<String>> {
    let mut rooted: HashSet<String> = HashSet::new();
    for key in keys {
        let mut path: Vec<&str> = vec![key];
        let mut seen: HashSet<&str> = HashSet::from([key.as_str()]);
        let mut looped = false;
        let mut at = parent_of.get(key);
        while let Some(parent) =
            at.filter(|a| parent_of.contains_key(a.as_str()) && !rooted.contains(a.as_str()))
        {
            if !seen.insert(parent) {
                looped = true;
                break;
            }
            path.push(parent);
            at = parent_of.get(parent);
        }
        if looped {
            parent_of.insert(key.clone(), fallback.to_owned());
            rooted.insert(key.clone());
        } else {
            rooted.extend(path.into_iter().map(str::to_owned));
        }
    }
    let mut children: IndexMap<String, Vec<String>> = IndexMap::new();
    for (key, parent) in parent_of.iter() {
        children
            .entry(parent.clone())
            .or_default()
            .push(key.clone());
    }
    children
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_loop_moves_to_the_fallback() {
        let mut parent_of: IndexMap<String, String> = [("a", "b"), ("b", "a"), ("c", "a")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let keys = ["a", "b", "c"].map(String::from);
        let children = folder_children(&mut parent_of, &keys, "other");
        assert_eq!(parent_of["a"], "other");
        assert_eq!(children["other"], vec!["a"]);
        assert_eq!(children["a"], vec!["b", "c"]);
    }
}
