use std::mem::size_of;

use fromsoftware_shared::game_version::{GameVersion, LANG_ID_EN};
use pelite::pe64::PeView;
use windows::Win32::System::LibraryLoader::GetModuleHandleA;

pub const PROFILE: Er117Profile = Er117Profile {
    name: "WW 2.7.0.0",
    selector_refresh_rva: 0x00AF_EEA0,
    appearance_resolver_rva: 0x00D4_23B0,
    game_data_man_global_rva: 0x03D6_1F98,
    game_selector_offset: 0x960,
    local_resolver_return_rva: 0x0050_D740,
    remote_resolver_return_rva: 0x0040_43EF,
    resolver_call_rvas: [0x0040_34AD, 0x0040_43EA, 0x0050_D73B],
    official_table_rva: 0x02BB_5028,
};

#[derive(Clone, Copy, Debug)]
pub struct Er117Profile {
    pub name: &'static str,
    pub selector_refresh_rva: usize,
    pub appearance_resolver_rva: usize,
    pub game_data_man_global_rva: usize,
    pub game_selector_offset: usize,
    pub local_resolver_return_rva: usize,
    pub remote_resolver_return_rva: usize,
    pub resolver_call_rvas: [usize; 3],
    pub official_table_rva: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SupportedExe {
    Ww270,
}

impl GameVersion for SupportedExe {
    const NAME: &'static str = "elden ring";

    fn from_lang_version(lang_id: u16, version: &str) -> Option<Self> {
        match (lang_id, version) {
            (LANG_ID_EN, "2.7.0.0") => Some(Self::Ww270),
            _ => None,
        }
    }
}

pub fn detect() -> Result<(usize, &'static Er117Profile), String> {
    let module = unsafe { GetModuleHandleA(None) }
        .map_err(|error| format!("GetModuleHandleA(None) failed: {error:?}"))?;
    let base = module.0 as usize;
    let image = unsafe { PeView::module(base as *const u8) };
    SupportedExe::detect(&image)
        .map_err(|error| format!("unsupported Elden Ring executable: {error}"))?;
    validate(base)?;
    Ok((base, &PROFILE))
}

pub fn validate(base: usize) -> Result<(), String> {
    let selector_signature: &[Option<u8>] = &[
        Some(0x4C),
        Some(0x8B),
        Some(0xDC),
        Some(0x55),
        Some(0x48),
        Some(0x83),
        Some(0xEC),
        Some(0x70),
        Some(0x48),
        Some(0x8B),
        Some(0x05),
        None,
        None,
        None,
        None,
        Some(0x48),
        Some(0x33),
        Some(0xC4),
    ];
    let resolver_signature: &[Option<u8>] = &[
        Some(0x80),
        Some(0xF9),
        Some(0x04),
        Some(0x73),
        Some(0x0E),
        Some(0x0F),
        Some(0xB6),
        Some(0xC1),
        Some(0x48),
        Some(0x8D),
        Some(0x0D),
        None,
        None,
        None,
        None,
        Some(0x8B),
        Some(0x04),
        Some(0x81),
        Some(0xC3),
        Some(0x83),
        Some(0xC8),
        Some(0xFF),
        Some(0xC3),
    ];
    let pack_signature: &[Option<u8>] = &[
        Some(0x0F),
        Some(0xB6),
        Some(0x8F),
        Some(0x60),
        Some(0x09),
        Some(0x00),
        Some(0x00),
        Some(0x0F),
        Some(0xB7),
        Some(0x83),
        Some(0xB4),
        Some(0x00),
        Some(0x00),
        Some(0x00),
        Some(0x66),
        Some(0xC1),
        Some(0xE1),
        Some(0x0E),
        Some(0x66),
        Some(0x23),
        Some(0xC2),
        Some(0x66),
        Some(0x0B),
        Some(0xC8),
    ];
    let unpack_signature: &[Option<u8>] = &[
        Some(0x41),
        Some(0x0F),
        Some(0xB7),
        Some(0x9F),
        Some(0xC4),
        Some(0x00),
        Some(0x00),
        Some(0x00),
        Some(0x66),
        Some(0xC1),
        Some(0xEB),
        Some(0x0E),
        Some(0x80),
        Some(0xE3),
        Some(0x03),
    ];

    unsafe {
        match_pattern(base + PROFILE.selector_refresh_rva, selector_signature)
            .then_some(())
            .ok_or_else(|| "selector refresh signature mismatch at RVA 0xAFEEA0".to_string())?;
        match_pattern(base + PROFILE.appearance_resolver_rva, resolver_signature)
            .then_some(())
            .ok_or_else(|| "appearance resolver signature mismatch at RVA 0xD423B0".to_string())?;
        match_pattern(base + 0x0025_F898, pack_signature)
            .then_some(())
            .ok_or_else(|| "two-bit selector pack signature mismatch".to_string())?;
        match_pattern(base + 0x0040_4361, unpack_signature)
            .then_some(())
            .ok_or_else(|| "two-bit selector unpack signature mismatch".to_string())?;

        for call_rva in PROFILE.resolver_call_rvas {
            let target = direct_call_target(base, call_rva)
                .ok_or_else(|| format!("resolver call is not E8 rel32 at RVA 0x{call_rva:X}"))?;
            if target != base + PROFILE.appearance_resolver_rva {
                return Err(format!(
                    "resolver call target mismatch at RVA 0x{call_rva:X}: 0x{target:X}"
                ));
            }
        }

        let table = std::slice::from_raw_parts(
            (base + PROFILE.official_table_rva) as *const i32,
            crate::catalog::OFFICIAL_APPEARANCE_IDS.len(),
        );
        if table != crate::catalog::OFFICIAL_APPEARANCE_IDS {
            return Err(format!("official appearance table mismatch: {table:?}"));
        }
    }
    Ok(())
}

unsafe fn match_pattern(address: usize, pattern: &[Option<u8>]) -> bool {
    let bytes = unsafe { std::slice::from_raw_parts(address as *const u8, pattern.len()) };
    bytes
        .iter()
        .zip(pattern)
        .all(|(actual, expected)| expected.is_none_or(|expected| *actual == expected))
}

unsafe fn direct_call_target(base: usize, rva: usize) -> Option<usize> {
    let address = base.checked_add(rva)?;
    if unsafe { *(address as *const u8) } != 0xE8 {
        return None;
    }
    let displacement = unsafe { *((address + 1) as *const i32) } as isize;
    (address + 5).checked_add_signed(displacement)
}

const _: () = assert!(size_of::<i32>() == 4);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_return_points_follow_the_three_known_calls() {
        assert_eq!(
            PROFILE.resolver_call_rvas[1] + 5,
            PROFILE.remote_resolver_return_rva
        );
        assert_eq!(
            PROFILE.resolver_call_rvas[2] + 5,
            PROFILE.local_resolver_return_rva
        );
    }
}
