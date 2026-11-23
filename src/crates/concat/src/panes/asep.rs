// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 ASEP AutoVid contributors

//! The library's ASEP page: the user's own asset folder.
//!
//! The folder is `Documents\ASEP-AutoVid-Assets`, outside the project and the
//! app's data (see `concat_host::assets`). A file comes in by the page's
//! Import button, or by being dropped on the window while the page is up. It
//! is copied to the sub-folder of its kind on a worker, and then handed on to
//! what already reads that kind: a LUT is imported as a look, a font is
//! registered, a sound joins the sound library, and a text preset, an effect
//! package and a template are copied into the app's own folders for them.
//! Pictures and videos stay in the library, for the timeline to take from.

use std::path::{Path, PathBuf};

use concat_host::assets::{self, AssetEntry, AssetKind};

use crate::i18n::tf;
use crate::panes::Msg;
use crate::panes::sounds::SoundsMsg;
use crate::studio::Studio;
use crate::ui::AssetCardData;

/// Why a file did not come in.
#[derive(Clone, Debug)]
pub enum Refusal {
    /// It is not something any folder of the library takes.
    Unsupported(PathBuf),
    /// It is, and copying it failed.
    Failed(String),
}

/// Everything that can happen to the ASEP page.
#[derive(Clone, Debug)]
pub enum AsepMsg {
    /// A library pane turned to the page or away from it.
    Shown(bool),
    /// A category picked: its place among [`AssetKind::ALL`].
    Category(i32),
    /// The folder looked at again.
    Refresh,
    /// The category's folder shown in the system's file manager.
    OpenFolder,
    /// A row used: a LUT is applied to the selected clip, and a sticker or
    /// an overlay is laid at the playhead. The row's place in the list.
    Add(i32),
    /// Files picked, or dropped while the page was up.
    Import(Vec<PathBuf>),
    /// The copying worker is done.
    Imported(Vec<Result<(AssetKind, PathBuf), Refusal>>),
}

/// The page's state.
#[derive(Default)]
pub struct AsepPane {
    /// A library is showing the page, so files dropped on the window go
    /// into the library rather than the bin.
    pub shown: bool,
    /// The category shown: its place among [`AssetKind::ALL`].
    pub category: usize,
    /// A copy running.
    pub importing: bool,
    /// What the category's folder held when it was last read.
    entries: Vec<AssetEntry>,
}

/// The Documents folder the system names, which only a desktop can ask for.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn documents() -> Option<PathBuf> {
    dirs::document_dir()
}

/// A phone keeps no library of this kind.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn documents() -> Option<PathBuf> {
    None
}

impl AsepPane {
    /// The library's folder: the one chosen in the preferences, or else
    /// `Documents\ASEP-AutoVid-Assets`.
    pub fn root(studio: &Studio) -> PathBuf {
        studio.prefs.assets_root(documents())
    }

    /// The kind of the category shown.
    pub fn kind(&self) -> AssetKind {
        AssetKind::ALL[self.category.min(AssetKind::ALL.len() - 1)]
    }

