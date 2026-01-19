use super::{Completer, CompletionItem, CompletionSource, CompletionStream, Replacement};
use std::boxed::Box;
use std::env;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs::ReadDir;
use tokio_stream::Stream;

pub struct PathCompleter {
    cwd: PathBuf,
}

impl PathCompleter {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self { cwd: cwd.into() }
    }

    pub fn with_current_dir() -> std::io::Result<Self> {
        Ok(Self {
            cwd: env::current_dir()?,
        })
    }

    pub fn home_dir(&self) -> PathBuf {
        env::home_dir().unwrap_or_else(|| self.cwd.clone())
    }

    fn expand_tilde(&self, path: &str) -> String {
        if path == "~" {
            return self.home_dir().to_string_lossy().to_string();
        }

        if let Some(rest) = path.strip_prefix("~/") {
            let home = self.home_dir();
            return format!("{}/{}", home.display(), rest);
        }

        path.to_string()
    }

    /// Parse prefix into (search_directory, filename_prefix_to_match, display_prefix)
    /// display_prefix is used to replace the original text in completion
    /// Argument `cpath` means canonicalized path.
    ///
    ///
    /// Examples:
    /// - "/etc/" -> ("/etc", "", "/etc/")
    /// - "/et"   -> ("/", "et", "/")
    /// - "~/Doc" -> ("/home/user", "Doc", "~/")
    /// - "dir/"  -> ("<cwd>/dir", "", "dir/")
    /// - "foo"   -> ("<cwd>", "foo", "")
    fn parse_prefix(
        &self,
        prefix: &str,
        cpath: &Path,
    ) -> Option<(PathBuf, String, String)> {
        let ends_in_slash = prefix.ends_with("/");
        let (search_path, fntm) = if ends_in_slash {
            (cpath.to_path_buf(), String::new())
        } else {
            // Manually split to handle "." and ".." as literal filename prefixes
            let path_str = cpath.to_string_lossy();
            if let Some(pos) = path_str.rfind('/') {
                let parent = &path_str[..pos];
                let filename = &path_str[pos + 1..];
                let parent = if parent.is_empty() { "/" } else { parent };
                (PathBuf::from(parent), filename.to_string())
            } else {
                // No slash means the whole thing is the filename, search in current context
                (PathBuf::new(), path_str.to_string())
            }
        };
        let display_prefix = match prefix.rfind('/') {
            Some(pos) => prefix[..=pos].to_string(),
            None => String::new(),
        };
        Some((search_path, fntm, display_prefix))
    }

    // We do this approach(literal prefix extracting) because Path("xxx/.").parent() ->
    // "", so as `parse_prefix` function.
    /// Soft canonicalize file path by only canonicalizing its parent path.
    /// Uses string manipulation to preserve `.` and `..` as literal filename prefixes.
    fn soft_canonicalize(
        &self,
        path: &Path,
        ends_with_slash: bool,
    ) -> std::io::Result<PathBuf> {
        if ends_with_slash {
            std::fs::canonicalize(path)
        } else {
            let path_str = path.to_string_lossy();
            if let Some(pos) = path_str.rfind('/') {
                let parent_str = &path_str[..pos];
                let filename = &path_str[pos + 1..];
                let parent = if parent_str.is_empty() {
                    Path::new("/")
                } else {
                    Path::new(parent_str)
                };
                let canonical_parent = std::fs::canonicalize(parent)?;
                Ok(canonical_parent.join(filename))
            } else {
                // No slash, relative path without directory component
                let canonical_cwd = std::fs::canonicalize(&self.cwd)?;
                Ok(canonical_cwd.join(path))
            }
        }
    }

}

