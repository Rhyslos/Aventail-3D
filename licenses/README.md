# Third-party licenses

License texts for third-party material embedded in the program itself (fonts, icons, images). Every build ships with this folder.

## Fonts

The interface uses Inter, embedded from `crates/ui/assets/fonts/`, with the default fonts bundled with egui (through the `epaint_default_fonts` library) behind it as fallbacks:

| Font | Used for | License | File |
| --- | --- | --- | --- |
| Inter (Regular, SemiBold) | Interface text and headings | SIL Open Font License 1.1 | [Inter-OFL-1.1.txt](fonts/Inter-OFL-1.1.txt) |
| Ubuntu Light | Fallback text | Ubuntu Font Licence 1.0 | [Ubuntu-Font-Licence-1.0.txt](fonts/Ubuntu-Font-Licence-1.0.txt) |
| Hack | Monospace text | MIT, plus Bitstream Vera License | [Hack-MIT-and-Bitstream-Vera.txt](fonts/Hack-MIT-and-Bitstream-Vera.txt) |
| Noto Emoji | Emoji | SIL Open Font License 1.1 | [NotoEmoji-OFL-1.1.txt](fonts/NotoEmoji-OFL-1.1.txt) |
| emoji-icon-font | Symbols | MIT | [emoji-icon-font-MIT.txt](fonts/emoji-icon-font-MIT.txt) |

## Icons

The palette icons are original drawings made in code (`crates/ui/src/icons.rs`), part of the app itself under its own license. No third-party icons are embedded.

## Code libraries

The app's Rust dependencies use permissive licenses (MIT, Apache 2.0, BSD, ISC, Zlib, Boost, CC0, Unicode), checked on every change by `cargo deny` (see `deny.toml`). A complete generated list of every library and its license is planned for the first public release.