    /// What the Import button's picker offers: the category's own
    /// extensions, or, for a category made of folders, a folder.
    pub fn picker(&self) -> Option<&'static [&'static str]> {
        let extensions = self.kind().extensions();
        (!extensions.is_empty()).then_some(extensions)
    }

    /// The asset in row `index` of the category shown.
    pub fn entry(&self, index: usize) -> Option<&AssetEntry> {
        self.entries.get(index)
    }

    /// Makes the library's folders if they are not there and reads the
    /// category's one.
    fn refresh(&mut self, studio: &Studio) {
        let root = Self::root(studio);
        if let Err(error) = assets::ensure_tree(&root) {
            log::warn!("assets: could not make the library folder: {error}");
        }
        self.entries = assets::scan(&root, self.kind());
    }

    /// [`Self::refresh`], and every LUT the folder holds that is not a look
    /// yet made one: a table put in the folder by hand is usable the moment
    /// the page shows it, and a drag of its row names a look that exists.
    fn reload(&mut self, studio: &mut Studio) {
        self.refresh(studio);
        if self.kind() != AssetKind::Lut {
            return;
        }
        let looks = Studio::looks_dir(&studio.host.dirs);
        let missing: Vec<PathBuf> = self
            .entries
            .iter()
            .filter(|entry| {
                let id = concat_effects::looks::look_id(&entry.path);
                !looks.join(id).join("effect.toml").is_file()
            })
            .map(|entry| entry.path.clone())
            .collect();
        if !missing.is_empty() {
            studio.import_luts(missing);
        }
    }

    /// Applies one message. The studio is the rest of the window; while
    /// this runs the studio's copy of the pane is a blank it must not read.
    pub fn update(&mut self, msg: AsepMsg, studio: &mut Studio) {
        match msg {
            AsepMsg::Shown(on) => {
                self.shown = on;
                if on {
                    self.reload(studio);
                }
            }
            AsepMsg::Category(index) => {
                self.category = (index.max(0) as usize).min(AssetKind::ALL.len() - 1);
                self.reload(studio);
            }
            AsepMsg::Refresh => self.reload(studio),
            AsepMsg::Add(index) => {
                let Some(entry) = self.entries.get(index.max(0) as usize).cloned() else {
                    return;
                };
                match entry.kind {
                    AssetKind::Lut => {
                        // On the selected clip, or as a layer of its own at
                        // the playhead when nothing is selected.
                        let id = concat_effects::looks::look_id(&entry.path);
                        studio.apply_lut(&id, &entry.name);
                    }
                    AssetKind::Sticker | AssetKind::Overlay => {
                        let start = f64::from(studio.playhead.max(0.0));
                        studio.place_asset(&entry.path, &entry.name, None, start);
                    }
                    _ => {}
                }
            }
            AsepMsg::OpenFolder => {
                let root = Self::root(studio);
                if let Err(error) = assets::ensure_tree(&root) {
                    studio.notify(&error, true);
                    return;
                }
                let folder = root.join(self.kind().folder());
                if let Err(error) = crate::platform::open_link(&folder.to_string_lossy()) {
                    studio.notify(&error, true);
                }
            }
            AsepMsg::Import(paths) => {
                if paths.is_empty() {
                    return;
                }
                let root = Self::root(studio);
                let chosen = self.kind();
                self.importing = true;
                crate::host::spawn_long(
                    move || {
                        paths
                            .iter()
                            .map(|path| {
                                // The page's own category first; a file that
                                // is not one of its, from a drop or a pick of
                                // everything, goes where its extension says.
                                let kind = if chosen.accepts(path) {
                                    Some(chosen)
                                } else {
                                    assets::guess(path)
                                };
                                match kind {
                                    Some(kind) => assets::import(&root, kind, path)
                                        .map(|copy| (kind, copy))
                                        .map_err(Refusal::Failed),
                                    None => Err(Refusal::Unsupported(path.clone())),
                                }
                            })
                            .collect::<Vec<_>>()
                    },
                    |studio, _, _, results| {
                        studio.handle(Msg::Asep(AsepMsg::Imported(results)));
                    },
                );
            }
            AsepMsg::Imported(results) => {
                self.importing = false;
                let mut copied: Vec<(AssetKind, PathBuf)> = Vec::new();
                for result in results {
                    match result {
                        Ok(done) => copied.push(done),
                        Err(Refusal::Unsupported(path)) => studio.notify(
                            &tf("asepLibrary.notSupported", &[&display_name(&path)]),
                            true,
                        ),
                        Err(Refusal::Failed(error)) => studio.notify(&error, true),
                    }
                }
                self.refresh(studio);
                self.hand_on(&copied, studio);
            }
        }
    }

    /// Gives what was copied in to whatever reads its kind, and says how
    /// many arrived where nothing else says so.
    fn hand_on(&self, copied: &[(AssetKind, PathBuf)], studio: &mut Studio) {
        let of = |wanted: AssetKind| -> Vec<PathBuf> {
            copied
                .iter()
                .filter(|(kind, _)| *kind == wanted)
                .map(|(_, path)| path.clone())
                .collect()
        };

        // A LUT becomes a look on the Adjustments page, and says so itself.
        let luts = of(AssetKind::Lut);
        // A font is registered, and says so itself.
        let fonts = of(AssetKind::Font);
        // A sound joins the sound library, and says so itself.
        let sounds: Vec<PathBuf> = of(AssetKind::Sfx)
            .into_iter()
            .chain(of(AssetKind::Music))
            .collect();
        let told = luts.len() + fonts.len() + sounds.len();

        if !luts.is_empty() {
            studio.import_luts(luts);
        }
        if !fonts.is_empty() {
            studio.import_fonts(fonts);
        }
        if !sounds.is_empty() {
            studio.handle(Msg::Sounds(SoundsMsg::Import(sounds)));
        }

        // The rest are the app's folders' to read, and are copied there.
        let dirs = studio.host.dirs.clone();
        let into = [
            (AssetKind::TextPreset, crate::presets::dir(&dirs)),
            (AssetKind::EffectPackage, Studio::looks_dir(&dirs)),
            (
                AssetKind::Template,
                concat_host::templates::templates_dir(&dirs.config),
            ),
        ];
        for (kind, folder) in into {
            for path in of(kind) {
                if let Err(error) = assets::copy_into(&folder, &path) {
                    studio.notify(&error, true);
                }
            }
        }

        // Pictures and videos have nowhere to go yet but the library.
        let quiet = copied.len().saturating_sub(told);
        if quiet > 0 {
            studio.notify(&tf("asepLibrary.added", &[&quiet]), false);
        }
    }

    /// The category's rows, for the page.
    pub fn cards(&self) -> Vec<AssetCardData> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| AssetCardData {
                id: entry.path.to_string_lossy().as_ref().into(),
                name: entry.name.as_str().into(),
                detail: detail(entry).into(),
                payload: payload(index, entry).into(),
            })
            .collect()
    }
}

