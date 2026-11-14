//! Run without the viewer's native codec/GPU dependencies:
//! rustc --edition 2024 --test tests/culling_standalone.rs -o /tmp/culling-tests
#[path = "../src/culling.rs"]
mod culling;
#[path = "../src/raw_extensions.rs"]
mod raw_extensions;
use culling::*;
use std::{fs, path::PathBuf};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "siv-culling-{}-{}",
            std::process::id(),
            ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn file(&self, name: &str) -> PathBuf {
        let p = self.0.join(name);
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&p, name).unwrap();
        p
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn shared_rating_and_directory_isolation() {
    let mut s = Selection::default();
    s.rate("a/IMG.JPG".as_ref(), 4);
    assert_eq!(s.rating("a/IMG.CR3".as_ref()), 4);
    assert_eq!(s.rating("b/IMG.CR3".as_ref()), 0);
    assert_eq!(s.rating("a/img.CR3".as_ref()), 0);
    s.rate("a/IMG.CR3".as_ref(), 0);
    assert_eq!(s.rating("a/IMG.JPG".as_ref()), 0);
}
#[test]
fn filters_cover_all_ratings() {
    for n in 0..=5 {
        assert!(StarFilter::AtLeast(0).accepts(n));
        assert_eq!(StarFilter::AtLeast(4).accepts(n), n >= 4);
        assert_eq!(StarFilter::Exactly(4).accepts(n), n == 4);
        assert_eq!(
            StarFilter::AnyOf([true, false, false, false, true, false]).accepts(n),
            n == 0 || n == 4
        );
    }
    let mut s = Selection::default();
    assert!(s.accepts("x.JPEG".as_ref()));
    assert!(!s.accepts("x.arw".as_ref()));
    s.format = ViewFormat::Raw;
    assert!(s.accepts("x.ARW".as_ref()));
    assert!(!s.accepts("x.png".as_ref()));
    s.format = ViewFormat::All;
    assert!(s.accepts("x.png".as_ref()));
}
#[test]
fn paired_export_copies_all_raw_variants_once_and_reports_missing() {
    let t = Temp::new();
    let jpg = t.file("x.JPG");
    t.file("x.ARW");
    t.file("x.cr2");
    let missing = t.file("missing.jpg");
    t.file("elsewhere/missing.nef");
    let mut events = vec![];
    let r = export(&[jpg.clone(), jpg, missing], &t.0, "2145", true, |a, b| {
        events.push((a, b))
    })
    .unwrap();
    assert_eq!(r.copied, 4);
    assert_eq!(r.missing_pairs, 1);
    assert!(r.errors.is_empty());
    assert_eq!(events.last(), Some(&(4, 4)));
    assert_eq!(fs::read(t.0.join("raw2145/x.ARW")).unwrap(), b"x.ARW");
}
#[test]
fn reverse_pair_and_repeated_exports_never_overwrite() {
    let t = Temp::new();
    t.file("x.jpeg");
    let raw = t.file("x.NEF");
    let r = export(&[raw.clone()], &t.0, "0102", true, |_, _| {}).unwrap();
    assert_eq!(r.copied, 2);
    fs::write(t.0.join("raw0102/x.NEF"), "previous").unwrap();
    let r = export(&[raw], &t.0, "0102", false, |_, _| {}).unwrap();
    assert_eq!(r.copied, 1);
    assert!(t.0.join("raw0102_1/x.NEF").exists());
    assert_eq!(fs::read(t.0.join("raw0102/x.NEF")).unwrap(), b"previous");
}
#[test]
fn duplicate_basenames_and_missing_sources_are_reported() {
    let t = Temp::new();
    let a = t.file("a/x.jpg");
    let b = t.file("b/x.jpg");
    let gone = t.0.join("gone.jpg");
    let r = export(&[a, b, gone], &t.0, "1200", false, |_, _| {}).unwrap();
    assert_eq!(r.copied, 1);
    assert_eq!(r.errors.len(), 2);
    assert!(!t.0.join("jpg1200/gone.jpg").exists());
}
#[test]
fn invalid_or_empty_exports_do_not_create_directories() {
    let t = Temp::new();
    let root = t.0.join("out");
    assert!(export(&[], &root, "../x", false, |_, _| {}).is_err());
    assert!(export(&[], &root, "1200", false, |_, _| {}).is_err());
    assert!(!root.exists());
}
