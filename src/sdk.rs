//! Consumer-owned phase/epoch policy. SDK memory checks do not acquire native graph locks.
use shikifsrs_elden_ring::{
    TaskCallback, Ww171File,
    current_network::CurrentNetworkSessions,
    current_tasks::CurrentTaskEngine,
    current_torrent::{CurrentTorrentSelector, TorrentSelectorWrite},
    task_resident::ResidentCallbackGroupSlot,
};
use shikifsrs_runtime::{code_patches::OwnedCodePatches, memory::CurrentProcess};
use std::{
    cell::RefCell,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
pub struct Context {
    pub bytes: Arc<[u8]>,
    pub base: u64,
    pub patches: Arc<OwnedCodePatches>,
}
static CONTEXT: OnceLock<Context> = OnceLock::new();
static NETWORK_TASK: ResidentCallbackGroupSlot<1> = ResidentCallbackGroupSlot::new();
thread_local! {
    static SELECTOR: RefCell<Option<CurrentTorrentSelector>> = const { RefCell::new(None) };
    static NETWORK: RefCell<Option<CurrentNetworkSessions>> = const { RefCell::new(None) };
}
pub fn publish(context: Context) -> Result<(), String> {
    CONTEXT
        .set(context)
        .map_err(|_| "SDK context already published".to_string())
}
pub fn select(
    catalog: &crate::catalog::SlotCatalog,
) -> Option<(crate::state::LocalSelection, bool)> {
    let context = CONTEXT.get()?;
    SELECTOR.with(|slot| {
        let mut reader = slot.borrow_mut();
        if reader.is_none() {
            // SAFETY: Called from the owned retained refresh hook on its actual thread.
            // Host supplies exclusive saved-state phase and resident graph; initial epoch1
            // is experimental and does not automatically detect native address reuse.
            *reader = unsafe {
                CurrentTorrentSelector::bind(
                    Arc::clone(&context.bytes),
                    context.base,
                    1,
                    Some(Arc::clone(&context.patches)),
                )
            }
            .ok();
        }
        let reader = reader.as_ref()?;
        let ids: Vec<_> = catalog.slots().iter().map(|slot| slot.event_flag).collect();
        let flags = match reader.flags(&ids, 65536) {
            Ok(Some(flags)) => flags,
            Ok(None) => return None,
            Err(error) => {
                crate::logging::write(format!("selector flag observation rejected: {error}"));
                return None;
            }
        };
        let selected = selection_from_flags(catalog, &flags);
        let mut changed = false;
        match reader.observe() {
            Ok(Some(before)) => {
                // SAFETY: Same owned refresh hook's exclusive saved allocation/phase contract.
                match unsafe { reader.compare_replace(before, selected.vanilla_mirror) } {
                    Ok(TorrentSelectorWrite::Applied) => changed = true,
                    Ok(_) => {}
                    Err(error) => {
                        crate::logging::write(format!("selector mirror rejected: {error}"))
                    }
                }
            }
            Ok(None) => {} // Preserve local publication when saved data has not initialized.
            Err(error) => crate::logging::write(format!("selector mirror unavailable: {error}")),
        }
        Some((selected, changed))
    })
}
fn selection_from_flags(
    catalog: &crate::catalog::SlotCatalog,
    flags: &[shikifsrs_elden_ring::event_flags::EventFlagObservation],
) -> crate::state::LocalSelection {
    crate::state::LocalSelection::new(
        catalog.select_by_flag(|id| flags.iter().any(|f| f.id == id && f.value)),
    )
}

pub fn with_network<T>(
    operation: impl FnOnce(&CurrentNetworkSessions) -> Result<T, String>,
) -> Result<T, String> {
    let context = CONTEXT.get().ok_or("SDK hook context unavailable")?;
    NETWORK.with(|slot| {
        let mut reader = slot.borrow_mut();
        if reader.is_none() {
            // SAFETY: First actual retained network callback thread; host supplies native
            // session/transport graph, serialized phase, and protected-helper obligations.
            // This code is compiled experimentally, never invoked by the offline test suite.
            *reader = Some(
                unsafe {
                    CurrentNetworkSessions::bind(
                        Arc::clone(&context.bytes),
                        context.base,
                        1,
                        Some(Arc::clone(&context.patches)),
                    )
                }
                .map_err(|e| e.to_string())?,
            );
        }
        operation(reader.as_ref().ok_or("network reader unavailable")?)
    })
}
pub fn register_network(callback: TaskCallback) -> Result<(), String> {
    let context = CONTEXT.get().ok_or("SDK hook context unavailable")?;
    let file = Ww171File::verify(&context.bytes).map_err(|e| e.to_string())?;
    let phase = file
        .phase("TaskLineIdx_FrpgNet_Other")
        .map_err(|e| e.to_string())?;
    let started = Instant::now();
    loop {
        let catalog = file
            .bind_with_patches(&CurrentProcess, context.base, Some(&context.patches))
            .map_err(|e| e.to_string())?;
        let id = catalog
            .observe_phase(phase)
            .map_err(|e| e.to_string())?
            .and_then(|v| v.native_group_id());
        if let Some(id) = id
            && catalog
                .registered_group(id, 4096)
                .map_err(|e| e.to_string())?
                .is_some()
        {
            break;
        }
        if started.elapsed() >= Duration::from_secs(30) {
            return Err("native network phase not registered within30seconds".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // SAFETY: Resident pinned module; serialized allocator/registration and actual future
    // callback phase/complete graphs remain explicit experimental host obligations.
    let engine = unsafe {
        CurrentTaskEngine::bind_with_patches(
            Arc::clone(&context.bytes),
            context.base,
            Some(Arc::clone(&context.patches)),
        )
    }
    .map_err(|e| e.to_string())?;
    // SAFETY: Static owner retains callback/receipt for process lifetime, no external unregister.
    let installed =
        unsafe { NETWORK_TASK.install(engine, [phase], [callback]) }.map_err(|e| e.to_string())?;
    let control = installed.registration.map_err(|e| e.to_string())?;
    if !control.registered_all() {
        return Err("network registration failed; retained actual outcomes".into());
    }
    Ok(())
}

/// Revalidate the exact module and owned hook manifest before copying diagnostic links.
pub fn trace_manager() -> Option<u64> {
    let context = CONTEXT.get()?;
    let file = Ww171File::verify(&context.bytes).ok()?;
    let objects = file
        .bind_objects_with_patches(&CurrentProcess, context.base, Some(&context.patches))
        .ok()?;
    objects
        .torrent_selector(1)
        .ok()?
        .map(|saved| saved.manager_address)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shikifsrs_elden_ring::event_flags::{EventFlagObservation, EventFlagStorage};

    #[test]
    fn sdk_flag_boundary_preserves_every_configured_slot_priority_and_native_mirror() {
        let catalog = crate::catalog::SlotCatalog::default_er117();
        let flag = |id, value| EventFlagObservation {
            manager_address: 8,
            id,
            group_id: id / 1000,
            bit_in_group: id % 1000,
            node_address: None,
            storage: EventFlagStorage::MissingGroup,
            value,
        };
        assert_eq!(
            selection_from_flags(&catalog, &[]),
            crate::state::LocalSelection::new(0)
        );
        for slot in catalog.slots() {
            let flags = [flag(slot.event_flag, true), flag(9000, true)];
            assert_eq!(
                selection_from_flags(&catalog, &flags),
                crate::state::LocalSelection::new(slot.selector)
            );
        }
        let flags = [flag(6706, true), flag(6704, true), flag(6700, false)];
        assert_eq!(
            selection_from_flags(&catalog, &flags),
            crate::state::LocalSelection {
                selector: 4,
                vanilla_mirror: 0
            }
        );
    }
}
