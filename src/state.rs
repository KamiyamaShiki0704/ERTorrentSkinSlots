use std::collections::HashMap;

use crate::{catalog::SlotCatalog, protocol::WireMessage};

pub type PeerId = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalSelection {
    pub selector: u8,
    pub vanilla_mirror: u8,
}

impl LocalSelection {
    pub fn new(selector: u8) -> Self {
        Self {
            selector,
            vanilla_mirror: if selector <= 3 { selector } else { 0 },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerSelection {
    pub selector: u8,
    pub sequence: u32,
}

#[derive(Debug, Default)]
pub struct PeerRegistry {
    peers: HashMap<PeerId, PeerSelection>,
}

impl PeerRegistry {
    pub fn accept(
        &mut self,
        peer: PeerId,
        message: WireMessage,
        catalog: &SlotCatalog,
    ) -> Result<PeerSelection, RejectReason> {
        if message.catalog_hash != catalog.hash() {
            return Err(RejectReason::CatalogMismatch {
                expected: catalog.hash(),
                actual: message.catalog_hash,
            });
        }
        if !catalog.contains_selector(message.selector) {
            return Err(RejectReason::UnknownSelector(message.selector));
        }

        if let Some(current) = self.peers.get(&peer)
            && !is_newer_sequence(message.sequence, current.sequence)
        {
            return Err(RejectReason::StaleSequence {
                current: current.sequence,
                received: message.sequence,
            });
        }

        let selection = PeerSelection {
            selector: message.selector,
            sequence: message.sequence,
        };
        self.peers.insert(peer, selection);
        Ok(selection)
    }

    pub fn get(&self, peer: PeerId) -> Option<PeerSelection> {
        self.peers.get(&peer).copied()
    }

    pub fn remove(&mut self, peer: PeerId) {
        self.peers.remove(&peer);
    }
}

pub fn is_newer_sequence(candidate: u32, current: u32) -> bool {
    let distance = candidate.wrapping_sub(current);
    distance != 0 && distance < 0x8000_0000
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectReason {
    CatalogMismatch { expected: u64, actual: u64 },
    UnknownSelector(u8),
    StaleSequence { current: u32, received: u32 },
}

#[cfg(test)]
mod tests {
    use crate::protocol::MessageKind;

    use super::*;

    fn message(catalog: &SlotCatalog, sequence: u32, selector: u8) -> WireMessage {
        WireMessage {
            kind: MessageKind::State,
            catalog_hash: catalog.hash(),
            sequence,
            selector,
        }
    }

    #[test]
    fn mirror_is_zero_only_for_extended_slots() {
        assert_eq!(LocalSelection::new(3).vanilla_mirror, 3);
        assert_eq!(LocalSelection::new(4).vanilla_mirror, 0);
        assert_eq!(LocalSelection::new(255).vanilla_mirror, 0);
    }

    #[test]
    fn peer_registry_rejects_catalog_selector_and_stale_state() {
        let catalog = SlotCatalog::default_er117();
        let mut registry = PeerRegistry::default();
        registry
            .accept(10, message(&catalog, 5, 4), &catalog)
            .unwrap();

        assert!(matches!(
            registry.accept(10, message(&catalog, 5, 5), &catalog),
            Err(RejectReason::StaleSequence { .. })
        ));
        assert!(matches!(
            registry.accept(10, message(&catalog, 4, 5), &catalog),
            Err(RejectReason::StaleSequence { .. })
        ));
        assert!(matches!(
            registry.accept(11, message(&catalog, 1, 99), &catalog),
            Err(RejectReason::UnknownSelector(99))
        ));

        let mut wrong_catalog = message(&catalog, 6, 5);
        wrong_catalog.catalog_hash ^= 1;
        assert!(matches!(
            registry.accept(10, wrong_catalog, &catalog),
            Err(RejectReason::CatalogMismatch { .. })
        ));
        assert_eq!(registry.get(10).unwrap().selector, 4);
    }

    #[test]
    fn sequence_wrap_is_accepted() {
        assert!(is_newer_sequence(0, u32::MAX));
        assert!(!is_newer_sequence(u32::MAX, 0));
    }
}
