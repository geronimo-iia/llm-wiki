//! Filesystem path helpers shared across the engine.

use std::path::PathBuf;

/// Strip the Windows extended-length (verbatim) prefix from a path.
///
/// On Windows, `std::fs::canonicalize` returns paths with the `\\?\` verbatim
/// prefix (and `\\?\UNC\` for network shares). Clients that turn a returned
/// path into a `file://` URL cannot parse the `?`, so the prefix is removed
/// before a path leaves the engine. The path is converted back to its normal
/// form:
///
/// - `\\?\C:\dir`         becomes `C:\dir`
/// - `\\?\UNC\srv\share`  becomes `\\srv\share`
///
/// Paths without a verbatim prefix are returned unchanged, so this is a no-op
/// on non-Windows platforms.
pub(crate) fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{rest}"))
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_verbatim_disk_prefix() {
        let p = PathBuf::from(r"\\?\C:\Users\me\wikis\ddlc\wiki\log.md");
        assert_eq!(
            strip_verbatim_prefix(p),
            PathBuf::from(r"C:\Users\me\wikis\ddlc\wiki\log.md")
        );
    }

    #[test]
    fn strips_verbatim_unc_prefix() {
        let p = PathBuf::from(r"\\?\UNC\server\share\wiki");
        assert_eq!(
            strip_verbatim_prefix(p),
            PathBuf::from(r"\\server\share\wiki")
        );
    }

    #[test]
    fn leaves_plain_windows_path_unchanged() {
        let p = PathBuf::from(r"C:\Users\me\wiki");
        assert_eq!(strip_verbatim_prefix(p.clone()), p);
    }

    #[test]
    fn leaves_posix_path_unchanged() {
        let p = PathBuf::from("/home/me/wikis/ddlc/wiki/log.md");
        assert_eq!(strip_verbatim_prefix(p.clone()), p);
    }
}
