#![cfg_attr(not(test), allow(dead_code))]

use std::{
    ffi::c_void,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use windows::Win32::{Foundation::HMODULE, System::LibraryLoader::GetModuleFileNameW};

pub mod catalog;
pub mod config;
mod game_network;
mod hooks;
mod logging;
pub mod network;
pub mod profile;
pub mod protocol;
pub mod state;

use config::{Config, NetworkMode};

/// # Safety
///
/// Windows calls this entry point while loading the DLL. It must not be invoked manually.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DllMain(hmodule: usize, reason: u32, _reserved: usize) -> bool {
    const DLL_PROCESS_ATTACH: u32 = 1;
    if reason != DLL_PROCESS_ATTACH {
        return true;
    }

    thread::spawn(move || initialize(hmodule));
    true
}

fn initialize(hmodule: usize) {
    let module_path = module_path(hmodule);
    let config_path = module_path
        .as_deref()
        .and_then(|path| sidecar_path(path, "er_torrent_skin_slots.ini"));
    let config = match config_path.as_deref() {
        Some(path) => Config::load(path),
        None => Ok(Config::default()),
    };

    let config = match config {
        Ok(config) => config,
        Err(error) => {
            let log_path = module_path
                .as_deref()
                .and_then(|path| sidecar_path(path, "er_torrent_skin_slots.log"));
            logging::initialize(log_path);
            logging::write(format!("configuration rejected: {error}"));
            return;
        }
    };

    let log_path = config
        .log_enabled
        .then(|| {
            module_path
                .as_deref()
                .and_then(|path| sidecar_path(path, "er_torrent_skin_slots.log"))
        })
        .flatten();
    logging::initialize(log_path);
    logging::write(format!(
        "loaded enabled={} slots={} catalog_hash=0x{:016X} network_mode={:?} packet_type={} heartbeat_frames={}",
        config.enabled,
        config.catalog.slots().len(),
        config.catalog.hash(),
        config.network_mode,
        config.packet_type,
        config.heartbeat_frames
    ));

    if !config.enabled {
        logging::write("disabled by configuration");
        return;
    }

    thread::sleep(Duration::from_millis(1500));
    let (base, profile) = match profile::detect() {
        Ok(result) => result,
        Err(error) => {
            logging::write(format!(
                "safety gate rejected executable: {error}; zero hooks installed"
            ));
            return;
        }
    };
    logging::write(format!(
        "safety gate accepted {} base=0x{base:X}",
        profile.name
    ));

    if let Err(error) = hooks::install(base, profile, config.catalog.clone()) {
        logging::write(format!("hook installation failed: {error}"));
        return;
    }

    match config.network_mode {
        NetworkMode::Off => logging::write("network adapter disabled; local true-slot probe only"),
        NetworkMode::Probe => {
            if let Err(error) = game_network::install_probe(
                config.catalog,
                config.packet_type,
                config.heartbeat_frames,
            ) {
                logging::write(format!("network probe not installed: {error}"));
            }
        }
        NetworkMode::On => logging::write(
            "network_mode=on rejected: no runtime-proven Seamless packet/identity profile; local hooks remain active",
        ),
    }
}

fn module_path(hmodule: usize) -> Option<PathBuf> {
    let mut buffer = [0u16; 32768];
    let length =
        unsafe { GetModuleFileNameW(Some(HMODULE(hmodule as *mut c_void)), &mut buffer) } as usize;
    (length != 0).then(|| PathBuf::from(String::from_utf16_lossy(&buffer[..length])))
}

fn sidecar_path(module_path: &Path, file_name: &str) -> Option<PathBuf> {
    let mut path = module_path.to_path_buf();
    path.set_file_name(file_name);
    Some(path)
}