impl Completer for PathCompleter {
    /// Give completions for path starts with the given prefix. For example:
    /// /etc -> None
    /// /et -> `/etc`
    /// /etc/ -> all files(and dirs) under `/etc/`
    /// ~/.local/ -> all files(and dirs) under `/home/<current-user>/.local/`
    /// dir0/ -> all files unders `<cwd/dir0>
    async fn complete(&self, prefix: &str) -> CompletionStream {
        let prefix = if prefix == "~" { "~/" } else { prefix };
        let ends_with_slash = prefix.ends_with('/');
        
        let mut path = PathBuf::from(self.expand_tilde(prefix));
        path = if (path).is_absolute() {
            path
        } else {
            self.cwd.join(path)
        };

        path = match self.soft_canonicalize(&path, ends_with_slash) {
            Ok(p) => p,
            Err(_) => {
                return Box::pin(PathCompletionStream::empty());
            }
        };

        let Some((search_dir, filename_prefix, display_prefix)) =
            self.parse_prefix(prefix, &path)
        else {
            return Box::pin(PathCompletionStream::empty());
        };

        match tokio::fs::read_dir(&search_dir).await {
            Ok(read_dir) => Box::pin(PathCompletionStream::new(
                read_dir,
                filename_prefix,
                display_prefix,
            )),
            Err(_) => Box::pin(PathCompletionStream::empty()),
        }
    }
}

struct PathCompletionStream {
    read_dir: Option<ReadDir>,
    filename_prefix: String,
    display_prefix: String,
}

impl PathCompletionStream {
    fn new(read_dir: ReadDir, filename_prefix: String, display_prefix: String) -> Self {
        Self {
            read_dir: Some(read_dir),
            filename_prefix,
            display_prefix,
        }
    }
    fn empty() -> Self {
        Self {
            read_dir: None,
            filename_prefix: String::new(),
            display_prefix: String::new(),
        }
    }
}

impl Stream for PathCompletionStream {
    type Item = CompletionItem;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = &mut *self;
        let read_dir = match &mut this.read_dir {
            Some(rd) => rd,
            None => return Poll::Ready(None),
        };
        loop {
            match read_dir.poll_next_entry(cx) {
                Poll::Ready(Ok(Some(entry))) => {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();

                    if !file_name_str.starts_with(&this.filename_prefix) {
                        continue;
                    }

                    let is_dir = entry.path().is_dir();
                    let suffix = if is_dir { "/" } else { "" };
                    let label = format!("{}{}", file_name_str, suffix);
                    let new_text = format!("{}{}", this.display_prefix, label);
                    return Poll::Ready(Some(CompletionItem {
                        label,
                        replacement: Replacement::text_only(new_text),
                        source: CompletionSource::FileSystem,
                    }));
                }
                Poll::Ready(Ok(None)) => {
                    // No more entries
                    this.read_dir = None;
                    return Poll::Ready(None);
                }
                Poll::Ready(Err(_)) => {
                    this.read_dir = None;
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case("/etc/", "/etc", Some(("/etc", "", "/etc/")))]
    #[case("/et", "/et", Some(("/", "et", "/")))]
    #[case("~/Doc", "<home>/Doc", Some(("<home>", "Doc", "~/")))]
    #[case("~/.", "<home>/.", Some(("<home>", ".", "~/")))]
    // #[case("~/.", "<home>", Some(("<home>", ".", "~/")))]
    #[case("~/Doc/", "<home>/Doc", Some(("<home>/Doc", "", "~/Doc/")))]
    #[case("dir/", "<cwd>/dir", Some(("<cwd>/dir", "", "dir/")))]
    #[case("foo", "<cwd>/foo", Some(("<cwd>", "foo", "")))]
    fn test_parse_prefix(
        #[case] prefix: &str,
        #[case] cpath: PathBuf,
        #[case] expected: Option<(&str, &str, &str)>,
    ) {
        let (sd, fptm, dp) = expected.unwrap();
        let expected = Some((PathBuf::from(sd), fptm.into(), dp.into()));
        assert_eq!(
            expected,
            PathCompleter::new("<cwd>").parse_prefix(prefix, &cpath)
        );
    }
}
