use shikifsrs_elden_ring::TaskCallback;
use std::collections::HashSet;
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

    let mut sync = SyncEngine::new(catalog, packet_type, hooks::local_selector());
    let mut frame = 0u32;
    let mut first_publish = true;
    let mut last_selector = hooks::local_selector();

    let callback = TaskCallback::new(move |_| {
        frame = frame.wrapping_add(1);
        let mut adapter = GameNetworkAdapter;
        let selector = hooks::local_selector();

        if selector != last_selector {
            match sync.set_local_selector(selector, &mut adapter) {
                Ok(true) => logging::write(format!(
                    "network probe published selector change selector={selector} packet_type={packet_type}"
                )),
                Ok(false) => {}
                Err(error) => {
                    logging::write(format!("network probe selector publish failed: {error}"))
                }
            }
            last_selector = selector;
            first_publish = false;
        } else if first_publish || heartbeat_due(frame, heartbeat_frames) {
            match sync.publish(&mut adapter) {
                Ok(()) => logging::write(format!(
                    "network probe heartbeat frame={frame} selector={selector} packet_type={packet_type}"
                )),
                Err(error) => logging::write(format!("network probe heartbeat failed: {error}")),
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
    });
    crate::sdk::register_network(callback)?;

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
        crate::sdk::with_network(|native| {
            let sessions = unique_sessions(native.observe(4096).map_err(|e| e.to_string())?);
            let Some(session) = sessions.first().copied() else {
                return Ok(());
            };
            // SAFETY: Only the actual retained network callback, not worker/tests. Complete
            // graph, authority and phase remain experimental caller contracts; receipt only.
            unsafe { native.broadcast(session, packet_type, bytes) }.map_err(|e| e.to_string())?;
            Ok(())
        })
    }
    fn drain(&mut self, packet_type: u8) -> Result<Vec<InboundPacket>, Self::Error> {
        crate::sdk::with_network(|native| {
            let sessions = unique_sessions(native.observe(4096).map_err(|e| e.to_string())?);
            let mut packets = Vec::new();
            for session in sessions {
                for _ in 0..MAX_PACKETS_PER_PEER_PER_FRAME {
                    // SAFETY: Same retained phase with exclusive session/transport graph;
                    // SDK's decoded ABI owns its output buffer and checks all copy bounds.
                    let bytes =
                        unsafe { native.receive(session, packet_type, RECEIVE_BUFFER_SIZE) }
                            .map_err(|e| e.to_string())?;
                    if bytes.is_empty() {
                        break;
                    }
                    packets.push(InboundPacket {
                        peer: session.native_identifier,
                        bytes,
                    });
                }
            }
            Ok(packets)
        })
    }
}
fn unique_sessions(
    sessions: Vec<shikifsrs_elden_ring::network_sessions::NetworkSessionObservation>,
) -> Vec<shikifsrs_elden_ring::network_sessions::NetworkSessionObservation> {
    let mut identities = HashSet::new();
    sessions
        .into_iter()
        .filter(|session| {
            session.native_identifier != 0 && identities.insert(session.native_identifier)
        })
        .collect()
}
fn heartbeat_due(frame: u32, every: u32) -> bool {
    frame.wrapping_sub(1).is_multiple_of(every.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn heartbeat_keeps_original_frame_one_schedule_and_repeats_every_frame_when_interval_one() {
        assert_eq!(
            (1..=13)
                .filter(|frame| heartbeat_due(*frame, 6))
                .collect::<Vec<_>>(),
            vec![1, 7, 13]
        );
        assert!((1..=12).all(|frame| heartbeat_due(frame, 1)));
    }
    #[test]
    fn session_dedup_keeps_first_nonzero_opaque_identifier() {
        use shikifsrs_elden_ring::{
            character_sets::CharacterIdentity, network_sessions::NetworkSessionObservation,
        };
        let character = CharacterIdentity {
            address: 1,
            world_address: 2,
            native_identifier: 3,
            epoch: 1,
        };
        let session = |id, address| NetworkSessionObservation {
            character,
            holder_address: 4,
            session_address: address,
            native_identifier: id,
        };
        assert_eq!(
            unique_sessions(vec![
                session(0, 8),
                session(5, 16),
                session(5, 24),
                session(7, 32)
            ]),
            vec![session(5, 16), session(7, 32)]
        );
    }
}
