# Building Spotifast J on Windows

Use the Rust version in `rust-toolchain.toml`, Visual Studio C++ Build Tools
and the Windows SDK. Run from an x64 developer shell on `main`.

MilkDrop additionally needs CMake, Ninja, libclang, and vcpkg with
`glew:x64-windows-static`. Set `CMAKE_GENERATOR=Ninja`,
`VCPKG_INSTALLATION_ROOT` to the vcpkg root, and `LIBCLANG_PATH` to the folder
containing `libclang.dll`.

```powershell
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo build --locked --release
```

For UI development, `cargo run --locked --features demo -- --demo` uses sample
pins without signing into Spotify. Add `--no-default-features` for a development
build that omits MilkDrop and its native prerequisites.

The Windows installer is built with Inno Setup 6 from
`packaging/windows/spotifast.iss`; pass `Version`, `Arch=x86_64`, the absolute
`Binary` path and `OutputDir` on the ISCC command line. It installs **Spotifast J**
under the user's Programs folder with its own Windows uninstall identity.

The fork deliberately reuses upstream Spotifast's configuration and protected
credential store. Uninstall the original application before running this fork;
keep the configuration directories. No credentials belong in this repository.

CI uses the existing cross-platform upstream checks and retains the Windows
executable as an Actions artifact. Auto-update targets `jnxzu/my-spotifast` and
still requires signed checksums. A future fork release must configure its own
release signing key before self-updates can be published.
