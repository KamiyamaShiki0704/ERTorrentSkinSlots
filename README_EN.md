# ER Torrent Skin Slots

[简体中文](README.md) | [English](README_EN.md)

A configurable DLL that adds true expansion slots for Torrent appearances in
ELDEN RING 1.17. It works around the game's original four-value (`0..3`)
appearance selector and maps additional selectors to event flags and appearance
`NpcParam` rows through an INI catalog.

> Currently supported: WW `eldenring.exe` product versions `2.7.0.0` / `2.7.1.0` (game
> versions 1.17 / 1.17.1).
>
> This is an unofficial modding tool and is not affiliated with FromSoftware,
> Bandai Namco, or Mod Engine 2. Disable Easy Anti-Cheat and use it only offline
> or in environments where mods are explicitly allowed. Do not connect to the
> official online service while using it.

## Features and responsibility boundary

The DLL is responsible for:

- loading an extensible Torrent slot catalog from
  `er_torrent_skin_slots.ini`;
- selecting the local slot from the event flag assigned to each catalog entry;
- mapping that selector to an appearance `NpcParam` ID when Torrent is first
  created and its appearance is resolved;
- extending native character recognition and mount
  physics recognition from the INI catalog, so configured mounts enter the
  original mounted-animation, cleanup, and physics branches;
- validating the executable version and critical code signatures before
  installing any hook;
- logging initialization, selection changes, and appearance resolution beside
  the DLL.

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

## Validation status

- v0.1.2 fixes configured mounts being excluded from native character recognition
  and mount physics initialization. On 2026-10-13, the project owner confirmed
  both reported c8008 issues were fixed in a local 1.17.1 mod without Seamless
  Co-op: small-ledge traversal, and a disappearing horse / stuck mounted player
  after falling, quitting to title, continuing, and jumping. This gameplay
  acceptance covers the same repair logic in `0.1.2-test.1`; v0.1.2 only removes
  the test-version suffix and updates documentation, without changing the fix.
  Automated tests execute actual 1.17.1 EXE code before and after the hooks,
  covering recognition, animation selection, physics initialization, and hook
  rollback. The old 1.17 EXE was not regression-tested in this round, and this
  local acceptance does not establish multiplayer compatibility. The INI format is
  unchanged; recognition follows configured extended slots, not three fixed IDs.
  With `log_enabled = true`, up to 64 changed recovery-boundary snapshots are
  logged. These are read-only; no saved position, HP, flags, or link pointers are
  rewritten.

- v0.1.1 adds WW 1.17.1 (EXE 2.7.1.0) support with an updated appearance
  resolver address, selector-refresh signature, and pinned compatible fsrs fork.
  Static EXE checks, safety-gate corruption tests, and compilation pass. The
  separate 1.17 profile is retained; see v0.1.2 for the new mount behavior fixes.

- Local use: selectors `4`, `5`, and `6` from the shipped catalog have passed
  in-game testing with three additional mounts.
- Official slots: selectors `0..3` retain the game's original appearance
  `NpcParam` mappings.
- Seamless Co-op custom synchronization: still experimental. `probe` only
  transmits, receives, and logs catalog state; it does not route remote player
  identity into appearance creation. `on` is reserved and rejected in version
  `0.1.0`.

Keep `network_mode = off` when you only need local expanded mounts.

## Shipped slot catalog

The included INI defines the four official slots and three locally tested
extended slots:

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

After building from source, locate:

```text
target/x86_64-pc-windows-msvc/release/er_torrent_skin_slots.dll
```

Place it beside the `er_torrent_skin_slots.ini` from the repository root:

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
# ER Torrent Skin Slots - WW Elden Ring 1.17 / executable 2.7.0.0
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
| `on` | Reserved and rejected in version `0.1.0`; local hooks remain available |

`probe` messages contain a hash of the entire slot catalog. A state message is
rejected when the two catalogs differ, but a matching hash does not mean remote
appearance synchronization is ready. For multiplayer testing, all participants
should still install the same DLL, INI, Talk, Event, Param, and mount resources.
This version does not promise correct remote display of extended mounts.

## Logging and troubleshooting

The default log is created beside the DLL:

```text
er_torrent_skin_slots.log
```

Useful messages from a normal startup include:

```text
loaded enabled=true ... slots=...
safety gate accepted WW 2.7.0.0 ...
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

For selectors above `3`, the DLL mirrors `0` into the game's original two-bit
selection value while using the true extended selector at appearance
resolution. Another mod that only reads the vanilla two-bit value may therefore
see `0`; this is a compatibility boundary.

## Building from source

Requirements:

- 64-bit Windows;
- a stable Rust toolchain with Rust 2024 edition support;
- Visual Studio 2022 Build Tools with the MSVC C++ toolchain and Windows SDK;
- the `x86_64-pc-windows-msvc` Rust target.

```powershell
rustup update stable
rustup target add x86_64-pc-windows-msvc
cargo test -p er-torrent-skin-slots --locked
cargo build --release --locked --target x86_64-pc-windows-msvc
```

The release DLL is written to:

```text
target/x86_64-pc-windows-msvc/release/er_torrent_skin_slots.dll
```

This project uses the independently implemented private [ShikiFSRS](https://github.com/KamiyamaShiki0704/ShikiFSRS) SDK, pinned to `930f818431e72209b49d2e37265a1f8150fa5390`. The first build needs repository read access, Git credentials, and network access; subsequent builds can use Cargo's local cache. `Cargo.lock` records the same immutable revision. Offline validation covers ER WW 1.17.1; gameplay acceptance remains manual.

## Repository layout

```text
├─ src/                         DLL, configuration, hooks, and experimental network code
├─ Cargo.toml
├─ Cargo.lock
├─ er_torrent_skin_slots.ini    Editable shipped catalog
├─ README.md                     Chinese documentation
├─ README_EN.md                  English documentation
├─ LICENSE                      MIT License for this project
└─ THIRD_PARTY_NOTICES.md       Third-party attribution
```

## License

ER Torrent Skin Slots is released under the [MIT License](LICENSE). Third-party
dependencies fetched by Cargo retain their original licensing; see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
