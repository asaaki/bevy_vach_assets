# Bevy Vach Assets [BVA]

**Warning! Very basic, very opinionated, rough edges everywhere!**

_This project is written to work for me and my ideas, it might not suite your needs._

## Bevy plugin

A plugin to use an archive file for your assets in your Bevy projects.

It builds on [vach] for the archive format which provides compression and encryption.

Both of those features are non-negotiatable defaults in `BVA`.

### Bevy compatibility

| bevy | bevy_vach_assets |
| ---- | -----------------|
| 0.12 | 0.1.*            |

### Limitations and constraints

As mentioned compression and encryption (plus signing) are enabled by default.
This does come with performance hits compared to native direct asset loading.
There are no benchmarks, but asset loading performance is not a goal right now.

It should work for WASM as a target, but only if the public/verifiying key and the archive are provides as byte arrays to the plugin.
Thus the best way is to embed the assets archive into the binary, making it quite similar to directly embed the assets with [bevy_embedded_assets]. The difference is that the embedded archive still benefits from the compression and encryption, making it harder to inspect the binary for asset data.

### Inspirations

* [bevy_assets_bundler] — great predecessor, but it sadly fell behind bevy's fast pace
* [bevy_embedded_assets] — basic structure used to build this plugin

## CLI

The `bva_cli` package provides a helper program (`bva` executable) to quickly generate keys and archive the assets.

Note: `vach` also provides a CLI, but `bva` is tailored to work with your Bevy project and the `bevy_vach_assets` plugin.

-----

## License

Licensed under either of

* Apache License, Version 2.0
   ([LICENSE-APACHE-2.0](LICENSE-Apache-2.0) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT License
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license,
shall be dual licensed as above,
without any additional terms or conditions.

<!-- links -->
[vach]: https://github.com/zeskeertwee/vach
[bevy_assets_bundler]: https://github.com/hanabi1224/bevy_assets_bundler
[bevy_embedded_assets]: https://github.com/vleue/bevy_embedded_assets
