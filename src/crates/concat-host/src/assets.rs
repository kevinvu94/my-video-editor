// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 ASEP AutoVid contributors

//! The ASEP asset library: an ordinary folder of the user's own files.
//!
//! `Documents\ASEP-AutoVid-Assets\` holds one folder per kind of asset
//! ([`AssetKind`]). It lives outside the repository and outside the app's
//! own data, so it is the user's to fill, back up and sync, and nothing in
//! it is ever committed.
//!
//! This module only knows about the folder: where it is, what goes in which
//! sub-folder, what is in it, and how a file is copied in. What an asset
//! *does* once it is in - a LUT becomes a look, a font becomes a face, a
//! sound joins the sound library - is the window's business, and it reuses
//! the importers that already do each of those.
//!
//! Importing copies, never moves or links: the library is the one place the
//! files are kept, and a project that uses one does not break when the
//! original is moved or deleted.

use std::path::{Path, PathBuf};

/// The library's folder name, under the user's Documents.
pub const FOLDER: &str = "ASEP-AutoVid-Assets";

/// How deep a bundle's folders may go before the copy gives up, so a folder
/// that loops back on itself cannot copy forever.
const MAX_DEPTH: usize = 12;

/// One kind of asset, which is also one sub-folder of the library.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AssetKind {
    /// A `.cube` colour table, used as a filter on a clip.
    Lut,
    /// A picture or animation placed over the footage as a sticker.
    Sticker,
    /// A picture or a video laid over the footage on a track above it; the
    /// clip's blend mode lets a black-background effect drop its black.
    Overlay,
    /// A font file for the Text page.
    Font,
    /// A short sound effect.
    Sfx,
    /// Music for under a whole edit.
    Music,
    /// A text style: a `preset.toml` or a `.toml` file.
    TextPreset,
    /// An effect, filter, transition or audio-effect package: a folder with
    /// an `effect.toml` in it.
    EffectPackage,
    /// A project template: a folder with a `template.json` in it.
    Template,
}

impl AssetKind {
    /// Every kind, in the order the library lists them.
    pub const ALL: [AssetKind; 9] = [
        AssetKind::Lut,
        AssetKind::Sticker,
        AssetKind::Overlay,
        AssetKind::Font,
        AssetKind::Sfx,
        AssetKind::Music,
        AssetKind::TextPreset,
        AssetKind::EffectPackage,
        AssetKind::Template,
    ];

    /// The sub-folder this kind lives in.
    pub fn folder(self) -> &'static str {
        match self {
            AssetKind::Lut => "luts",
            AssetKind::Sticker => "stickers",
            AssetKind::Overlay => "overlays",
            AssetKind::Font => "fonts",
            AssetKind::Sfx => "sfx",
            AssetKind::Music => "music",
            AssetKind::TextPreset => "text-presets",
            AssetKind::EffectPackage => "effects",
            AssetKind::Template => "templates",
        }
    }

    /// The kind whose sub-folder is called `name`.
    pub fn from_folder(name: &str) -> Option<AssetKind> {
        AssetKind::ALL
            .into_iter()
            .find(|kind| kind.folder().eq_ignore_ascii_case(name))
    }

    /// The extensions of the single files this kind is made of. Empty for a
    /// bundle, which is a folder.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            AssetKind::Lut => &["cube"],
            AssetKind::Sticker => &["png", "webp", "gif"],
            AssetKind::Overlay => &["png", "webp", "gif", "mp4", "webm", "mov"],
            AssetKind::Font => &["ttf", "otf", "ttc"],
            AssetKind::Sfx | AssetKind::Music => &crate::sounds::EXTENSIONS,
            AssetKind::TextPreset => &["toml"],
            AssetKind::EffectPackage | AssetKind::Template => &[],
        }
    }

    /// For a bundle, the file that makes a folder one of this kind.
    pub fn marker(self) -> Option<&'static str> {
        match self {
            AssetKind::EffectPackage => Some("effect.toml"),
            AssetKind::Template => Some("template.json"),
            _ => None,
        }
    }

    /// Whether this kind is a folder rather than a file.
    pub fn is_bundle(self) -> bool {
        self.marker().is_some()
    }

    /// Whether `path` is something this kind takes: a file with one of its
    /// extensions, or, for a bundle, a folder that holds its marker.
    pub fn accepts(self, path: &Path) -> bool {
        match self.marker() {
            Some(marker) => path.is_dir() && path.join(marker).is_file(),
            None => path.is_file() && has_extension(path, self.extensions()),
        }
    }
}

