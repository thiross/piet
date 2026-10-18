# Automatic backend selection for [Piet][]

Automatically chooses the appropriate implementation of the [Piet][] 2D graphics API for the current platform.

By default, the backend is [piet-direct2d][] on Windows, [piet-coregraphics][] on
macOS and iOS, and [piet-cairo][] on Linux, OpenBSD, FreeBSD, and NetBSD. The
[piet-web][] backend is selected when targeting `wasm32`.

The optional `tui` feature selects the terminal backend from
[piet-tui](https://crates.io/crates/piet-tui). Because Cargo features are
additive, disable default features to avoid pulling in the native GUI
dependencies:

```toml
piet-common = { version = "0.8", default-features = false, features = ["tui"] }
```

The terminal backend renders into a `ratatui-core::buffer::Buffer` and supports
rich text, fills, strokes, clipping, transforms, gradients, and images. Its
`BitmapTarget` exposes the underlying buffer, while pixel extraction and PNG
export return `Error::NotSupported`. Enable the `tui-big-text` feature to use
the terminal display-font layouts.

## Minimum supported Rust Version (MSRV)

This version of Piet has been verified to compile with **Rust 1.92** and later.

Future versions of Piet might increase the Rust version requirement.
It will not be treated as a breaking change and as such can even happen with small patch releases.

<details>
<summary>Click here if compiling fails.</summary>

As time has passed, some of Piet's dependencies could have released versions with a higher Rust requirement.
If you encounter a compilation issue due to a dependency and don't want to upgrade your Rust toolchain, then you could downgrade the dependency.

```sh
# Use the problematic dependency's name and version
cargo update -p package_name --precise 0.1.1
```

</details>

## Community

Discussion of Piet development happens in the [Linebender Zulip](https://xi.zulipchat.com/), specifically the [#piet stream](https://xi.zulipchat.com/#narrow/channel/259397-piet).
All public content can be read without logging in.

Contributions are welcome by pull request.
The [Rust code of conduct] applies.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache 2.0 license, shall be licensed as noted in the [License](#license) section, without any additional terms or conditions.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

[Piet]: https://crates.io/crates/piet
[piet-direct2d]: https://crates.io/crates/piet-direct2d
[piet-cairo]: https://crates.io/crates/piet-cairo
[piet-web]: https://crates.io/crates/piet-web
[piet-coregraphics]: https://crates.io/crates/piet-coregraphics
[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
