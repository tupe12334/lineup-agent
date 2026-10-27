use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Find all package.json files under `root`.
///
/// Prunes `node_modules` and `.git` instead of walking into them, and honors
/// `.gitignore` / `.ignore` files so build output (dist/, target/, ...) is skipped.
pub(crate) fn find_package_jsons(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .filter_entry(|e| e.file_name() != "node_modules" && e.file_name() != ".git")
        .build()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()) && e.file_name() == "package.json")
        .map(ignore::DirEntry::into_path)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn skips_node_modules_and_gitignored_dirs() {
        let root = TempDir::new().unwrap();
        let r = root.path();
        for dir in ["", "pkg", "node_modules/dep", "dist"] {
            fs::create_dir_all(r.join(dir)).unwrap();
            fs::write(r.join(dir).join("package.json"), "{}").unwrap();
        }
        fs::write(r.join(".gitignore"), "dist/\n").unwrap();

        let mut found = find_package_jsons(r);
        found.sort();
        assert_eq!(
            found,
            vec![r.join("package.json"), r.join("pkg/package.json")]
        );
    }
}
