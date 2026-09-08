# Third-party notices

ER Torrent Skin Slots depends on selected crates from
[`eldenring-rs` / FromSoftware.rs](https://github.com/KamiyamaShiki0704/fromsoftware-rs):

- `eldenring` 0.14.0;
- `fromsoftware-shared` 0.14.0, including its transitive workspace crates.

Cargo fetches these crates from the immutable upstream commit
[`7d4fdd66eda1a35284c170feb4b07b058c005f8f`](https://github.com/KamiyamaShiki0704/fromsoftware-rs/commit/7d4fdd66eda1a35284c170feb4b07b058c005f8f),
which includes ELDEN RING 1.17.1 compatibility. No FromSoftware.rs source is vendored
in this repository.

FromSoftware.rs is available under the author's choice of:

- [Apache License, Version 2.0](https://github.com/KamiyamaShiki0704/fromsoftware-rs/blob/7d4fdd66eda1a35284c170feb4b07b058c005f8f/LICENSE-ASL2); or
- [MIT License](https://github.com/KamiyamaShiki0704/fromsoftware-rs/blob/7d4fdd66eda1a35284c170feb4b07b058c005f8f/LICENSE-MIT).

The dependency source and its license texts remain the work of their respective
authors. The root `LICENSE` applies to ER Torrent Skin Slots itself and does not
replace any third-party license.

Additional Rust dependencies are declared in `Cargo.toml`, resolved exactly by
`Cargo.lock`, and remain subject to their own license terms.
