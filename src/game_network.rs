use std::{collections::HashSet, ptr, time::Duration};

use eldenring::{
    cs::{
        CSTaskGroupIndex, CSTaskImp, PlayerIns, PlayerNetworkSession, ReceivePacketAllocation,
        ReceivedPacketReader, WorldChrMan,
    },
    fd4::FD4TaskData,
};
use fromsoftware_shared::{FromStatic, SharedTaskImpExt};
use windows::{Win32::System::LibraryLoader::GetModuleHandleA, core::PCSTR};

use crate::{
    catalog::SlotCatalog,
    hooks, logging,
    network::{InboundPacket, NetworkAdapter, SyncEngine},
};

const MAX_PACKETS_PER_PEER_PER_FRAME: usize = 8;
const RECEIVE_BUFFER_SIZE: usize = 256;

pub fn install_probe(
    catalog: SlotCatalog,
    packet_type: u8,
    heartbeat_frames: u32,
) -> Result<(), String> {
    if !seamless_loaded() {
        return Err(
            "network probe requested but neither ersc.dll nor elden_ring_seamless_coop.dll is loaded"
                .to_string(),
        );
    }

    let task = CSTaskImp::wait_for_instance(Duration::from_secs(30))
        .map_err(|_| "CSTaskImp was not available within 30 seconds".to_string())?;
    let mut sync = SyncEngine::new(catalog, packet_type, hooks::local_selector());
    let mut frame = 0u32;
    let mut first_publish = true;
    let mut last_selector = hooks::local_selector();

    task.run_recurring(
        move |_: &FD4TaskData| {
            frame = frame.wrapping_add(1);
            let mut adapter = GameNetworkAdapter;
            let selector = hooks::local_selector();

            if selector != last_selector {
                match sync.set_local_selector(selector, &mut adapter) {
                    Ok(true) => logging::write(format!(
                        "network probe published selector change selector={selector} packet_type={packet_type}"
                    )),
                    Ok(false) => {}
                    Err(error) => logging::write(format!(
                        "network probe selector publish failed: {error}"
                    )),
                }
                last_selector = selector;
                first_publish = false;
            } else if first_publish || frame % heartbeat_frames == 1 {
                match sync.publish(&mut adapter) {
                    Ok(()) => logging::write(format!(
                        "network probe heartbeat frame={frame} selector={selector} packet_type={packet_type}"
                    )),
                    Err(error) => {
                        logging::write(format!("network probe heartbeat failed: {error}"))
                    }
                }
                first_publish = false;
            }

            match sync.receive(&mut adapter) {
                Ok(results) => {
                    for (peer, result) in results {
                        match result {
                            Ok(selector) => logging::write(format!(
                                "network probe accepted peer={peer} selector={selector}"
                            )),
                            Err(reason) => logging::write(format!(
                                "network probe rejected peer={peer} reason={reason:?}"
                            )),
                        }
                    }
                }
                Err(error) => logging::write(format!("network probe receive failed: {error}")),
            }
        },
        CSTaskGroupIndex::TaskLineIdx_FrpgNet_Other,
    );

    logging::write(format!(
        "network probe scheduler installed packet_type={packet_type} heartbeat_frames={heartbeat_frames}"
    ));
    Ok(())
}

fn seamless_loaded() -> bool {
    [
        b"ersc.dll\0".as_slice(),
        b"elden_ring_seamless_coop.dll\0".as_slice(),
    ]
    .into_iter()
    .any(|name| unsafe { GetModuleHandleA(PCSTR(name.as_ptr())) }.is_ok())
}

struct GameNetworkAdapter;

impl NetworkAdapter for GameNetworkAdapter {
    type Error = String;

    fn broadcast(&mut self, packet_type: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        let sessions = unsafe { remote_sessions()? };
        let Some(session) = sessions.first().copied() else {
            return Ok(());
        };
        let length = u32::try_from(bytes.len())
            .map_err(|_| "outbound packet does not fit a u32 length".to_string())?;
        unsafe {
            let player_session = &(*session).player_session;
            (player_session.vftable.broadcast_packet)(
                player_session,
                bytes.as_ptr(),
                length,
                packet_type,
            );
        }
        Ok(())
    }

    fn drain(&mut self, packet_type: u8) -> Result<Vec<InboundPacket>, Self::Error> {
        let sessions = unsafe { remote_sessions()? };
        let mut packets = Vec::new();

        for session in sessions {
            let peer = unsafe { (*session).remote_identity };
            for _ in 0..MAX_PACKETS_PER_PEER_PER_FRAME {
                let mut storage = [0u8; RECEIVE_BUFFER_SIZE];
                let mut allocation = ReceivePacketAllocation {
                    buffer: storage.as_mut_ptr(),
                    size: storage.len(),
                };
                let mut reader = ReceivedPacketReader {
                    buffer: ptr::null_mut(),
                    allocation_size: 0,
                    received_size: 0,
                };

                unsafe {
                    let player_session = &mut (*session).player_session;
                    let _ = (player_session.vftable.receive_packet)(
                        player_session,
                        &mut allocation,
                        &mut reader,
                        packet_type,
                    );
                }
                if reader.received_size == 0 {
                    break;
                }
                if reader.received_size > RECEIVE_BUFFER_SIZE {
                    return Err(format!(
                        "peer {peer} reported oversized packet {} > {RECEIVE_BUFFER_SIZE}",
                        reader.received_size
                    ));
                }

                let source = if reader.buffer.is_null() {
                    storage.as_ptr()
                } else {
                    reader.buffer.cast_const()
                };
                let bytes = unsafe { std::slice::from_raw_parts(source, reader.received_size) };
                packets.push(InboundPacket {
                    peer,
                    bytes: bytes.to_vec(),
                });
            }
        }
        Ok(packets)
    }
}

unsafe fn remote_sessions() -> Result<Vec<*mut PlayerNetworkSession>, String> {
    let world = unsafe { WorldChrMan::instance_mut() }
        .map_err(|_| "WorldChrMan is not available".to_string())?;
    let mut identities = HashSet::new();
    let mut sessions = Vec::new();

    for player in world.player_chr_set.characters() {
        let session = unsafe { raw_session_pointer(player) };
        if session.is_null() {
            continue;
        }
        let identity = unsafe { (*session).remote_identity };
        if identity == 0 || !identities.insert(identity) {
            continue;
        }
        sessions.push(session);
    }
    Ok(sessions)
}

unsafe fn raw_session_pointer(player: &mut PlayerIns) -> *mut PlayerNetworkSession {
    let owned_pointer_address =
        ptr::addr_of_mut!(player.player_session_holder.player_network_session);
    unsafe {
        owned_pointer_address
            .cast::<*mut PlayerNetworkSession>()
            .read()
    }
}
