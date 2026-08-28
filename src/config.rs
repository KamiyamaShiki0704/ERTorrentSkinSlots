use std::{fs, path::Path};

use crate::catalog::{Slot, SlotCatalog};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NetworkMode {
    #[default]
    Off,
    Probe,
    On,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub enabled: bool,
    pub log_enabled: bool,
    pub network_mode: NetworkMode,
    pub packet_type: u8,
    pub heartbeat_frames: u32,
    pub catalog: SlotCatalog,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            log_enabled: true,
            network_mode: NetworkMode::Off,
            packet_type: 238,
            heartbeat_frames: 120,
            catalog: SlotCatalog::default_er117(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let Ok(contents) = fs::read_to_string(path) else {
            return Ok(Self::default());
        };
        Self::parse(&contents)
    }

    pub fn parse(contents: &str) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        let mut slots = Vec::new();

        for (line_index, raw_line) in contents.lines().enumerate() {
            let line_number = line_index + 1;
            let line = raw_line
                .split_once('#')
                .map_or(raw_line, |(value, _)| value)
                .trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(ConfigError::InvalidLine(line_number));
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "enabled" => config.enabled = parse_bool(value, line_number)?,
                "log_enabled" => config.log_enabled = parse_bool(value, line_number)?,
                "network_mode" => {
                    config.network_mode = match value.to_ascii_lowercase().as_str() {
                        "off" => NetworkMode::Off,
                        "probe" => NetworkMode::Probe,
                        "on" => NetworkMode::On,
                        _ => return Err(ConfigError::InvalidNetworkMode(line_number)),
                    }
                }
                "packet_type" => {
                    config.packet_type = value
                        .parse()
                        .map_err(|_| ConfigError::InvalidNumber(line_number))?
                }
                "heartbeat_frames" => {
                    config.heartbeat_frames = value
                        .parse::<u32>()
                        .map_err(|_| ConfigError::InvalidNumber(line_number))?
                        .max(1)
                }
                "slot" => slots.push(parse_slot(value, line_number)?),
                _ => return Err(ConfigError::UnknownKey(line_number, key.to_string())),
            }
        }

        if !slots.is_empty() {
            config.catalog = SlotCatalog::new(slots).map_err(ConfigError::Catalog)?;
        }
        Ok(config)
    }
}

fn parse_bool(value: &str, line: usize) -> Result<bool, ConfigError> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ConfigError::InvalidBool(line)),
    }
}

fn parse_slot(value: &str, line: usize) -> Result<Slot, ConfigError> {
    let values = value.split(',').map(str::trim).collect::<Vec<_>>();
    if values.len() != 3 {
        return Err(ConfigError::InvalidSlot(line));
    }
    Ok(Slot {
        selector: values[0]
            .parse()
            .map_err(|_| ConfigError::InvalidSlot(line))?,
        event_flag: values[1]
            .parse()
            .map_err(|_| ConfigError::InvalidSlot(line))?,
        appearance_npc_param: values[2]
            .parse()
            .map_err(|_| ConfigError::InvalidSlot(line))?,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfigError {
    InvalidLine(usize),
    UnknownKey(usize, String),
    InvalidBool(usize),
    InvalidNetworkMode(usize),
    InvalidNumber(usize),
    InvalidSlot(usize),
    Catalog(crate::catalog::CatalogError),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLine(line) => write!(f, "invalid config line {line}"),
            Self::UnknownKey(line, key) => write!(f, "unknown key {key:?} on line {line}"),
            Self::InvalidBool(line) => write!(f, "invalid bool on line {line}"),
            Self::InvalidNetworkMode(line) => write!(f, "invalid network_mode on line {line}"),
            Self::InvalidNumber(line) => write!(f, "invalid number on line {line}"),
            Self::InvalidSlot(line) => write!(f, "invalid slot on line {line}"),
            Self::Catalog(error) => write!(f, "invalid slot catalog: {error}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shipped_shape() {
        let text = r#"
enabled = true
network_mode = probe
packet_type = 238
slot = 0, 6700, 80020000
slot = 1, 6701, 80030000
slot = 2, 6702, 80040000
slot = 3, 6703, 80050000
slot = 4, 6704, 80060000
"#;
        let config = Config::parse(text).unwrap();
        assert_eq!(config.network_mode, NetworkMode::Probe);
        assert_eq!(config.catalog.appearance(4), Some(80_060_000));
    }
}
