use std::path::PathBuf;

/// Extension of the file a download streams into while it is still running.
///
/// Downloads usually land in a shared directory, so the transfer code that
/// creates these and the share scanner that must skip them agree here.
pub const PART_SUFFIX: &str = ".part";

#[must_use]
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix('~')
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(stripped.trim_start_matches('/'));
    }
    PathBuf::from(path)
}

/// One path component made safe to create locally: separators and characters
/// Windows rejects become `_`, and an empty, `.` or `..` name becomes `_`.
fn safe_component(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control()
                || matches!(
                    c,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
            {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').to_string();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        "_".to_string()
    } else {
        trimmed
    }
}

/// The directory a download is saved in: `<base>/<user>/<remote folder>`.
///
/// This is the layout SoulseekQt uses, so files from different peers and
/// albums never share a name. `<remote folder>` is the last folder of the
/// remote path; a bare filename goes straight under `<base>/<user>`.
///
/// Idempotent: a `base` that already ends in `<user>/<remote folder>` (a
/// persisted download being restored) is returned unchanged.
#[must_use]
pub fn download_directory_for(
    base: &str,
    username: &str,
    remote_path: &str,
) -> String {
    let mut parts: Vec<&str> = remote_path.split(['/', '\\']).collect();
    parts.pop();
    let mut suffix = PathBuf::from(safe_component(username));
    if let Some(folder) = parts.into_iter().rev().find(|p| !p.is_empty()) {
        suffix.push(safe_component(folder));
    }
    let base_path = PathBuf::from(base);
    if base_path.ends_with(&suffix) {
        return base.to_string();
    }
    base_path.join(suffix).display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_directory_nests_user_and_remote_folder() {
        assert_eq!(
            download_directory_for(
                "/music",
                "amy",
                "@@abc\\Albums\\Blur - Parklife\\01 - Girls.mp3"
            ),
            "/music/amy/Blur - Parklife"
        );
        assert_eq!(
            download_directory_for("/music", "amy", "plain.mp3"),
            "/music/amy"
        );
        assert_eq!(
            download_directory_for("/music", "a/b", "x\\..\\f.mp3"),
            "/music/a_b/_"
        );
    }

    #[test]
    fn download_directory_is_idempotent_for_restored_downloads() {
        let once = download_directory_for("/music", "amy", "x\\Album\\f.mp3");
        assert_eq!(
            download_directory_for(&once, "amy", "x\\Album\\f.mp3"),
            once
        );
    }

    #[test]
    fn no_tilde_returned_as_is() {
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
        assert_eq!(expand_tilde("rel/path"), PathBuf::from("rel/path"));
    }

    #[test]
    fn tilde_expanded_when_home_set() {
        // SAFETY: tests run single-threaded by default in this crate; if you
        // ever run with --test-threads >1 this needs serialization.
        unsafe {
            std::env::set_var("HOME", "/home/test");
        }
        assert_eq!(expand_tilde("~"), PathBuf::from("/home/test"));
        assert_eq!(
            expand_tilde("~/Downloads"),
            PathBuf::from("/home/test/Downloads")
        );
    }
}