/// Whether `path`'s extension is one of `extensions`, in any case.
fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|known| known.eq_ignore_ascii_case(extension))
        })
}

/// The kind a dropped file most likely is, when nothing says which page it
/// was dropped on. A picture is a sticker and a video an overlay; the two
/// share formats with each other and with nothing else, so a PNG meant as an
/// overlay is imported from the Overlays page, where the kind is given.
pub fn guess(path: &Path) -> Option<AssetKind> {
    for kind in AssetKind::ALL {
        if kind.is_bundle() && kind.accepts(path) {
            return Some(kind);
        }
    }
    let by_extension = [
        AssetKind::Lut,
        AssetKind::Font,
        AssetKind::Sfx,
        AssetKind::TextPreset,
        AssetKind::Sticker,
        AssetKind::Overlay,
    ];
    by_extension.into_iter().find(|kind| kind.accepts(path))
}

/// The library folder: `Documents\ASEP-AutoVid-Assets`. `documents` is the
/// folder the system names, which OneDrive often moves, and `home` is the
/// fallback when the system names none. The caller supplies both because the
/// crate that can ask the system is the window's.
pub fn root_in(documents: Option<PathBuf>, home: Option<PathBuf>) -> PathBuf {
    let documents = documents
        .filter(|path| !path.as_os_str().is_empty())
        .or_else(|| {
            home.filter(|path| !path.as_os_str().is_empty())
                .map(|home| home.join("Documents"))
        })
        .unwrap_or_else(|| PathBuf::from("Documents"));
    documents.join(FOLDER)
}

/// Makes the library folder and every sub-folder, leaving what is already
/// there alone.
pub fn ensure_tree(root: &Path) -> Result<(), String> {
    for kind in AssetKind::ALL {
        let dir = root.join(kind.folder());
        std::fs::create_dir_all(&dir)
            .map_err(|error| format!("could not create {}: {error}", dir.display()))?;
    }
    Ok(())
}

/// One asset in the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetEntry {
    pub kind: AssetKind,
    /// What a card says: the file's name without its extension, or the
    /// bundle's folder name.
    pub name: String,
    /// The file or the bundle's folder.
    pub path: PathBuf,
}

/// What is in `root`'s sub-folder for `kind`, by name. A missing folder is an
/// empty one; anything that is not something the kind takes is left out.
pub fn scan(root: &Path, kind: AssetKind) -> Vec<AssetEntry> {
    let Ok(entries) = std::fs::read_dir(root.join(kind.folder())) else {
        return Vec::new();
    };
    let mut found: Vec<AssetEntry> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| kind.accepts(path))
        .map(|path| AssetEntry {
            kind,
            name: card_name(&path, kind),
            path,
        })
        .collect();
    found.sort_by_key(|entry| entry.name.to_lowercase());
    found
}

fn card_name(path: &Path, kind: AssetKind) -> String {
    let name = if kind.is_bundle() {
        path.file_name()
    } else {
        path.file_stem()
    };
    name.map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Copies `source` into `root`'s sub-folder for `kind` and returns where the
/// copy is. A name the folder already has is numbered, never overwritten, and
/// a source already inside that sub-folder is returned as it is, since there
/// is nothing to copy.
pub fn import(root: &Path, kind: AssetKind, source: &Path) -> Result<PathBuf, String> {
    if !kind.accepts(source) {
        return Err(format!(
            "{} is not something the {} folder takes",
            source.display(),
            kind.folder()
        ));
    }
    let dir = root.join(kind.folder());
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("could not create {}: {error}", dir.display()))?;
    if source.parent() == Some(dir.as_path()) {
        return Ok(source.to_path_buf());
    }
    copy_into(&dir, source)
}

/// Copies a file, or a folder and what is in it, into `dir`, and returns
/// where the copy is. A name `dir` already has is numbered, never
/// overwritten. This is how a library asset is handed on to one of the app's
/// own folders - a look's, the text presets', the templates' - which read
/// from there and not from the library.
pub fn copy_into(dir: &Path, source: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir)
        .map_err(|error| format!("could not create {}: {error}", dir.display()))?;
    let name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = source.is_dir();
    let target = dir.join(free_name(dir, &name, folder));
    if folder {
        copy_folder(source, &target, 0)?;
    } else {
        std::fs::copy(source, &target)
            .map_err(|error| format!("could not copy {}: {error}", source.display()))?;
    }
    Ok(target)
}

