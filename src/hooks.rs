use std::{
    cell::Cell,
    mem::transmute,
    sync::{
        OnceLock,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    },
};

use ilhook::x64::{CallbackOption, HookFlags, Registers, hook_closure_retn};
use shikifsrs_elden_ring::Ww171File;
use shikifsrs_runtime::{
    code_patches::{CodePatchSite, PreparedCodeSites},
    memory::{AddressSpace, CurrentProcess},
};
use std::sync::Arc;

use crate::{catalog::SlotCatalog, logging, profile::Er117Profile};

static CATALOG: OnceLock<SlotCatalog> = OnceLock::new();
static LOCAL_SELECTOR: AtomicU8 = AtomicU8::new(0);
static RESOLVER_LOG_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    static REMOTE_SELECTOR_CONTEXT: Cell<Option<u8>> = const { Cell::new(None) };
}

type SelectorRefreshFn = unsafe extern "C" fn() -> bool;
type AppearanceResolverFn = unsafe extern "C" fn(u8) -> i32;

pub fn install(
    base: usize,
    profile: &'static Er117Profile,
    catalog: SlotCatalog,
) -> Result<(), String> {
    let bytes: Arc<[u8]> = std::fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?
        .into();
    let file = Ww171File::verify(&bytes).map_err(|e| e.to_string())?;
    let objects = file
        .bind_objects(&CurrentProcess, base as u64)
        .map_err(|e| e.to_string())?;
    let actual = objects.torrent_hook_sites().map_err(|e| e.to_string())?;
    if (
        actual.appearance_resolver,
        actual.selector_refresh,
        actual.spawn_return,
        actual.assembly_return,
    ) != (
        (base + profile.appearance_resolver_rva) as u64,
        (base + profile.selector_refresh_rva) as u64,
        (base + profile.local_resolver_return_rva) as u64,
        (base + profile.remote_resolver_return_rva) as u64,
    ) {
        return Err("owned hook sites differ from SDK current evidence".into());
    }
    let original_evidence = file.torrent_hook_evidence();
    // Independently checked complete current-version classifier leaves.
    let evidence = [
        original_evidence[0],
        original_evidence[1],
        shikifsrs_runtime::profile::CodeRange {
            rva: 0x3e5ea0,
            length: 25,
        },
        shikifsrs_runtime::profile::CodeRange {
            rva: 0xd43120,
            length: 40,
        },
    ];
    let sites = evidence.map(|evidence| CodePatchSite {
        rva: evidence.rva,
        evidence,
        length: 14,
    });
    let prepared = PreparedCodeSites::prepare(file.file(), &CurrentProcess, base as u64, &sites)
        .map_err(|e| e.to_string())?;
    let mut pinned = windows::Win32::Foundation::HMODULE::default();
    // SAFETY: FROM_ADDRESS uses this DLL function; PIN keeps owned hooks resident even on abort.
    unsafe {
        windows::Win32::System::LibraryLoader::GetModuleHandleExW(
            0x5,
            windows::core::PCWSTR(install as *const () as *const u16),
            &mut pinned,
        )
    }
    .map_err(|e| e.to_string())?;
    CATALOG
        .set(catalog)
        .map_err(|_| "slot catalog was already installed".to_string())?;

    // Keep the full transaction owned locally. A later install failure drops
    // earlier hooks and restores their original bytes.
    let recognition_hooks =
        unsafe { crate::mount_identity::install(base, profile, CATALOG.get().unwrap().clone())? };

    let resolver_address = base + profile.appearance_resolver_rva;
    let resolver_hook = unsafe {
        hook_closure_retn(
            resolver_address,
            move |registers, original| appearance_resolver_hook(base, profile, registers, original),
            CallbackOption::None,
            HookFlags::empty(),
        )
    }
    .map_err(|error| format!("appearance resolver hook install failed: {error:?}"))?;

    let refresh_address = base + profile.selector_refresh_rva;
    let refresh_hook = unsafe {
        hook_closure_retn(
            refresh_address,
            move |_registers, original| selector_refresh_hook(base, profile, original),
            CallbackOption::None,
            HookFlags::empty(),
        )
    }
    .map_err(|error| format!("selector refresh hook install failed: {error:?}"))?;

    let declarations = sites
        .iter()
        .map(|site| {
            let mut bytes = vec![0; site.length];
            CurrentProcess
                .read_exact(base as u64 + u64::from(site.rva), &mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes[0] != 0xe9 && bytes[..6] != [0xff, 0x25, 0, 0, 0, 0] {
                return Err("owned ilhook redirect encoding differs".to_string());
            }
            Ok((site.rva, bytes))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let prepared = prepared
        .declare_replacements(&declarations)
        .map_err(|e| e.to_string())?;
    // SAFETY: Only these just-installed owned hooks, pinned DLL and retained trampolines.
    // Installation quiescence/ABI remain experimental host obligations, no third-party patch.
    let patches = Arc::new(unsafe { prepared.into_owned() });
    crate::sdk::publish(crate::sdk::Context {
        bytes,
        base: base as u64,
        patches,
    })?;
    let _ = Box::leak(Box::new(resolver_hook));
    let _ = Box::leak(Box::new(refresh_hook));
    let _ = Box::leak(Box::new(recognition_hooks));
    logging::write(format!(
        "hooks installed selector_refresh=eldenring.exe+0x{:X} resolver=eldenring.exe+0x{:X}",
        profile.selector_refresh_rva, profile.appearance_resolver_rva
    ));
    logging::write(format!(
        "mount identity hooks installed character=eldenring.exe+0x{:X} appearance=eldenring.exe+0x{:X}",
        profile.character_classifier_rva, profile.appearance_classifier_rva
    ));
    Ok(())
}

pub fn local_selector() -> u8 {
    LOCAL_SELECTOR.load(Ordering::Acquire)
}

pub fn with_remote_selector<R>(selector: Option<u8>, operation: impl FnOnce() -> R) -> R {
    REMOTE_SELECTOR_CONTEXT.with(|context| {
        let previous = context.replace(selector);
        let result = operation();
        context.set(previous);
        result
    })
}

fn selector_refresh_hook(base: usize, profile: &Er117Profile, original: usize) -> usize {
    // AFDE40 calls this refresh before restoring CSRideGameData. Observe only;
    // never manufacture saved flags, mount handles, or ride-node relationships.
    unsafe { crate::recovery_trace::observe(base, profile, local_selector(), "before-refresh") };
    let original: SelectorRefreshFn = unsafe { transmute(original) };
    let original_changed = unsafe { original() };
    let Some(catalog) = CATALOG.get() else {
        return usize::from(original_changed);
    };

    let Some((selection, mirror_changed)) = crate::sdk::select(catalog) else {
        return usize::from(original_changed);
    };
    let previous = LOCAL_SELECTOR.swap(selection.selector, Ordering::AcqRel);
    unsafe { crate::recovery_trace::observe(base, profile, selection.selector, "after-refresh") };

    if previous != selection.selector {
        logging::write(format!(
            "local selector changed previous={} selector={} vanilla_mirror={} appearance={:?}",
            previous,
            selection.selector,
            selection.vanilla_mirror,
            catalog.appearance(selection.selector)
        ));
    }

    usize::from(original_changed || previous != selection.selector || mirror_changed)
}

fn appearance_resolver_hook(
    base: usize,
    profile: &Er117Profile,
    registers: *mut Registers,
    original: usize,
) -> usize {
    let registers = unsafe { &*registers };
    let raw_selector = registers.rcx as u8;
    let caller = unsafe { registers.get_stack(0) as usize };
    let selector = if caller == base + profile.local_resolver_return_rva {
        LOCAL_SELECTOR.load(Ordering::Acquire)
    } else if caller == base + profile.remote_resolver_return_rva {
        REMOTE_SELECTOR_CONTEXT
            .with(Cell::get)
            .unwrap_or(raw_selector)
    } else {
        raw_selector
    };

    if let Some(appearance) = CATALOG
        .get()
        .and_then(|catalog| catalog.appearance(selector))
    {
        let count = RESOLVER_LOG_COUNT.fetch_add(1, Ordering::Relaxed);
        if selector >= 4 || count < 16 {
            logging::write(format!(
                "appearance resolve caller=eldenring.exe+0x{:X} raw_selector={} effective_selector={} appearance={}",
                caller.saturating_sub(base),
                raw_selector,
                selector,
                appearance
            ));
        }
        return appearance as u32 as usize;
    }

    let original: AppearanceResolverFn = unsafe { transmute(original) };
    unsafe { original(raw_selector) as u32 as usize }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_context_restores_previous_value() {
        REMOTE_SELECTOR_CONTEXT.with(|context| context.set(Some(2)));
        let seen = with_remote_selector(Some(6), || REMOTE_SELECTOR_CONTEXT.with(Cell::get));
        assert_eq!(seen, Some(6));
        assert_eq!(REMOTE_SELECTOR_CONTEXT.with(Cell::get), Some(2));
    }
}