/// What a drag of the row carries, or nothing for a row that is not laid on
/// the timeline. A sticker or an overlay is named by its place in the list,
/// since a path has a colon in it on Windows and the payload is cut at
/// colons; a LUT rides as the look it was made into, the way a look dragged
/// from the Filters page does.
fn payload(index: usize, entry: &AssetEntry) -> String {
    match entry.kind {
        AssetKind::Sticker | AssetKind::Overlay => format!("asep:{index}:{}", entry.name),
        AssetKind::Lut => format!(
            "filter:{}:{}",
            concat_effects::looks::look_id(&entry.path),
            entry.name
        ),
        _ => String::new(),
    }
}

/// A row's second line: the file's type, or nothing for a folder.
fn detail(entry: &AssetEntry) -> String {
    if entry.kind.is_bundle() {
        return String::new();
    }
    entry
        .path
        .extension()
        .map(|extension| extension.to_string_lossy().to_uppercase())
        .unwrap_or_default()
}

/// What a notice calls a file: its name, not its whole path.
fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: AssetKind, name: &str, path: &str) -> AssetEntry {
        AssetEntry {
            kind,
            name: name.to_owned(),
            path: PathBuf::from(path),
        }
    }

    /// A row drags as what it is: a sticker or an overlay by its place in
    /// the list - never by its path, which has a colon in it on Windows and
    /// the payload is cut at colons - a LUT as the look it was made into, and
    /// the rest not at all.
    #[test]
    fn a_row_drags_as_what_it_is() {
        assert_eq!(
            payload(
                2,
                &entry(AssetKind::Sticker, "Heart", "/lib/stickers/Heart.png")
            ),
            "asep:2:Heart"
        );
        assert_eq!(
            payload(
                0,
                &entry(AssetKind::Overlay, "Sparks", "/lib/overlays/Sparks.webm")
            ),
            "asep:0:Sparks"
        );
        assert_eq!(
            payload(
                5,
                &entry(AssetKind::Lut, "Warm Film", "/lib/luts/Warm Film.cube")
            ),
            "filter:user.warm-film:Warm Film"
        );
        assert_eq!(
            payload(1, &entry(AssetKind::Font, "Face", "/lib/fonts/Face.ttf")),
            ""
        );
        assert_eq!(
            payload(1, &entry(AssetKind::Sfx, "Hit", "/lib/sfx/Hit.wav")),
            ""
        );
    }

    /// A payload names a row and its label, so a name with a colon in it
    /// still reads back whole: the payload is cut into three at most.
    #[test]
    fn a_name_with_a_colon_is_the_label_not_the_row() {
        let text = payload(
            4,
            &entry(AssetKind::Sticker, "Take: 2", "/lib/stickers/Take 2.png"),
        );
        let mut fields = text.splitn(3, ':');
        assert_eq!(fields.next(), Some("asep"));
        assert_eq!(fields.next(), Some("4"));
        assert_eq!(fields.next(), Some("Take: 2"));
    }

    #[test]
    fn a_folder_row_has_no_type_and_a_file_row_has_its_extension() {
        assert_eq!(
            detail(&entry(AssetKind::Lut, "Warm", "/lib/luts/Warm.cube")),
            "CUBE"
        );
        assert_eq!(
            detail(&entry(
                AssetKind::EffectPackage,
                "glow",
                "/lib/effects/glow"
            )),
            ""
        );
    }
}
