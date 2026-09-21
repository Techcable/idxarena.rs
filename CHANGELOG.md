# Changelog

Notable changes to this project should be documented in this file.
Make sure it is up to date before performing a release.

This project follows the [Keep a Changelog](https://keepachangelog.com/en/2.0.0/) format wherever that is reasonable.

The "title" of each release should be its first line.
A title is required for publishing a github release, so all versions should have one.

Most changes include the relevant [jj](https://jj-vcs.dev) change ids in parens. An example of a change id is wuoxvnsw.

## Unreleased

## 0.1.0
Initial release.

For the time being, the [`ArenaAllocFrom`] and [`Internable`] traits are sealed
and cannot be implemented by outside types.
This restriction will likely be lifted in the future.

The MSRV is currently 1.85, as that is what hashbrown v0.17 requires.
An attempt was made to support Rust 1.71, but that ended up to be too difficult (see xurrlqxl, zvmrzupr, ktuqvtzv).

[`ArenaAllocFrom`]: https://docs.rs/idxarena/0.1.0/idxarena/trait.ArenaAllocFrom.html
[`Internable`]: https://docs.rs/idxarena/0.1.0/idxarena/intern/trait.Internable.html


Originally based on some internal code of [DuckLogic](https://ducklogic.org/).
