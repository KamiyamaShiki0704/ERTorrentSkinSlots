use crate::{
    catalog::SlotCatalog,
    protocol::{MessageKind, WireMessage},
    state::{PeerId, PeerRegistry, RejectReason},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboundPacket {
    pub peer: PeerId,
    pub bytes: Vec<u8>,
}

pub trait NetworkAdapter {
    type Error;

    fn broadcast(&mut self, packet_type: u8, bytes: &[u8]) -> Result<(), Self::Error>;
    fn drain(&mut self, packet_type: u8) -> Result<Vec<InboundPacket>, Self::Error>;
}

pub type PeerReceiveResult = (PeerId, Result<u8, ReceiveReject>);

#[derive(Debug)]
pub struct SyncEngine {
    catalog: SlotCatalog,
    packet_type: u8,
    sequence: u32,
    local_selector: u8,
    peers: PeerRegistry,
}

impl SyncEngine {
    pub fn new(catalog: SlotCatalog, packet_type: u8, local_selector: u8) -> Self {
        Self {
            catalog,
            packet_type,
            sequence: 0,
            local_selector,
            peers: PeerRegistry::default(),
        }
    }

    pub fn publish<A: NetworkAdapter>(&mut self, adapter: &mut A) -> Result<(), A::Error> {
        self.sequence = self.sequence.wrapping_add(1);
        let packet = WireMessage {
            kind: MessageKind::State,
            catalog_hash: self.catalog.hash(),
            sequence: self.sequence,
            selector: self.local_selector,
        }
        .encode();
        adapter.broadcast(self.packet_type, &packet)
    }

    pub fn set_local_selector<A: NetworkAdapter>(
        &mut self,
        selector: u8,
        adapter: &mut A,
    ) -> Result<bool, A::Error> {
        if selector == self.local_selector {
            return Ok(false);
        }
        self.local_selector = selector;
        self.publish(adapter)?;
        Ok(true)
    }

    pub fn receive<A: NetworkAdapter>(
        &mut self,
        adapter: &mut A,
    ) -> Result<Vec<PeerReceiveResult>, A::Error> {
        let packets = adapter.drain(self.packet_type)?;
        Ok(packets
            .into_iter()
            .map(|packet| {
                let result = WireMessage::decode(&packet.bytes)
                    .map_err(ReceiveReject::Decode)
                    .and_then(|message| {
                        self.peers
                            .accept(packet.peer, message, &self.catalog)
                            .map(|state| state.selector)
                            .map_err(ReceiveReject::State)
                    });
                (packet.peer, result)
            })
            .collect())
    }

    pub fn peer_selector(&self, peer: PeerId) -> Option<u8> {
        self.peers.get(peer).map(|state| state.selector)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiveReject {
    Decode(crate::protocol::DecodeError),
    State(RejectReason),
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;

    #[derive(Debug, Default)]
    struct MemoryAdapter {
        sent: Vec<(u8, Vec<u8>)>,
        inbound: VecDeque<InboundPacket>,
    }

    impl NetworkAdapter for MemoryAdapter {
        type Error = ();

        fn broadcast(&mut self, packet_type: u8, bytes: &[u8]) -> Result<(), Self::Error> {
            self.sent.push((packet_type, bytes.to_vec()));
            Ok(())
        }

        fn drain(&mut self, packet_type: u8) -> Result<Vec<InboundPacket>, Self::Error> {
            let mut result = Vec::new();
            while let Some(packet) = self.inbound.pop_front() {
                result.push(packet);
            }
            assert_eq!(packet_type, 238);
            Ok(result)
        }
    }

    #[test]
    fn in_memory_transport_preserves_peer_identity_and_selector() {
        let catalog = SlotCatalog::default_er117();
        let mut sender = SyncEngine::new(catalog.clone(), 238, 4);
        let mut sender_adapter = MemoryAdapter::default();
        sender.publish(&mut sender_adapter).unwrap();

        let bytes = sender_adapter.sent.pop().unwrap().1;
        let mut receiver = SyncEngine::new(catalog, 238, 5);
        let mut receiver_adapter = MemoryAdapter::default();
        receiver_adapter.inbound.push_back(InboundPacket {
            peer: 76561198000000001,
            bytes,
        });

        let results = receiver.receive(&mut receiver_adapter).unwrap();
        assert_eq!(results, vec![(76561198000000001, Ok(4))]);
        assert_eq!(receiver.peer_selector(76561198000000001), Some(4));
    }
}
