# ER Torrent Skin Slots

[简体中文](README.md) | [English](README_EN.md)

A configurable DLL that adds custom Torrent appearance slots to ELDEN RING.
The default catalog contains four official slots and three custom-slot examples.
More mounts can be added through the INI without rebuilding the DLL for each slot.

Environment: Windows x64, WW ELDEN RING 1.17.1 (EXE product version `2.7.1.0`).

This is an unofficial modding tool and is not affiliated with FromSoftware,
Bandai Namco, or Mod Engine 2. Disable Easy Anti-Cheat and use it only offline
or in environments where mods are explicitly allowed. Do not connect to the
official online service while using it.

## Features and responsibility boundary

The DLL provides:

- a configurable Torrent appearance catalog in `er_torrent_skin_slots.ini`;
- appearance selection using the event flag assigned to each slot;
- the four official slots alongside additional custom mounts;
- normal mounted movement and terrain traversal for added mounts;
- optional logs for loading, selection, and appearance resolution.

The DLL does **not**:

- modify `m00_00_00_00.talkesdbnd.dcx` or create a Torrent selection menu;
- initialize, clear, or persist additional event flags;
- merge `regulation.bin` or create `NpcParam` / `RideParam` rows;
- install character models, textures, animations, audio, FMG text, or other
  assets;
- resolve Param, Talk, Event, or loose-file conflicts with other mods.

This separation keeps the DLL focused on expanding the low-level slot catalog.
Each mod author remains in control of the actual mounts, menu, game data, and
resources.

## v0.1.2 fixes

- Fixed custom mounts failing to clamber over small elevations and low obstacles.
- Fixed the mount disappearing and the player becoming stuck in a mounted animation with impaired movement after quitting to the title screen while falling on a custom mount, continuing the game, and jumping.

## Shipped slot catalog

The included INI defines the four official slots and three
custom-slot examples:

| Selector | Event flag | Appearance `NpcParam` | Purpose |
|---:|---:|---:|---|
| 0 | 6700 | 80020000 | Official slot |
| 1 | 6701 | 80030000 | Official slot |
| 2 | 6702 | 80040000 | Official slot |
| 3 | 6703 | 80050000 | Official slot |
| 4 | 6704 | 80060000 | Extended slot example |
| 5 | 6705 | 80070000 | Extended slot example |
| 6 | 6706 | 80080000 | Extended slot example |

The third column must contain the complete `NpcParam` row ID. The DLL only
returns this ID. Which `RideParam` it references and which character resources
are ultimately loaded are determined by your own `regulation.bin` and mod
files.

## Installation

### 1. Prepare the DLL and INI

Place `er_torrent_skin_slots.dll` and `er_torrent_skin_slots.ini` in the same directory:

```text
your-mod/
├─ er_torrent_skin_slots.dll
└─ er_torrent_skin_slots.ini
```

The INI is a sidecar configuration file and is read once when the DLL starts.
Completely exit and restart the game after changing it.

### 2. Load the DLL

For example, with Mod Engine 2, add the DLL path to `external_dlls` under the
corresponding `[modengine]` section. This example assumes the DLL is in the
launcher directory:

```toml
[modengine]
external_dlls = [
    "er_torrent_skin_slots.dll"
]
```

If the DLL is in a subdirectory, use the correct relative or absolute path for
your Mod Engine 2 launcher and configuration. Regardless of where the loader
finds the DLL, the INI must be beside the DLL that is actually loaded.

### 3. Install your content mod

You must still install the following through Mod Engine 2 or your own packaging
system:

- a modified Talk menu;
- Event initialization and flag logic;
- a `regulation.bin` containing the additional `NpcParam` / `RideParam` rows;
- the models, materials, textures, animations, and other resources required by
  the new mounts;
- the corresponding menu text.

This repository does not distribute files extracted from the game.

## INI reference

```ini
# ER Torrent Skin Slots - WW Elden Ring 1.17.1
enabled = true
log_enabled = true

network_mode = off
packet_type = 238
heartbeat_frames = 120

# selector, event flag, appearance NpcParam ID
slot = 0, 6700, 80020000
slot = 1, 6701, 80030000
slot = 2, 6702, 80040000
slot = 3, 6703, 80050000
slot = 4, 6704, 80060000
slot = 5, 6705, 80070000
slot = 6, 6706, 80080000
```

### General options

| Key | Accepted values | Purpose |
|---|---|---|
| `enabled` | `true` / `false` | Installs no hooks when set to `false` |
| `log_enabled` | `true` / `false` | Controls `er_torrent_skin_slots.log` output |
| `network_mode` | `off` / `probe` / `on` | Experimental network mode; use `off` normally |
| `packet_type` | `0..255` | Candidate custom packet type used by `probe` |
| `heartbeat_frames` | Positive integer | Frame interval between `probe` heartbeats; clamped to at least 1 |
| `slot` | Three decimal integers | `selector, event_flag, appearance_npc_param` |

