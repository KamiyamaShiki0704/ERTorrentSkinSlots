//! Catalog-backed native mount recognition. The game derives a four-digit
//! character ID as (NpcParam / 10_000) % 10_000 (WW D42180).
use std::{
    mem::transmute,
    sync::atomic::{AtomicUsize, Ordering},
};

use ilhook::x64::{CallbackOption, ClosureHookPoint, HookFlags, Registers, hook_closure_retn};

use crate::{catalog::SlotCatalog, logging, profile::Er117Profile};

pub fn custom_character(catalog: &SlotCatalog, character: u32) -> bool {
    character != 0
        && catalog.slots().iter().any(|slot| {
            slot.selector >= 4 && character_from_appearance(slot.appearance_npc_param) == character
        })
}

pub fn custom_appearance(catalog: &SlotCatalog, appearance: i32) -> bool {
    catalog
        .slots()
        .iter()
        .any(|slot| slot.selector >= 4 && slot.appearance_npc_param == appearance)
}

fn character_from_appearance(appearance: i32) -> u32 {
    (appearance as u32 / 10_000) % 10_000
}

type CharacterPredicate = unsafe extern "C" fn(*const u32) -> u8;
type AppearancePredicate = unsafe extern "C" fn(*const u8) -> u8;

/// All hooks remain owned by the caller until the complete installation succeeds.
pub unsafe fn install(
    base: usize,
    profile: &Er117Profile,
    catalog: SlotCatalog,
) -> Result<Vec<ClosureHookPoint<'static>>, String> {
    let mut hooks = Vec::new();
    let characters = catalog.clone();
    let character_logs = AtomicUsize::new(0);
    hooks.push(
        unsafe {
            hook_closure_retn(
                base + profile.character_classifier_rva,
                move |registers: *mut Registers, original| {
                    let input = (*registers).rcx as *const u32;
                    if !input.is_null() {
                        let id = input.read_unaligned();
                        if custom_character(&characters, id) {
                            if character_logs.fetch_add(1, Ordering::Relaxed) < 8 {
                                logging::write(format!(
                                    "mount recognition character={id} accepted"
                                ));
                            }
                            return 1;
                        }
                    }
                    let original: CharacterPredicate = transmute(original);
                    usize::from(original(input))
                },
                CallbackOption::None,
                HookFlags::empty(),
            )
        }
        .map_err(|error| format!("character recognition hook failed: {error:?}"))?,
    );

    let appearance_logs = AtomicUsize::new(0);
    hooks.push(
        unsafe {
            hook_closure_retn(
                base + profile.appearance_classifier_rva,
                move |registers: *mut Registers, original| {
                    let entry = (*registers).rcx as *const u8;
                    // Match the native function's non-null Param entry prerequisite.
                    if !entry.is_null() && entry.cast::<usize>().read_unaligned() != 0 {
                        let id = entry.add(8).cast::<i32>().read_unaligned();
                        if custom_appearance(&catalog, id) {
                            if appearance_logs.fetch_add(1, Ordering::Relaxed) < 8 {
                                logging::write(format!("mount physics appearance={id} accepted"));
                            }
                            return 1;
                        }
                    }
                    let original: AppearancePredicate = transmute(original);
                    usize::from(original(entry))
                },
                CallbackOption::None,
                HookFlags::empty(),
            )
        }
        .map_err(|error| format!("appearance recognition hook failed: {error:?}"))?,
    );
    Ok(hooks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognition_is_catalog_scoped_and_never_matches_player_zero() {
        let catalog = SlotCatalog::default_er117();
        for id in [8006, 8007, 8008] {
            assert!(custom_character(&catalog, id));
        }
        for id in [0, 8001, 8009, 9999, 18_008] {
            assert!(!custom_character(&catalog, id));
        }
        assert!(custom_appearance(&catalog, 80_080_000));
        assert!(!custom_appearance(&catalog, 80_080_001));
        assert!(!custom_appearance(&catalog, 80_090_000));
    }

    #[test]
    fn added_and_removed_ini_slots_control_recognition() {
        use crate::catalog::Slot;
        let mut slots = SlotCatalog::default_er117().slots()[..4].to_vec();
        slots.push(Slot {
            selector: 255,
            event_flag: 6999,
            appearance_npc_param: 1_180_090_123,
        });
        let catalog = SlotCatalog::new(slots).unwrap();
        assert!(custom_character(&catalog, 8009));
        assert!(custom_appearance(&catalog, 1_180_090_123));
        assert!(!custom_character(&catalog, 8008));
        assert!(!custom_appearance(&catalog, 80_080_000));
        assert!(!custom_appearance(&catalog, 1_180_090_124));
    }

    #[test]
    #[ignore = "requires ER_TEST_EXE; executes native predicates in this test process"]
    fn native_predicates_before_after_and_rollback() {
        use crate::profile::PROFILE_1171;
        use pelite::pe64::{Pe, PeFile};
        use std::{ffi::c_void, ptr};

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn VirtualAlloc(address: *mut c_void, size: usize, kind: u32, protect: u32) -> *mut u8;
            fn VirtualFree(address: *mut c_void, size: usize, kind: u32) -> i32;
        }
        let bytes = std::fs::read(std::env::var_os("ER_TEST_EXE").unwrap()).unwrap();
        let pe = PeFile::from_bytes(&bytes).unwrap();
        let memory = unsafe { VirtualAlloc(ptr::null_mut(), 4096, 0x3000, 0x40) };
        assert!(!memory.is_null());
        let profile = Er117Profile {
            character_classifier_rva: 0,
            appearance_classifier_rva: 0x100,
            ..PROFILE_1171
        };
        unsafe {
            ptr::copy_nonoverlapping(
                pe.slice_bytes(PROFILE_1171.character_classifier_rva as u32)
                    .unwrap()
                    .as_ptr(),
                memory,
                25,
            );
            ptr::copy_nonoverlapping(
                pe.slice_bytes(PROFILE_1171.appearance_classifier_rva as u32)
                    .unwrap()
                    .as_ptr(),
                memory.add(0x100),
                40,
            );
        }
        let character: CharacterPredicate = unsafe { transmute(memory) };
        let appearance: AppearancePredicate = unsafe { transmute(memory.add(0x100)) };
        // Relocate the actual native pose-selection and physics-setting blocks
        // into this page so the regression reaches consumers of the predicates.
        unsafe {
            ptr::copy_nonoverlapping(
                pe.slice_bytes(0xD430F0).unwrap().as_ptr(),
                memory.add(0x180),
                40,
            );
        }
        fn rel32(code: &mut [u8], at: usize, next: usize, target: usize) {
            code[at..at + 4]
                .copy_from_slice(&((target as isize - next as isize) as i32).to_le_bytes());
        }
        let mut pose = vec![0x48, 0x83, 0xEC, 0x28, 0x48, 0x8B, 0xC1];
        let pose_start = 0x200 + pose.len();
        let mut fragment = pe.slice_bytes(0x41ED89).unwrap()[..0x1C].to_vec();
        rel32(&mut fragment, 4, pose_start + 8, 0);
        rel32(&mut fragment, 11, pose_start + 15, 0x380);
        rel32(&mut fragment, 21, pose_start + 25, 0x388);
        pose.extend(fragment);
        pose.extend([0x48, 0x83, 0xC4, 0x28, 0xC3]);
        unsafe {
            ptr::copy_nonoverlapping(pose.as_ptr(), memory.add(0x200), pose.len());
            memory.add(0x380).cast::<usize>().write_unaligned(0x1111); // W_Ridden_Enemy_Idles
            memory.add(0x388).cast::<usize>().write_unaligned(0x2222); // W_RideIdle
        }
        let select_pose: unsafe extern "C" fn(*const u32) -> usize =
            unsafe { transmute(memory.add(0x200)) };
        unsafe {
            ptr::copy_nonoverlapping(
                pe.slice_bytes(0xD42180).unwrap().as_ptr(),
                memory.add(0x600),
                76,
            );
        }
        let model_id: unsafe extern "C" fn(*mut u32, i32) -> *mut u32 =
            unsafe { transmute(memory.add(0x600)) };
        for id in [80_080_000, 80_080_123, 1_180_080_123, i32::MAX] {
            let mut native_id = 0;
            unsafe {
                model_id(&mut native_id, id);
            }
            assert_eq!(character_from_appearance(id), native_id);
        }
        let mut physics = vec![
            0x55, 0x56, 0x41, 0x55, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xF1, 0x48, 0x8B, 0xEA,
            0xE8, 0, 0, 0, 0, 0x44, 0x0F, 0xB6, 0xE8,
        ];
        rel32(&mut physics, 15, 0x400 + 19, 0x100);
        let physics_start = 0x400 + physics.len();
        let mut fragment = pe.slice_bytes(0x460067).unwrap()[..0x5F].to_vec();
        for (at, next, target) in [
            (9, 13, 0x580),
            (0x26, 0x2A, 0x180),
            (0x32, 0x36, 0x588),
            (0x3C, 0x40, 0x584),
            (0x44, 0x48, 0x590),
            (0x51, 0x55, 0x58C),
        ] {
            rel32(&mut fragment, at, physics_start + next, target);
        }
        physics.extend(fragment);
        physics.extend([0x48, 0x83, 0xC4, 0x20, 0x41, 0x5D, 0x5E, 0x5D, 0xC3]);
        unsafe {
            ptr::copy_nonoverlapping(physics.as_ptr(), memory.add(0x400), physics.len());
            for (offset, value) in [
                (0x580, 0.8f32),
                (0x584, 0.3),
                (0x588, 1.5),
                (0x58C, 50f32.to_radians()),
                (0x590, 60f32.to_radians()),
            ] {
                memory.add(offset).cast::<f32>().write_unaligned(value);
            }
        }
        let init_physics: unsafe extern "C" fn(*const Entry, *mut u8) =
            unsafe { transmute(memory.add(0x400)) };
        #[repr(C)]
        struct Entry {
            pointer: usize,
            id: i32,
            pad: i32,
        }
        let npc_param = [0u8; 0x40]; // alternate native predicate sees npcType=0
        let entry = |id| Entry {
            pointer: npc_param.as_ptr() as usize,
            id,
            pad: 0,
        };
        let invoke = |id| unsafe { appearance((&entry(id) as *const Entry).cast()) };
        let height = |id| {
            let mut config = [0u8; 0x40];
            unsafe {
                init_physics(&entry(id), config.as_mut_ptr());
                config.as_ptr().add(0x17).cast::<f32>().read_unaligned()
            }
        };
        assert_eq!(unsafe { character(&8008) }, 0);
        assert_eq!(invoke(80_080_000), 0);
        assert_eq!(height(80_020_000), 0.8);
        assert_eq!(height(80_080_000), 0.3);
        assert_eq!(unsafe { select_pose(&8002) }, 0x2222);
        assert_eq!(unsafe { select_pose(&8008) }, 0x1111);
        let hooks =
            unsafe { install(memory as usize, &profile, SlotCatalog::default_er117()) }.unwrap();
        for id in [8000, 8002, 8003, 8004, 8005, 8006, 8007, 8008] {
            assert_eq!(unsafe { character(&id) }, 1);
        }
        for id in [0, 8001, 8009, 9000] {
            assert_eq!(unsafe { character(&id) }, 0);
        }
        for id in [
            80_020_000, 80_030_000, 80_040_000, 80_050_000, 80_060_000, 80_070_000, 80_080_000,
        ] {
            assert_eq!(invoke(id), 1);
        }
        assert_eq!(invoke(80_090_000), 0);
        assert_eq!(height(80_020_000), 0.8);
        assert_eq!(height(80_080_000), 0.8);
        assert_eq!(height(80_090_000), 0.3);
        assert_eq!(unsafe { select_pose(&8008) }, 0x2222);
        assert_eq!(unsafe { select_pose(&8009) }, 0x1111);
        let missing = Entry {
            pointer: 0,
            id: 80_080_000,
            pad: 0,
        };
        assert_eq!(unsafe { appearance((&missing as *const Entry).cast()) }, 0);
        drop(hooks);
        assert_eq!(unsafe { character(&8008) }, 0);
        assert_eq!(invoke(80_080_000), 0);
        assert_eq!(height(80_080_000), 0.3);
        assert_eq!(unsafe { select_pose(&8008) }, 0x1111);
        assert_ne!(unsafe { VirtualFree(memory.cast(), 0, 0x8000) }, 0);
    }
}
