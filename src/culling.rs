//! Photographer selection model and non-destructive export, independent of the UI.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewFormat {
    #[default]
    Jpeg,
    Raw,
    All,
}
impl ViewFormat {
    pub fn accepts(self, path: &Path) -> bool {
        match self {
            Self::Jpeg => is_jpeg(path),
            Self::Raw => is_raw(path),
            Self::All => true,
        }
    }
}
pub fn is_jpeg(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("jpg") || s.eq_ignore_ascii_case("jpeg"))
}
pub fn is_raw(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(crate::raw_extensions::is_raw_extension)
}
/// Keep the directory and exact stem: never pair photos from different folders.
pub fn key(path: &Path) -> PathBuf {
    path.with_extension("")
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StarFilter {
    AtLeast(u8),
    Exactly(u8),
    AnyOf([bool; 6]),
}
impl Default for StarFilter {
    fn default() -> Self {
        Self::AtLeast(0)
    }
}
impl StarFilter {
    pub fn accepts(&self, stars: u8) -> bool {
        match self {
            Self::AtLeast(n) => stars >= *n,
            Self::Exactly(n) => stars == *n,
            Self::AnyOf(mask) => mask.get(stars as usize).copied().unwrap_or(false),
        }
    }
}
#[derive(Default)]
pub struct Selection {
    pub format: ViewFormat,
    pub filter: StarFilter,
    // Session ratings intentionally never modify image metadata.
    pub ratings: HashMap<PathBuf, u8>,
}
impl Selection {
    pub fn rating(&self, path: &Path) -> u8 {
        self.ratings.get(&key(path)).copied().unwrap_or(0)
    }
    pub fn rate(&mut self, path: &Path, stars: u8) {
        self.ratings.insert(key(path), stars.min(5));
    }
    pub fn accepts(&self, path: &Path) -> bool {
        self.format.accepts(path) && self.filter.accepts(self.rating(path))
    }
}
#[derive(Default, Debug)]
pub struct ExportReport {
    pub copied: usize,
    pub missing_pairs: usize,
    pub errors: Vec<String>,
    pub directories: Vec<PathBuf>,
}
/// Snapshot source paths before starting. The callback counts completed file attempts.
/// Each run reserves fresh directories so repeat exports never overwrite earlier runs.
pub fn export(
    selected: &[PathBuf],
    root: &Path,
    stamp: &str,
    paired: bool,
    mut progress: impl FnMut(usize, usize),
) -> io::Result<ExportReport> {
    if stamp.len() != 4 || !stamp.bytes().all(|b| b.is_ascii_digit()) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "expected HHMM"));
    }
    let mut report = ExportReport::default();
    let mut sources = Vec::new();
    let mut seen = HashSet::new();
    let mut indexes: HashMap<PathBuf, HashMap<PathBuf, Vec<PathBuf>>> = HashMap::new();
    for source in selected {
        if !is_jpeg(source) && !is_raw(source) {
            continue;
        }
        if seen.insert(source.clone()) {
            sources.push(source.clone());
        }
        if !paired {
            continue;
        }
        let parent = source.parent().unwrap_or(Path::new("."));
        if !indexes.contains_key(parent) {
            let mut index: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
            for entry in fs::read_dir(parent)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    let path = entry.path();
                    if is_jpeg(&path) || is_raw(&path) {
                        index.entry(key(&path)).or_default().push(path);
                    }
                }
            }
            indexes.insert(parent.to_path_buf(), index);
        }
        let matches: Vec<_> = indexes[parent]
            .get(&key(source))
            .into_iter()
            .flatten()
            .filter(|p| is_jpeg(p) != is_jpeg(source))
            .cloned()
            .collect();
        if matches.is_empty() {
            report.missing_pairs += 1;
        }
        for path in matches {
            if seen.insert(path.clone()) {
                sources.push(path);
            }
        }
    }
    sources.sort();
    if sources.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "no JPEG/RAW selected",
        ));
    }
    fs::create_dir_all(root)?;
    let mut targets = HashMap::new();
    for (prefix, required) in [
        ("jpg", sources.iter().any(|p| is_jpeg(p))),
        ("raw", sources.iter().any(|p| is_raw(p))),
    ] {
        if !required {
            continue;
        }
        let mut suffix = 0;
        loop {
            let name = if suffix == 0 {
                format!("{prefix}{stamp}")
            } else {
                format!("{prefix}{stamp}_{suffix}")
            };
            let path = root.join(name);
            match fs::create_dir(&path) {
                Ok(()) => {
                    report.directories.push(path.clone());
                    targets.insert(prefix, path);
                    break;
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => suffix += 1,
                Err(e) => return Err(e),
            }
        }
    }
    progress(0, sources.len());
    for (i, source) in sources.iter().enumerate() {
        let prefix = if is_jpeg(source) { "jpg" } else { "raw" };
        let dest = targets[prefix].join(source.file_name().unwrap());
        // create_new also detects duplicate basenames in recursive selections.
        let result = (|| -> io::Result<()> {
            let mut input = fs::File::open(source)?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dest)?;
            if let Err(e) = io::copy(&mut input, &mut output).and_then(|_| output.sync_all()) {
                drop(output);
                let _ = fs::remove_file(&dest);
                return Err(e);
            }
            Ok(())
        })();
        match result {
            Ok(()) => report.copied += 1,
            Err(e) => report.errors.push(format!("{}: {e}", source.display())),
        }
        progress(i + 1, sources.len());
    }
    Ok(report)
}
