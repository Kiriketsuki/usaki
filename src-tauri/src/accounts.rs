//! Account discovery.
//!
//! Milestone 1 has no config file. Each Claude cache file in the usaki cache folder is one
//! account. The config file in milestone 3 replaces this discovery.

use std::io;
use std::path::Path;

use usaki_core::providers::claude_cache::{account_id_from_file_name, usaki_cache_dir};
use usaki_core::{Account, ProviderKind};

/// Returns one Claude account for each cache file under `cache_dir`, sorted by id.
///
/// The sort keeps the ring order stable between polls. A missing usaki folder gives no
/// accounts. The function skips a file whose name is not a valid cache file name.
pub fn discover(cache_dir: &Path) -> io::Result<Vec<Account>> {
    let entries = match std::fs::read_dir(usaki_cache_dir(cache_dir)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut ids: Vec<String> = entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(account_id_from_file_name)
                .map(str::to_owned)
        })
        .collect();
    ids.sort();
    Ok(ids
        .into_iter()
        .map(|id| Account::new(id, ProviderKind::ClaudeCache, None))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(dir: &Path, name: &str) {
        std::fs::write(dir.join(name), "{}").unwrap();
    }

    fn ids(accounts: &[Account]) -> Vec<&str> {
        accounts.iter().map(|a| a.id.as_str()).collect()
    }

    #[test]
    fn missing_folder_gives_no_accounts() {
        let dir = tempfile::tempdir().unwrap();
        assert!(discover(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn finds_cache_files_sorted_by_id() {
        let dir = tempfile::tempdir().unwrap();
        let usaki = usaki_cache_dir(dir.path());
        std::fs::create_dir_all(&usaki).unwrap();
        touch(&usaki, "claude-acc2.json");
        touch(&usaki, "claude-acc1.json");
        let found = discover(dir.path()).unwrap();
        assert_eq!(ids(&found), ["acc1", "acc2"]);
        assert!(
            found
                .iter()
                .all(|a| a.provider == ProviderKind::ClaudeCache)
        );
        assert!(found.iter().all(|a| a.source.is_none()));
    }

    #[test]
    fn skips_other_files_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        let usaki = usaki_cache_dir(dir.path());
        std::fs::create_dir_all(usaki.join("claude-folder.json")).unwrap();
        touch(&usaki, "claude-acc1.json");
        touch(&usaki, "claude-acc1.json.tmp");
        touch(&usaki, "claude-a.b.json");
        touch(&usaki, "notes.txt");
        assert_eq!(ids(&discover(dir.path()).unwrap()), ["acc1"]);
    }
}
