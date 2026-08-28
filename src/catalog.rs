use std::collections::HashSet;

use crate::protocol::PROTOCOL_VERSION;

pub const OFFICIAL_APPEARANCE_IDS: [i32; 4] = [80_020_000, 80_030_000, 80_040_000, 80_050_000];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub selector: u8,
    pub event_flag: u32,
    pub appearance_npc_param: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotCatalog {
    slots: Vec<Slot>,
    hash: u64,
}

impl SlotCatalog {
    pub fn new(mut slots: Vec<Slot>) -> Result<Self, CatalogError> {
        if slots.is_empty() {
            return Err(CatalogError::Empty);
        }

        slots.sort_unstable_by_key(|slot| slot.selector);

        let mut selectors = HashSet::new();
        let mut flags = HashSet::new();
        for slot in &slots {
            if !selectors.insert(slot.selector) {
                return Err(CatalogError::DuplicateSelector(slot.selector));
            }
            if !flags.insert(slot.event_flag) {
                return Err(CatalogError::DuplicateEventFlag(slot.event_flag));
            }
            if slot.appearance_npc_param <= 0 {
                return Err(CatalogError::InvalidAppearanceId {
                    selector: slot.selector,
                    value: slot.appearance_npc_param,
                });
            }
        }

        for (selector, expected) in OFFICIAL_APPEARANCE_IDS.into_iter().enumerate() {
            let Some(slot) = slots.iter().find(|slot| slot.selector == selector as u8) else {
                return Err(CatalogError::MissingOfficialSlot(selector as u8));
            };
            if slot.appearance_npc_param != expected {
                return Err(CatalogError::OfficialSlotRemapped {
                    selector: selector as u8,
                    expected,
                    actual: slot.appearance_npc_param,
                });
            }
        }

        let hash = catalog_hash(&slots);
        Ok(Self { slots, hash })
    }

    pub fn default_er117() -> Self {
        Self::new(vec![
            Slot {
                selector: 0,
                event_flag: 6700,
                appearance_npc_param: 80_020_000,
            },
            Slot {
                selector: 1,
                event_flag: 6701,
                appearance_npc_param: 80_030_000,
            },
            Slot {
                selector: 2,
                event_flag: 6702,
                appearance_npc_param: 80_040_000,
            },
            Slot {
                selector: 3,
                event_flag: 6703,
                appearance_npc_param: 80_050_000,
            },
            Slot {
                selector: 4,
                event_flag: 6704,
                appearance_npc_param: 80_060_000,
            },
            Slot {
                selector: 5,
                event_flag: 6705,
                appearance_npc_param: 80_070_000,
            },
            Slot {
                selector: 6,
                event_flag: 6706,
                appearance_npc_param: 80_080_000,
            },
        ])
        .expect("the built-in ER 1.17 catalog must be valid")
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    pub fn contains_selector(&self, selector: u8) -> bool {
        self.appearance(selector).is_some()
    }

    pub fn appearance(&self, selector: u8) -> Option<i32> {
        self.slots
            .binary_search_by_key(&selector, |slot| slot.selector)
            .ok()
            .map(|index| self.slots[index].appearance_npc_param)
    }

    pub fn select_by_flag(&self, mut is_set: impl FnMut(u32) -> bool) -> u8 {
        self.slots
            .iter()
            .find(|slot| is_set(slot.event_flag))
            .map_or(0, |slot| slot.selector)
    }

    pub fn hash(&self) -> u64 {
        self.hash
    }
}

fn catalog_hash(slots: &[Slot]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    let mut hash = OFFSET_BASIS;
    for byte in PROTOCOL_VERSION
        .to_le_bytes()
        .into_iter()
        .chain((slots.len() as u32).to_le_bytes())
        .chain(slots.iter().flat_map(|slot| {
            [slot.selector]
                .into_iter()
                .chain(slot.event_flag.to_le_bytes())
                .chain(slot.appearance_npc_param.to_le_bytes())
        }))
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogError {
    Empty,
    DuplicateSelector(u8),
    DuplicateEventFlag(u32),
    InvalidAppearanceId {
        selector: u8,
        value: i32,
    },
    MissingOfficialSlot(u8),
    OfficialSlotRemapped {
        selector: u8,
        expected: i32,
        actual: i32,
    },
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "slot catalog is empty"),
            Self::DuplicateSelector(value) => write!(f, "duplicate selector {value}"),
            Self::DuplicateEventFlag(value) => write!(f, "duplicate event flag {value}"),
            Self::InvalidAppearanceId { selector, value } => {
                write!(f, "selector {selector} has invalid appearance ID {value}")
            }
            Self::MissingOfficialSlot(value) => write!(f, "official selector {value} is missing"),
            Self::OfficialSlotRemapped {
                selector,
                expected,
                actual,
            } => write!(
                f,
                "official selector {selector} must remain {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for CatalogError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_selector_wins_when_multiple_flags_are_set() {
        let catalog = SlotCatalog::default_er117();
        let selected = catalog.select_by_flag(|flag| flag == 6704 || flag == 6706);
        assert_eq!(selected, 4);
    }

    #[test]
    fn catalog_hash_changes_with_mapping() {
        let baseline = SlotCatalog::default_er117();
        let mut changed = baseline.slots().to_vec();
        changed.last_mut().unwrap().appearance_npc_param += 1;
        let changed = SlotCatalog::new(changed).unwrap();
        assert_ne!(baseline.hash(), changed.hash());
    }

    #[test]
    fn official_slots_cannot_be_remapped() {
        let mut slots = SlotCatalog::default_er117().slots().to_vec();
        slots[0].appearance_npc_param = 80_060_000;
        assert!(matches!(
            SlotCatalog::new(slots),
            Err(CatalogError::OfficialSlotRemapped { selector: 0, .. })
        ));
    }
}
