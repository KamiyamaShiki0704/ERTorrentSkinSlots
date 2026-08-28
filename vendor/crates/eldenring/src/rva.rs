use pelite::pe64::PeView;
use std::sync::LazyLock;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::core::PCSTR;

mod bundle;
mod rva_jp;
mod rva_ww;
mod rva_ww_270;

pub use bundle::RvaBundle;

use fromsoftware_shared::game_version::{GameVersion, LANG_ID_EN, LANG_ID_JP};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ERGameVersion {
    Ww2700,
    Ww262,
    Jp2621,
}

impl GameVersion for ERGameVersion {
    const NAME: &'static str = "elden ring";

    fn from_lang_version(lang_id: u16, version: &str) -> Option<Self> {
        match (lang_id, version) {
            (LANG_ID_EN, "2.7.0.0") => Some(Self::Ww2700),
            (LANG_ID_EN, "2.6.2.0") => Some(Self::Ww262),
            (LANG_ID_JP, "2.6.2.1") => Some(Self::Jp2621),
            _ => None,
        }
    }
}

impl ERGameVersion {
    const fn rvas(self) -> RvaBundle {
        match self {
            Self::Ww2700 => rva_ww_270::RVAS,
            Self::Ww262 => rva_ww::RVAS,
            Self::Jp2621 => rva_jp::RVAS,
        }
    }
}

/// Returns the RVA bundle for the current executable region and version.
///
/// This will panic if the current executable isn't supported by this package.
pub fn get() -> &'static RvaBundle {
    static RVAS: LazyLock<RvaBundle> = LazyLock::new(|| {
        let module = unsafe {
            PeView::module(GetModuleHandleA(PCSTR(std::ptr::null())).unwrap().0 as *const u8)
        };
        ERGameVersion::detect(&module)
            .unwrap_or_else(|e| panic!("{e}"))
            .rvas()
    });

    &RVAS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_ww_270_for_runtime_rvas() {
        let version = ERGameVersion::from_lang_version(LANG_ID_EN, "2.7.0.0")
            .expect("WW 2.7.0.0 should have task-runtime RVAs");
        let rvas = version.rvas();

        assert_eq!(rvas.global_hinstance, 0x03D89708);
        assert_eq!(rvas.register_task, 0x00EB3DE0);
    }
}
