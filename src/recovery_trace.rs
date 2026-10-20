//! Bounded, read-only observations at the native selection/restore boundary.
//! The CSRideGameData layout was traced in the WW 2.7.1.0 save/restore code.
use std::sync::Mutex;

use crate::{logging, profile::Er117Profile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Snapshot {
    player_data: usize,
    ride_data: usize,
    handle: u64,
    flags: Option<u32>,
    hp: Option<u32>,
    selector: u8,
    mirror: u8,
}

static TRACE: Mutex<(Option<Snapshot>, usize)> = Mutex::new((None, 0));
const TRACE_LIMIT: usize = 64;

/// Called synchronously from the native refresh hook, while game data is owned
/// by the calling game thread. Offsets are read-only and null-checked.
pub unsafe fn observe(base: usize, profile: &Er117Profile, selector: u8, phase: &str) {
    if !logging::enabled() || profile.name != crate::profile::PROFILE_1171.name {
        return;
    }
    let Ok(mut trace) = TRACE.try_lock() else {
        return;
    };
    if trace.1 >= TRACE_LIMIT {
        return;
    }
    let manager =
        unsafe { ((base + profile.game_data_man_global_rva) as *const *const u8).read_unaligned() };
    let snapshot = unsafe { read_snapshot(manager, profile.game_selector_offset, selector) };
    if snapshot == trace.0 {
        return;
    }
    trace.0 = snapshot;
    trace.1 += 1;
    match snapshot {
        Some(snapshot) => logging::write(format!(
            "mount recovery phase={phase} player_data=0x{:X} ride_data=0x{:X} handle=0x{:016X} flags={:?} hp={:?} selector={} mirror={}",
            snapshot.player_data,
            snapshot.ride_data,
            snapshot.handle,
            snapshot.flags,
            snapshot.hp,
            snapshot.selector,
            snapshot.mirror
        )),
        None => logging::write(format!("mount recovery phase={phase} player_data=none")),
    }
}

unsafe fn read_snapshot(
    manager: *const u8,
    mirror_offset: usize,
    selector: u8,
) -> Option<Snapshot> {
    if manager.is_null() {
        return None;
    }
    let player = unsafe { manager.add(8).cast::<*const u8>().read_unaligned() };
    if player.is_null() {
        return None;
    }
    let ride = unsafe { player.add(0x8E0).cast::<*const u8>().read_unaligned() };
    Some(Snapshot {
        player_data: player as usize,
        ride_data: ride as usize,
        handle: unsafe { player.add(0x950).cast::<u64>().read_unaligned() },
        flags: (!ride.is_null()).then(|| unsafe { ride.add(0x34).cast::<u32>().read_unaligned() }),
        hp: (!ride.is_null()).then(|| unsafe { ride.add(0x30).cast::<u32>().read_unaligned() }),
        selector,
        mirror: unsafe { player.add(mirror_offset).read() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reads_native_layout_without_mutating_it() {
        let mut manager = [0u8; 0x10];
        let mut player = [0u8; 0x970];
        let mut ride = [0u8; 0x40];
        unsafe {
            manager
                .as_mut_ptr()
                .add(8)
                .cast::<*const u8>()
                .write_unaligned(player.as_ptr());
            player
                .as_mut_ptr()
                .add(0x8E0)
                .cast::<*const u8>()
                .write_unaligned(ride.as_ptr());
            player
                .as_mut_ptr()
                .add(0x950)
                .cast::<u64>()
                .write_unaligned(0x123456789ABCDEF0);
            ride.as_mut_ptr()
                .add(0x30)
                .cast::<u32>()
                .write_unaligned(1400);
            ride.as_mut_ptr()
                .add(0x34)
                .cast::<u32>()
                .write_unaligned(13);
        }
        let before = (manager, player, ride);
        let snapshot = unsafe { read_snapshot(manager.as_ptr(), 0x960, 6) }.unwrap();
        assert_eq!(snapshot.flags, Some(13));
        assert_eq!(snapshot.hp, Some(1400));
        assert_eq!(snapshot.handle, 0x123456789ABCDEF0);
        assert_eq!((snapshot.selector, snapshot.mirror), (6, 0));
        assert_eq!((manager, player, ride), before);
        unsafe {
            player
                .as_mut_ptr()
                .add(0x8E0)
                .cast::<usize>()
                .write_unaligned(0)
        };
        assert_eq!(
            unsafe { read_snapshot(manager.as_ptr(), 0x960, 6) }
                .unwrap()
                .flags,
            None
        );
        unsafe {
            manager
                .as_mut_ptr()
                .add(8)
                .cast::<usize>()
                .write_unaligned(0)
        };
        assert_eq!(unsafe { read_snapshot(manager.as_ptr(), 0x960, 6) }, None);
        assert_eq!(unsafe { read_snapshot(std::ptr::null(), 0x960, 6) }, None);
    }
}