Boolean values also accept `1/0`, `yes/no`, and `on/off`. Key names are case
sensitive. An unknown key, invalid number, or invalid catalog rejects the entire
configuration. In that case, the DLL installs no hooks and records the reason
in its log.

### `slot` rules

Each row uses this form:

```ini
slot = <selector>, <event_flag>, <appearance_npc_param>
```

The catalog must follow these rules:

1. `selector` is a unique integer from `0` through `255`. The catalog can
   therefore contain up to 256 entries: four official selectors and as many as
   252 custom selectors from `4` through `255`.
2. Every `event_flag` must be unique. Choose flags that do not conflict with
   the game or another mod.
3. `appearance_npc_param` must be a positive integer that identifies a real
   `NpcParam` row in the loaded game data.
4. Selectors `0..3` must all remain present, with appearance IDs `80020000`,
   `80030000`, `80040000`, and `80050000`, respectively. Keeping their default
   flags `6700..6703` is strongly recommended.
5. As soon as the INI contains one or more `slot` rows, those rows **replace the
   complete built-in catalog**. They are not appended to it. Keep every
   existing row when adding another slot.
6. If several catalog flags are ON at the same time, the lowest numeric
   selector wins. Correct menu logic should keep exactly one selection flag ON.

If the INI is missing, or if it contains no `slot` rows at all, the DLL uses its
built-in seven-entry catalog.

## Adding more mounts without rebuilding the DLL

No DLL rebuild is required. This example adds an eighth entry after the shipped
seven:

```ini
slot = 0, 6700, 80020000
slot = 1, 6701, 80030000
slot = 2, 6702, 80040000
slot = 3, 6703, 80050000
slot = 4, 6704, 80060000
slot = 5, 6705, 80070000
slot = 6, 6706, 80080000
slot = 7, 6707, 80090000
```

Then connect the content side of the mod:

1. Create `NpcParam` row `80090000` in `regulation.bin` and configure its
   `RideParam` and other references correctly.
2. Install the character resources used by the new mount.
3. Add an entry to the Talk menu. When selected, it should clear every slot
   flag in the catalog and then enable only `6707`.
4. Handle the new flag's default value, initialization, and any required
   persistence in Event logic.
5. Add the relevant FMG text and resolve Talk, Event, Param, and file conflicts
   with other mods.
6. Restart the game and check the log for `selector=7` and
   `appearance=80090000`.

Repeat the process for further mounts, assigning each one a new selector, a
unique flag, and a valid `NpcParam` ID. Custom selectors do not have to be
contiguous, although contiguous numbering is usually easier to maintain.

## Talk / Event contract

The recommended logic for every menu choice is deliberately simple:

```text
turn OFF every Torrent selection flag listed in the catalog
    -> turn ON only the flag assigned to this option
    -> close the menu or refresh its state
```

Do not make new entries depend on a second mixed state machine or leftover
flags. The DLL only scans the flags listed in the INI; it does not understand
the meaning of the Talk state machine and does not automatically repair
multiple active flags. If no catalog flag is ON, the selector falls back to
`0`.

## Network modes

| Mode | Current behavior |
|---|---|
| `off` | Default and recommended; only the local expanded-slot logic is enabled |
| `probe` | Attempts to send, receive, and log custom state packets when a supported co-op mod DLL is detected; remote state is not used for appearance creation |
| `on` | Reserved; remote appearance synchronization is not enabled, while local features remain available |

`probe` messages contain a hash of the entire slot catalog. A state message is
rejected when the two catalogs differ, but a matching hash does not mean remote
appearance synchronization is ready. For multiplayer use, all participants
should still install the same DLL, INI, Talk, Event, Param, and mount resources.
This version does not promise correct remote display of extended mounts.

## Logging and troubleshooting

The default log is created beside the DLL:

```text
er_torrent_skin_slots.log
```

Useful messages from a normal startup include:

```text
loaded version=0.1.2 enabled=true ... slots=...
safety gate accepted WW 2.7.1.0 ...
hooks installed ...
local selector changed ... selector=7 ...
appearance resolve ... effective_selector=7 appearance=80090000
```

Common problems:

- **No log is created:** confirm that the loader actually loads the DLL, the
  INI is beside it, and `log_enabled = true`.
- **`configuration rejected`:** use the reported line or catalog error to find
  an unknown key, duplicate selector, duplicate flag, missing official slot, or
  invalid `NpcParam` ID.
- **`safety gate rejected executable`:** the executable version or a critical
  signature is unsupported. The DLL fails closed and does not attempt to hook
  unknown addresses.
- **The menu works but the selector does not change:** make sure the Talk logic
  sets the exact flag listed in the INI and clears every other slot flag.
- **The selector is correct but the model is wrong or missing:** inspect the
  `NpcParam`, `RideParam`, and resource files first. The DLL does not generate
  or install them.
- **Problems occur only with another DLL enabled:** both mods may hook the same
  game logic. Disable other DLLs one at a time to locate the conflict, and
  confirm that every mod supports the installed game version.

## License

ER Torrent Skin Slots is released under the [MIT License](LICENSE). Third-party
dependencies retain their own licenses.
