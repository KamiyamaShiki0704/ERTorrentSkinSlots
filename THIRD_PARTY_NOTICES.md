# Third-party notices

ER Torrent Skin Slots depends on selected crates from
[`eldenring-rs` / FromSoftware.rs](https://github.com/vswarte/eldenring-rs):

- `eldenring` 0.14.0;
- `fromsoftware-shared` 0.14.0, including its transitive workspace crates.

Cargo fetches these crates from the immutable upstream commit
[`eae96dfec94fd9cf6f9d24813c8d08f72019f243`](https://github.com/vswarte/eldenring-rs/commit/eae96dfec94fd9cf6f9d24813c8d08f72019f243),
which includes the ELDEN RING 1.17 update. No FromSoftware.rs source is vendored
in this repository.

FromSoftware.rs is available under the author's choice of:

- [Apache License, Version 2.0](https://github.com/vswarte/eldenring-rs/blob/eae96dfec94fd9cf6f9d24813c8d08f72019f243/LICENSE-ASL2); or
- [MIT License](https://github.com/vswarte/eldenring-rs/blob/eae96dfec94fd9cf6f9d24813c8d08f72019f243/LICENSE-MIT).

The dependency source and its license texts remain the work of their respective
authors. The root `LICENSE` applies to ER Torrent Skin Slots itself and does not
replace any third-party license.

Additional Rust dependencies are declared in `Cargo.toml`, resolved exactly by
`Cargo.lock`, and remain subject to their own license terms.