/// `name` in `dir`, or `stem 2.ext` (`name 2` for a folder) and on, whichever
/// is free, with anything that could not be a plain file name taken out.
fn free_name(dir: &Path, name: &str, folder: bool) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let clean = clean.trim().trim_matches('.');
    let clean = if clean.is_empty() { "Asset" } else { clean };
    let (stem, extension) = if folder {
        (clean, "")
    } else {
        match clean.rsplit_once('.') {
            Some((stem, extension)) if !stem.is_empty() => (stem, extension),
            _ => (clean, ""),
        }
    };
    let named = |n: u32| {
        let stem = if n == 1 {
            stem.to_owned()
        } else {
            format!("{stem} {n}")
        };
        if extension.is_empty() {
            stem
        } else {
            format!("{stem}.{extension}")
        }
    };
    (1..)
        .map(named)
        .find(|candidate| !dir.join(candidate).exists())
        .unwrap_or_else(|| "Asset".to_owned())
}

/// Copies a folder and what is in it. A link is skipped, not followed: it
/// could point out of the folder, or back into it.
fn copy_folder(from: &Path, to: &Path, depth: usize) -> Result<(), String> {
    if depth > MAX_DEPTH {
        return Err(format!("{} is nested too deeply to copy", from.display()));
    }
    std::fs::create_dir_all(to)
        .map_err(|error| format!("could not create {}: {error}", to.display()))?;
    let entries = std::fs::read_dir(from)
        .map_err(|error| format!("could not read {}: {error}", from.display()))?;
    for entry in entries.filter_map(|entry| entry.ok()) {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let target = to.join(entry.file_name());
        if kind.is_symlink() {
            continue;
        } else if kind.is_dir() {
            copy_folder(&entry.path(), &target, depth + 1)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)
                .map_err(|error| format!("could not copy {}: {error}", entry.path().display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn every_kind_has_its_own_folder_name_and_finds_it_again() {
        let mut seen = std::collections::HashSet::new();
        for kind in AssetKind::ALL {
            assert!(seen.insert(kind.folder()), "{} twice", kind.folder());
            assert_eq!(AssetKind::from_folder(kind.folder()), Some(kind));
        }
        assert_eq!(AssetKind::from_folder("LUTS"), Some(AssetKind::Lut));
        assert_eq!(AssetKind::from_folder("nothing"), None);
    }

    #[test]
    fn the_library_sits_under_documents_or_else_under_home() {
        assert_eq!(
            root_in(Some(PathBuf::from("/od/Documents")), Some("/home".into())),
            Path::new("/od/Documents").join(FOLDER)
        );
        assert_eq!(
            root_in(None, Some("/home".into())),
            Path::new("/home").join("Documents").join(FOLDER)
        );
        assert_eq!(
            root_in(Some(PathBuf::new()), None),
            Path::new("Documents").join(FOLDER)
        );
    }

    #[test]
    fn ensure_tree_makes_every_folder_and_keeps_what_is_there() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(FOLDER);
        write(&root.join("luts").join("warm.cube"), "x");
        ensure_tree(&root).unwrap();
        for kind in AssetKind::ALL {
            assert!(root.join(kind.folder()).is_dir(), "{}", kind.folder());
        }
        assert!(root.join("luts").join("warm.cube").is_file());
        ensure_tree(&root).unwrap();
    }

    #[test]
    fn a_kind_takes_only_its_own_extensions_in_any_case() {
        let dir = tempfile::tempdir().unwrap();
        let file = |name: &str| {
            let path = dir.path().join(name);
            write(&path, "x");
            path
        };
        assert!(AssetKind::Lut.accepts(&file("Warm.CUBE")));
        assert!(!AssetKind::Lut.accepts(&file("warm.png")));
        assert!(AssetKind::Overlay.accepts(&file("spark.webm")));
        assert!(!AssetKind::Sticker.accepts(&file("spark.webm")));
        assert!(AssetKind::Sticker.accepts(&file("heart.gif")));
        assert!(AssetKind::Sfx.accepts(&file("whoosh.wav")));
        assert!(AssetKind::Font.accepts(&file("face.otf")));
        assert!(!AssetKind::Lut.accepts(&dir.path().join("missing.cube")));
    }

    #[test]
    fn a_bundle_is_a_folder_with_its_marker() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("glow").join("effect.toml"), "x");
        write(&dir.path().join("plain").join("readme.txt"), "x");
        assert!(AssetKind::EffectPackage.accepts(&dir.path().join("glow")));
        assert!(!AssetKind::EffectPackage.accepts(&dir.path().join("plain")));
        assert!(!AssetKind::Template.accepts(&dir.path().join("glow")));
        assert_eq!(
            guess(&dir.path().join("glow")),
            Some(AssetKind::EffectPackage)
        );
    }

    #[test]
    fn a_dropped_file_is_guessed_by_its_extension() {
        let dir = tempfile::tempdir().unwrap();
        let file = |name: &str| {
            let path = dir.path().join(name);
            write(&path, "x");
            path
        };
        assert_eq!(guess(&file("a.cube")), Some(AssetKind::Lut));
        assert_eq!(guess(&file("a.ttf")), Some(AssetKind::Font));
        assert_eq!(guess(&file("a.mp3")), Some(AssetKind::Sfx));
        assert_eq!(guess(&file("a.png")), Some(AssetKind::Sticker));
        assert_eq!(guess(&file("a.mp4")), Some(AssetKind::Overlay));
        assert_eq!(guess(&file("a.txt")), None);
    }

    #[test]
    fn importing_copies_and_numbers_a_name_the_folder_has() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(FOLDER);
        let source = dir.path().join("warm.cube");
        write(&source, "one");
        let first = import(&root, AssetKind::Lut, &source).unwrap();
        let second = import(&root, AssetKind::Lut, &source).unwrap();
        assert_eq!(first, root.join("luts").join("warm.cube"));
        assert_eq!(second, root.join("luts").join("warm 2.cube"));
        assert!(source.is_file(), "the original stays where it was");
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "one");
    }

    #[test]
    fn importing_refuses_a_file_of_the_wrong_kind_and_copies_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(FOLDER);
        let source = dir.path().join("notes.txt");
        write(&source, "x");
        assert!(import(&root, AssetKind::Lut, &source).is_err());
        assert!(!root.join("luts").join("notes.txt").exists());
    }

    #[test]
    fn a_file_already_in_its_folder_is_not_copied_onto_itself() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(FOLDER);
        let inside = root.join("sfx").join("hit.wav");
        write(&inside, "x");
        assert_eq!(import(&root, AssetKind::Sfx, &inside).unwrap(), inside);
        assert!(!root.join("sfx").join("hit 2.wav").exists());
    }

    #[test]
    fn a_name_cannot_walk_out_of_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(free_name(dir.path(), "..", false), "Asset");
        assert_eq!(free_name(dir.path(), "a:b*c.png", false), "a_b_c.png");
        assert_eq!(free_name(dir.path(), "x/../y.png", false), "x_.._y.png");
        assert!(!free_name(dir.path(), "../../evil.cube", false).contains(['/', '\\']));
    }

    #[test]
    fn a_bundle_is_copied_whole_and_numbered_like_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(FOLDER);
        let source = dir.path().join("glow");
        write(&source.join("effect.toml"), "toml");
        write(&source.join("sub").join("effect.wgsl"), "wgsl");
        let first = import(&root, AssetKind::EffectPackage, &source).unwrap();
        let second = import(&root, AssetKind::EffectPackage, &source).unwrap();
        assert_eq!(first, root.join("effects").join("glow"));
        assert_eq!(second, root.join("effects").join("glow 2"));
        assert_eq!(
            std::fs::read_to_string(first.join("sub").join("effect.wgsl")).unwrap(),
            "wgsl"
        );
    }

    #[test]
    fn copy_into_hands_a_file_or_a_folder_to_another_folder() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("look.toml");
        write(&file, "one");
        let folder = dir.path().join("pack");
        write(&folder.join("a.txt"), "two");
        let target = dir.path().join("app").join("effects");
        assert_eq!(copy_into(&target, &file).unwrap(), target.join("look.toml"));
        assert_eq!(
            copy_into(&target, &file).unwrap(),
            target.join("look 2.toml")
        );
        assert_eq!(copy_into(&target, &folder).unwrap(), target.join("pack"));
        assert_eq!(
            std::fs::read_to_string(target.join("pack").join("a.txt")).unwrap(),
            "two"
        );
    }

    #[test]
    fn scan_lists_what_the_kind_takes_by_name_and_skips_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(FOLDER);
        write(&root.join("stickers").join("b.png"), "x");
        write(&root.join("stickers").join("A.gif"), "x");
        write(&root.join("stickers").join("notes.txt"), "x");
        let names: Vec<String> = scan(&root, AssetKind::Sticker)
            .into_iter()
            .map(|entry| entry.name)
            .collect();
        assert_eq!(names, ["A", "b"]);
        assert!(
            scan(&root, AssetKind::Lut).is_empty(),
            "a missing folder is empty"
        );
    }
}
