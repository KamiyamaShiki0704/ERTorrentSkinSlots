//! Bounded, read-only observations at the native selection/restore boundary.
//! The CSRideGameData layout was traced in the WW 2.7.1.0 save/restore code.
use std::sync::Mutex;

use crate::{logging, profile::Er117Profile};
use shikifsrs_runtime::memory::{AddressSpace, CurrentProcess};

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
pub unsafe fn observe(_base: usize, profile: &Er117Profile, selector: u8, phase: &str) {
    if !logging::enabled() || profile.name != crate::profile::PROFILE_1171.name {
        return;
    }
    let Ok(mut trace) = TRACE.try_lock() else {
        return;
    };
    if trace.1 >= TRACE_LIMIT {
        return;
    }
    let Some(manager) = crate::sdk::trace_manager() else {
        return;
    };
    let snapshot = snapshot_from(
        &CurrentProcess,
        manager,
        profile.game_selector_offset,
        selector,
    );
    if crate::sdk::trace_manager() != Some(manager) {
        return;
    }
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

fn snapshot_from(
    memory: &impl AddressSpace,
    manager: u64,
    mirror_offset: usize,
    selector: u8,
) -> Option<Snapshot> {
    fn once(
        memory: &impl AddressSpace,
        manager: u64,
        mirror_offset: usize,
        selector: u8,
    ) -> Option<Snapshot> {
        if manager == 0 {
            return None;
        }
        let player = memory.read_u64(manager.checked_add(8)?).ok()?;
        if player == 0 {
            return None;
        }
        let ride = memory.read_u64(player.checked_add(0x8e0)?).ok()?;
        Some(Snapshot {
            player_data: usize::try_from(player).ok()?,
            ride_data: usize::try_from(ride).ok()?,
            handle: memory.read_u64(player.checked_add(0x950)?).ok()?,
            flags: if ride == 0 {
                None
            } else {
                Some(memory.read_u32(ride.checked_add(0x34)?).ok()?)
            },
            hp: if ride == 0 {
                None
            } else {
                Some(memory.read_u32(ride.checked_add(0x30)?).ok()?)
            },
            selector,
            mirror: memory
                .read_u8(player.checked_add(u64::try_from(mirror_offset).ok()?)?)
                .ok()?,
        })
    }
    let before = once(memory, manager, mirror_offset, selector)?;
    (once(memory, manager, mirror_offset, selector)? == before).then_some(before)
}

#[cfg(test)]
unsafe fn read_snapshot(
    manager: *const u8,
    mirror_offset: usize,
    selector: u8,
) -> Option<Snapshot> {
    snapshot_from(&CurrentProcess, manager as u64, mirror_offset, selector)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_rejects_every_selected_byte_truncation_or_visible_change() {
        use shikifsrs_runtime::{Error, Result, memory::SliceMemory};
        use std::cell::Cell;
        struct Fault<'a> {
            memory: SliceMemory<'a>,
            byte: u64,
            change: bool,
            reads: Cell<u32>,
        }
        impl AddressSpace for Fault<'_> {
            fn read_exact(&self, address: u64, out: &mut [u8]) -> Result<()> {
                if address <= self.byte && self.byte < address + out.len() as u64 {
                    if !self.change {
                        return Err(Error::OutOfBounds {
                            address,
                            length: out.len(),
                        });
                    }
                    let count = self.reads.get();
                    self.reads.set(count + 1);
                    self.memory.read_exact(address, out)?;
                    if count != 0 {
                        out[(self.byte - address) as usize] ^= 0x80;
                    }
                    return Ok(());
                }
                self.memory.read_exact(address, out)
            }
        }
        let mut bytes = [0u8; 0x1200];
        bytes[8..16].copy_from_slice(&0x1100u64.to_le_bytes());
        bytes[0x9e0..0x9e8].copy_from_slice(&0x2100u64.to_le_bytes());
        bytes[0xa50..0xa58].copy_from_slice(&0x123456789abcdef0u64.to_le_bytes());
        bytes[0xa60] = 2;
        bytes[0x1130..0x1134].copy_from_slice(&1400u32.to_le_bytes());
        bytes[0x1134..0x1138].copy_from_slice(&13u32.to_le_bytes());
        let memory = SliceMemory::new(0x1000, &bytes).unwrap();
        assert_eq!(
            snapshot_from(&memory, 0x1000, 0x960, 6).unwrap().hp,
            Some(1400)
        );
        for (at, len) in [
            (0x1008, 8),
            (0x19e0, 8),
            (0x1a50, 8),
            (0x1a60, 1),
            (0x2130, 4),
            (0x2134, 4),
        ] {
            for byte in at..at + len {
                for change in [false, true] {
                    let fault = Fault {
                        memory,
                        byte,
                        change,
                        reads: Cell::new(0),
                    };
                    assert_eq!(
                        snapshot_from(&fault, 0x1000, 0x960, 6),
                        None,
                        "byte={byte:x} change={change}"
                    );
                }
            }
        }
        assert_eq!(snapshot_from(&memory, u64::MAX, 0x960, 6), None);
    }

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
