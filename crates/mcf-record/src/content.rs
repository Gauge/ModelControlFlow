use std::path::{Path, PathBuf};

use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

const WHERE: Subsystem = Subsystem::new("mcf-record::content");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whose {
    User,
    Fixture,
}

impl Whose {
    #[must_use]
    pub const fn directory(self) -> &'static str {
        match self {
            Self::User => "content",
            Self::Fixture => "fixtures",
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Fixture => "fixture",
        }
    }

    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "user" => Some(Self::User),
            "fixture" => Some(Self::Fixture),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Content {
    text: String,
}

impl Content {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    #[must_use]
    pub fn disclose(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn length_bytes(&self) -> usize {
        self.text.len()
    }
}

impl core::fmt::Debug for Content {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Content({} bytes, undisclosed)", self.text.len())
    }
}

#[derive(Debug)]
pub struct ContentStore {
    path: PathBuf,
    whose: Whose,
}

impl ContentStore {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_for(path, Whose::User)
    }

    pub fn open_for(path: &Path, whose: Whose) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                Failure::new(
                    Category::RecordUnwritable,
                    Attribution::Machine,
                    Disposition::Refused,
                    WHERE,
                    "the content store could not be created",
                )
                .with_context("path", path.display().to_string())
                .with_context("os_error", error.to_string())
            })?;
        }
        Ok(Self {
            path: path.to_path_buf(),
            whose,
        })
    }

    #[must_use]
    pub const fn whose(&self) -> Whose {
        self.whose
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn keep(&self, key: &str, content: &Content) -> Result<()> {
        let path = self.file(key)?;
        std::fs::create_dir_all(&self.path).map_err(|error| Self::unwritable(&path, &error))?;
        std::fs::write(&path, content.text.as_bytes())
            .map_err(|error| Self::unwritable(&path, &error))
    }

    pub fn disclose_kept(&self, key: &str) -> Result<Option<Content>> {
        let path = self.file(key)?;
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(Content::new(String::from_utf8_lossy(&bytes)))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(Failure::new(
                Category::RecordContentUnreadable,
                Attribution::Machine,
                Disposition::Refused,
                WHERE,
                "content is filed here and would not be read",
            )
            .with_context("path", path.display().to_string())
            .with_context("os_error", error.to_string())),
        }
    }

    fn file(&self, key: &str) -> Result<PathBuf> {
        let usable = !key.is_empty()
            && key.len() <= 160
            && key
                .chars()
                .all(|held| held.is_ascii_alphanumeric() || matches!(held, '-' | '_' | '.'))
            && !key.starts_with('.');
        if !usable {
            return Err(Failure::new(
                Category::InternalInvariantViolated,
                Attribution::Mcf,
                Disposition::Refused,
                WHERE,
                "a content key must be a plain name: letters, digits, dash, underscore and dot",
            )
            .with_context("key_bytes", key.len().to_string()));
        }
        Ok(self.path.join(key))
    }

    fn unwritable(path: &Path, error: &std::io::Error) -> Failure {
        Failure::new(
            Category::RecordUnwritable,
            Attribution::Machine,
            Disposition::Refused,
            WHERE,
            "content could not be written to the content store",
        )
        .with_context("path", path.display().to_string())
        .with_context("os_error", error.to_string())
    }

    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        crate::journal::default_path().map(|record| Self::beside(&record))
    }

    #[must_use]
    pub fn beside(record: &Path) -> PathBuf {
        Self::beside_for(record, Whose::User)
    }

    #[must_use]
    pub fn beside_for(record: &Path, whose: Whose) -> PathBuf {
        record.parent().map_or_else(
            || PathBuf::from(whose.directory()),
            |dir| dir.join(whose.directory()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Content, ContentStore, Whose};

    #[test]
    fn content_does_not_disclose_itself_in_a_debug_rendering() {
        let secret = Content::new("the operator's private prompt");
        let rendered = format!("{secret:?}");
        assert!(!rendered.contains("private"), "{rendered}");
        assert!(rendered.contains("undisclosed"), "{rendered}");
        assert!(rendered.contains("29 bytes"), "{rendered}");
    }

    #[test]
    fn disclosure_returns_what_was_held() {
        let content = Content::new("hello");
        assert_eq!(content.disclose(), "hello");
        assert_eq!(content.length_bytes(), 5);
    }

    #[test]
    fn length_is_available_without_disclosure() {
        let content = Content::new("a".repeat(4096));
        assert_eq!(content.length_bytes(), 4096);
    }

    #[test]
    fn the_content_store_is_not_the_record() {
        let (Some(record), Some(content)) =
            (crate::journal::default_path(), ContentStore::default_path())
        else {
            return;
        };
        assert_ne!(record, content);
        assert!(!content.ends_with("record.jsonl"));
    }

    #[test]
    fn a_persons_text_and_mcfs_own_do_not_share_a_directory() {
        let record = std::path::Path::new("/somewhere/record.jsonl");
        let user = ContentStore::beside_for(record, Whose::User);
        let fixture = ContentStore::beside_for(record, Whose::Fixture);
        assert_ne!(user, fixture);
        assert!(user.ends_with("content"));
        assert!(fixture.ends_with("fixtures"));
    }

    #[test]
    fn a_store_knows_whose_text_it_holds() {
        let root = std::env::temp_dir().join(format!("mcf-whose-{}", std::process::id()));
        let _cleared = std::fs::remove_dir_all(&root);
        let user =
            ContentStore::open_for(&root.join("content"), Whose::User).expect("a store opens");
        let fixture =
            ContentStore::open_for(&root.join("fixtures"), Whose::Fixture).expect("a store opens");
        assert_eq!(user.whose(), Whose::User);
        assert_eq!(fixture.whose(), Whose::Fixture);

        user.keep("k", &Content::new("what a person typed"))
            .expect("kept");
        fixture
            .keep("k", &Content::new("what MCF asked"))
            .expect("kept");
        assert_eq!(
            user.disclose_kept("k")
                .expect("read")
                .expect("there")
                .disclose(),
            "what a person typed"
        );
        assert_eq!(
            fixture
                .disclose_kept("k")
                .expect("read")
                .expect("there")
                .disclose(),
            "what MCF asked"
        );
        let _removed = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_unknown_category_is_not_guessed() {
        assert_eq!(Whose::parse("user"), Some(Whose::User));
        assert_eq!(Whose::parse("fixture"), Some(Whose::Fixture));
        assert_eq!(Whose::parse("suite"), None);
        assert_eq!(Whose::parse(""), None);
    }

    #[test]
    fn a_store_can_be_opened_where_it_is_told_to_be() {
        let path = std::env::temp_dir().join(format!("mcf-content-{}", std::process::id()));
        let store = ContentStore::open(&path.join("content")).expect("a temporary path opens");
        assert!(store.path().ends_with("content"));
        let _removed = std::fs::remove_dir_all(&path);
    }
}
