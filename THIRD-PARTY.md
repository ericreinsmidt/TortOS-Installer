# Third-party notices

TortOS Installer's own code is licensed under the GNU General Public License,
version 3 or later; see `LICENSE`. Versions up to 2.0.0 were MIT.

It is built with **Tauri** (https://tauri.app) and the Rust crates below, which
keep their own licenses. Each app shows its window through the system's own web
view, which the installer doesn't carry: WebKit on macOS, Microsoft Edge
WebView2 on Windows, WebKitGTK on Linux.

The fonts in `ui/fonts` are **Josefin Sans** (regular, and thin italic for the
wordmark), by Santiago Orozco, under the SIL Open Font License 1.1
(`ui/fonts/OFL.txt`).

## Crates

Every crate compiled into an app, from Cargo's own records
(`tools/third-party.py`; `tools/release.sh` stops when this list no longer
matches the build). The ones that only run while building it aren't listed.

| Crate | Version | License | Repository | In |
|---|---|---|---|---|
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | https://github.com/oyvindln/adler2 | macOS, Windows, Linux |
| aho-corasick | 1.1.5 | Unlicense OR MIT | https://github.com/BurntSushi/aho-corasick | macOS, Windows, Linux |
| alloc-no-stdlib | 3.0.0 | BSD-3-Clause | https://github.com/dropbox/rust-alloc-no-stdlib | macOS, Windows, Linux |
| alloc-stdlib | 0.3.0 | BSD-3-Clause | https://github.com/dropbox/rust-alloc-no-stdlib | macOS, Windows, Linux |
| anyhow | 1.0.104 | MIT OR Apache-2.0 | https://github.com/dtolnay/anyhow | macOS, Windows, Linux |
| atk | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| atk-sys | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| base64 | 0.21.7 | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 | macOS, Windows, Linux |
| base64 | 0.22.1 | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 | macOS, Windows, Linux |
| base64 | 0.23.1 | MIT OR Apache-2.0 | https://github.com/marshallpierce/rust-base64 | macOS, Windows, Linux |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | https://github.com/contain-rs/bit-set | macOS, Windows, Linux |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | https://github.com/contain-rs/bit-vec | macOS, Windows, Linux |
| bitflags | 1.3.2 | MIT/Apache-2.0 | https://github.com/bitflags/bitflags | macOS, Windows, Linux |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | https://github.com/bitflags/bitflags | macOS, Windows, Linux |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils | macOS, Windows, Linux |
| block-buffer | 0.12.1 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils | macOS, Windows, Linux |
| block2 | 0.6.2 | MIT | https://github.com/madsmtm/objc2 | macOS |
| brotli | 9.0.0 | BSD-3-Clause AND MIT | https://github.com/dropbox/rust-brotli | macOS, Windows, Linux |
| brotli-decompressor | 6.0.1 | BSD-3-Clause/MIT | https://github.com/dropbox/rust-brotli-decompressor | macOS, Windows, Linux |
| byteorder | 1.5.0 | Unlicense OR MIT | https://github.com/BurntSushi/byteorder | macOS, Windows, Linux |
| bytes | 1.12.1 | MIT | https://github.com/tokio-rs/bytes | macOS, Windows, Linux |
| cairo-rs | 0.18.5 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| cairo-sys-rs | 0.18.2 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| camino | 1.2.6 | MIT OR Apache-2.0 | https://github.com/camino-rs/camino | macOS, Windows, Linux |
| cargo-platform | 0.1.9 | MIT OR Apache-2.0 | https://github.com/rust-lang/cargo | macOS, Windows, Linux |
| cargo_metadata | 0.19.2 | MIT | https://github.com/oli-obk/cargo_metadata | macOS, Windows, Linux |
| cfb | 0.14.0 | MIT | https://github.com/mdsteele/rust-cfb | macOS, Windows, Linux |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | https://github.com/rust-lang/cfg-if | macOS, Windows, Linux |
| const-oid | 0.10.2 | Apache-2.0 OR MIT | https://github.com/RustCrypto/formats | macOS, Windows, Linux |
| cookie | 0.18.2 | MIT OR Apache-2.0 | https://github.com/SergioBenitez/cookie-rs | macOS, Windows, Linux |
| core-foundation | 0.10.1 | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs | macOS |
| core-foundation-sys | 0.8.7 | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs | macOS |
| core-graphics | 0.25.0 | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs | macOS |
| core-graphics-types | 0.2.0 | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs | macOS |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils | macOS, Windows, Linux |
| cpufeatures | 0.3.1 | MIT OR Apache-2.0 | https://github.com/RustCrypto/utils | macOS, Windows, Linux |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 | https://github.com/srijs/rust-crc32fast | macOS, Windows, Linux |
| crossbeam-channel | 0.5.17 | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam | macOS, Windows, Linux |
| crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam | macOS, Windows, Linux |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits | macOS, Windows, Linux |
| crypto-common | 0.2.2 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits | macOS, Windows, Linux |
| cssparser | 0.37.0 | MPL-2.0 | https://github.com/servo/rust-cssparser | macOS, Windows, Linux |
| cssparser-macros | 0.7.1 | MPL-2.0 | https://github.com/servo/rust-cssparser | macOS, Windows, Linux |
| ctor | 1.0.13 | Apache-2.0 OR MIT | https://github.com/mmastrac/linktime | macOS, Windows, Linux |
| darling | 0.24.1 | MIT | https://github.com/TedDriggs/darling | macOS, Windows, Linux |
| darling_core | 0.24.1 | MIT | https://github.com/TedDriggs/darling | macOS, Windows, Linux |
| darling_macro | 0.24.1 | MIT | https://github.com/TedDriggs/darling | macOS, Windows, Linux |
| dbus | 0.9.12 | Apache-2.0/MIT | https://github.com/diwic/dbus-rs | Linux |
| deranged | 0.5.8 | MIT OR Apache-2.0 | https://github.com/jhpratt/deranged | macOS, Windows, Linux |
| derive_more | 2.1.1 | MIT | https://github.com/JelteF/derive_more | macOS, Windows, Linux |
| derive_more-impl | 2.1.1 | MIT | https://github.com/JelteF/derive_more | macOS, Windows, Linux |
| digest | 0.10.7 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits | macOS, Windows, Linux |
| digest | 0.11.3 | MIT OR Apache-2.0 | https://github.com/RustCrypto/traits | macOS, Windows, Linux |
| dirs | 6.0.0 | MIT OR Apache-2.0 | https://github.com/soc/dirs-rs | macOS, Windows, Linux |
| dirs | 7.0.0 | MIT OR Apache-2.0 | https://codeberg.org/dirs/dirs-rs | macOS, Windows, Linux |
| dirs-sys | 0.5.0 | MIT OR Apache-2.0 | https://github.com/dirs-dev/dirs-sys-rs | macOS, Windows, Linux |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT | https://github.com/madsmtm/objc2 | macOS |
| displaydoc | 0.2.7 | MIT OR Apache-2.0 | https://github.com/yaahc/displaydoc | macOS, Windows, Linux |
| dlopen2 | 0.8.2 | MIT | https://github.com/OpenByteDev/dlopen2 | Linux |
| dlopen2_derive | 0.4.3 | MIT | https://github.com/OpenByteDev/dlopen2 | Linux |
| dom_query | 0.28.0 | MIT | https://github.com/niklak/dom_query | macOS, Windows, Linux |
| dpi | 0.1.2 | Apache-2.0 AND MIT | https://github.com/rust-windowing/winit | macOS, Windows, Linux |
| dtoa | 1.0.11 | MIT OR Apache-2.0 | https://github.com/dtolnay/dtoa | macOS, Windows, Linux |
| dtoa-short | 0.3.5 | MPL-2.0 | https://github.com/upsuper/dtoa-short | macOS, Windows, Linux |
| dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | https://gitlab.com/kornelski/dunce | macOS, Windows, Linux |
| dyn-clone | 1.0.20 | MIT OR Apache-2.0 | https://github.com/dtolnay/dyn-clone | macOS, Windows, Linux |
| embed_plist | 1.2.2 | MIT OR Apache-2.0 | https://github.com/nvzqz/embed-plist-rs | macOS |
| equivalent | 1.0.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent | macOS, Windows, Linux |
| erased-serde | 0.4.10 | MIT OR Apache-2.0 | https://github.com/dtolnay/erased-serde | macOS, Windows, Linux |
| fastrand | 2.5.0 | Apache-2.0 OR MIT | https://github.com/smol-rs/fastrand | macOS, Windows, Linux |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | https://github.com/image-rs/fdeflate | macOS, Windows, Linux |
| field-offset | 0.3.6 | MIT OR Apache-2.0 | https://github.com/Diggsey/rust-field-offset | Linux |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | https://github.com/rust-lang/flate2-rs | macOS, Windows, Linux |
| fnv | 1.0.7 | Apache-2.0 / MIT | https://github.com/servo/rust-fnv | macOS, Windows, Linux |
| foldhash | 0.2.0 | Zlib | https://github.com/orlp/foldhash | macOS, Windows, Linux |
| foreign-types | 0.5.0 | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types | macOS |
| foreign-types-macros | 0.2.4 | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types | macOS |
| foreign-types-shared | 0.3.1 | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types | macOS |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | https://github.com/servo/rust-url | macOS, Windows, Linux |
| futures-channel | 0.3.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs | Linux |
| futures-core | 0.3.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs | Linux |
| futures-executor | 0.3.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs | Linux |
| futures-io | 0.3.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs | Linux |
| futures-macro | 0.3.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs | Linux |
| futures-task | 0.3.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs | Linux |
| futures-util | 0.3.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs | Linux |
| gdk | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| gdk-pixbuf | 0.18.5 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| gdk-pixbuf-sys | 0.18.0 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| gdk-sys | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| gdkwayland-sys | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| gdkx11 | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| gdkx11-sys | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| generic-array | 0.14.7 | MIT | https://github.com/fizyk20/generic-array.git | macOS, Windows, Linux |
| getrandom | 0.2.17 | MIT OR Apache-2.0 | https://github.com/rust-random/getrandom | macOS, Windows, Linux |
| getrandom | 0.3.4 | MIT OR Apache-2.0 | https://github.com/rust-random/getrandom | macOS, Windows, Linux |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | https://github.com/rust-random/getrandom | macOS, Windows, Linux |
| gio | 0.18.4 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| gio-sys | 0.18.1 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| glib | 0.18.5 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| glib-macros | 0.18.5 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| glib-sys | 0.18.1 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| glob | 0.3.4 | MIT OR Apache-2.0 | https://github.com/rust-lang/glob | macOS, Windows, Linux |
| gobject-sys | 0.18.0 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| gtk | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| gtk-sys | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| gtk3-macros | 0.18.2 | MIT | https://github.com/gtk-rs/gtk3-rs | Linux |
| hashbrown | 0.12.3 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown | macOS, Windows, Linux |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown | macOS, Windows, Linux |
| heck | 0.4.1 | MIT OR Apache-2.0 | https://github.com/withoutboats/heck | Linux |
| heck | 0.5.0 | MIT OR Apache-2.0 | https://github.com/withoutboats/heck | macOS, Windows, Linux |
| html5ever | 0.39.0 | MIT OR Apache-2.0 | https://github.com/servo/html5ever | macOS, Windows, Linux |
| http | 1.5.0 | MIT OR Apache-2.0 | https://github.com/hyperium/http | macOS, Windows, Linux |
| httparse | 1.10.1 | MIT OR Apache-2.0 | https://github.com/seanmonstar/httparse | macOS, Windows, Linux |
| hybrid-array | 0.4.15 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hybrid-array | macOS, Windows, Linux |
| ico | 0.5.0 | MIT | https://github.com/mdsteele/rust-ico | macOS, Windows, Linux |
| icu_collections | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| icu_locale_core | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| icu_normalizer | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| icu_properties | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| icu_properties_data | 2.3.0 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| icu_provider | 2.3.1 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| ident_case | 1.0.1 | MIT/Apache-2.0 | https://github.com/TedDriggs/ident_case | macOS, Windows, Linux |
| idna | 1.1.0 | MIT OR Apache-2.0 | https://github.com/servo/rust-url/ | macOS, Windows, Linux |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT | https://github.com/hsivonen/idna_adapter | macOS, Windows, Linux |
| indexmap | 1.9.3 | Apache-2.0 OR MIT | https://github.com/bluss/indexmap | macOS, Windows, Linux |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap | macOS, Windows, Linux |
| infer | 0.22.0 | MIT | https://github.com/bojand/infer | macOS, Windows, Linux |
| itoa | 1.0.18 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa | macOS, Windows, Linux |
| javascriptcore-rs | 1.1.2 | MIT | https://github.com/tauri-apps/javascriptcore-rs | Linux |
| javascriptcore-rs-sys | 1.1.1 | MIT | https://github.com/tauri-apps/javascriptcore-rs | Linux |
| json-patch | 4.2.0 | MIT/Apache-2.0 | https://github.com/idubrov/json-patch | macOS, Windows, Linux |
| jsonptr | 0.7.1 | MIT OR Apache-2.0 | https://github.com/chanced/jsonptr | macOS, Windows, Linux |
| keyboard-types | 0.8.3 | MIT OR Apache-2.0 | https://github.com/rust-windowing/keyboard-types | macOS, Windows, Linux |
| libc | 0.2.190 | MIT OR Apache-2.0 | https://github.com/rust-lang/libc | macOS, Windows, Linux |
| libdbus-sys | 0.2.7 | Apache-2.0/MIT | https://github.com/diwic/dbus-rs | Linux |
| litemap | 0.8.3 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| lock_api | 0.4.14 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot | macOS, Windows, Linux |
| log | 0.4.34 | MIT OR Apache-2.0 | https://github.com/rust-lang/log | macOS, Windows, Linux |
| lzma-rust2 | 0.20.1 | Apache-2.0 | https://github.com/hasenbanck/lzma-rust2/ | macOS, Windows, Linux |
| markup5ever | 0.39.0 | MIT OR Apache-2.0 | https://github.com/servo/html5ever | macOS, Windows, Linux |
| memchr | 2.8.3 | Unlicense OR MIT | https://github.com/BurntSushi/memchr | macOS, Windows, Linux |
| memoffset | 0.9.1 | MIT | https://github.com/Gilnaa/memoffset | Linux |
| mime | 0.3.17 | MIT OR Apache-2.0 | https://github.com/hyperium/mime | macOS, Windows, Linux |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide | macOS, Windows, Linux |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide | macOS, Windows, Linux |
| muda | 0.20.0 | Apache-2.0 OR MIT | https://github.com/tauri-apps/muda | macOS, Windows, Linux |
| new_debug_unreachable | 1.0.6 | MIT | https://github.com/mbrubeck/rust-debug-unreachable | macOS, Windows, Linux |
| num-conv | 0.2.2 | MIT OR Apache-2.0 | https://github.com/jhpratt/num-conv | macOS, Windows, Linux |
| objc2 | 0.6.5 | MIT | https://github.com/madsmtm/objc2 | macOS |
| objc2-app-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://github.com/madsmtm/objc2 | macOS |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://github.com/madsmtm/objc2 | macOS |
| objc2-encode | 4.1.0 | MIT | https://github.com/madsmtm/objc2 | macOS |
| objc2-exception-helper | 0.1.1 | Zlib OR Apache-2.0 OR MIT | https://github.com/madsmtm/objc2 | macOS |
| objc2-foundation | 0.3.2 | MIT | https://github.com/madsmtm/objc2 | macOS |
| objc2-quartz-core | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://github.com/madsmtm/objc2 | macOS |
| objc2-web-kit | 0.3.2 | Zlib OR Apache-2.0 OR MIT | https://github.com/madsmtm/objc2 | macOS |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | https://github.com/matklad/once_cell | macOS, Windows, Linux |
| option-ext | 0.2.0 | MPL-2.0 | https://github.com/soc/option-ext.git | macOS, Windows, Linux |
| pango | 0.18.3 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| pango-sys | 0.18.0 | MIT | https://github.com/gtk-rs/gtk-rs-core | Linux |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot | macOS, Windows, Linux |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot | macOS, Windows, Linux |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | https://github.com/servo/rust-url/ | macOS, Windows, Linux |
| phf | 0.13.1 | MIT | https://github.com/rust-phf/rust-phf | macOS, Windows, Linux |
| phf_generator | 0.13.1 | MIT | https://github.com/rust-phf/rust-phf | macOS, Windows, Linux |
| phf_macros | 0.13.1 | MIT | https://github.com/rust-phf/rust-phf | macOS, Windows, Linux |
| phf_shared | 0.13.1 | MIT | https://github.com/rust-phf/rust-phf | macOS, Windows, Linux |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | https://github.com/taiki-e/pin-project-lite | macOS, Windows, Linux |
| plist | 1.10.1 | MIT | https://github.com/ebarnard/rust-plist/ | macOS, Windows, Linux |
| png | 0.17.16 | MIT OR Apache-2.0 | https://github.com/image-rs/image-png | macOS, Windows, Linux |
| png | 0.18.1 | MIT OR Apache-2.0 | https://github.com/image-rs/image-png | macOS, Windows, Linux |
| potential_utf | 0.1.6 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| powerfmt | 0.2.1 | MIT OR Apache-2.0 | https://github.com/jhpratt/powerfmt | macOS, Windows, Linux |
| precomputed-hash | 0.1.1 | MIT | https://github.com/emilio/precomputed-hash | macOS, Windows, Linux |
| proc-macro-crate | 1.3.1 | MIT OR Apache-2.0 | https://github.com/bkchr/proc-macro-crate | Linux |
| proc-macro-crate | 2.0.2 | MIT OR Apache-2.0 | https://github.com/bkchr/proc-macro-crate | Linux |
| proc-macro-error | 1.0.4 | MIT OR Apache-2.0 | https://gitlab.com/CreepySkeleton/proc-macro-error | Linux |
| proc-macro-error-attr | 1.0.4 | MIT OR Apache-2.0 | https://gitlab.com/CreepySkeleton/proc-macro-error | Linux |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 | macOS, Windows, Linux |
| quick-xml | 0.42.0 | MIT | https://github.com/tafia/quick-xml | macOS, Windows, Linux |
| quote | 1.0.47 | MIT OR Apache-2.0 | https://github.com/dtolnay/quote | macOS, Windows, Linux |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | https://github.com/rust-windowing/raw-window-handle | macOS, Windows, Linux |
| regex | 1.13.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex | macOS, Windows, Linux |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex | macOS, Windows, Linux |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex | macOS, Windows, Linux |
| ring | 0.17.14 | Apache-2.0 AND ISC | https://github.com/briansmith/ring | macOS, Windows, Linux |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | https://github.com/rust-lang/rustc-hash | macOS, Windows, Linux |
| rustls | 0.23.45 | Apache-2.0 OR ISC OR MIT | https://github.com/rustls/rustls | macOS, Windows, Linux |
| rustls-pki-types | 1.15.1 | MIT OR Apache-2.0 | https://github.com/rustls/pki-types | macOS, Windows, Linux |
| rustls-webpki | 0.103.15 | ISC | https://github.com/rustls/webpki | macOS, Windows, Linux |
| same-file | 1.0.6 | Unlicense/MIT | https://github.com/BurntSushi/same-file | macOS, Windows, Linux |
| schemars | 0.8.22 | MIT | https://github.com/GREsau/schemars | macOS, Windows, Linux |
| schemars_derive | 0.8.22 | MIT | https://github.com/GREsau/schemars | macOS, Windows, Linux |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 | https://github.com/bluss/scopeguard | macOS, Windows, Linux |
| selectors | 0.38.0 | MPL-2.0 | https://github.com/servo/stylo | macOS, Windows, Linux |
| semver | 1.0.28 | MIT OR Apache-2.0 | https://github.com/dtolnay/semver | macOS, Windows, Linux |
| serde | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde | macOS, Windows, Linux |
| serde-untagged | 0.1.9 | MIT OR Apache-2.0 | https://github.com/dtolnay/serde-untagged | macOS, Windows, Linux |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde | macOS, Windows, Linux |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde | macOS, Windows, Linux |
| serde_derive_internals | 0.29.1 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde | macOS, Windows, Linux |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | https://github.com/serde-rs/json | macOS, Windows, Linux |
| serde_repr | 0.1.21 | MIT OR Apache-2.0 | https://github.com/dtolnay/serde-repr | macOS, Windows, Linux |
| serde_spanned | 0.6.9 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | Linux |
| serde_spanned | 1.1.1 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | macOS, Windows, Linux |
| serde_with | 3.24.0 | MIT OR Apache-2.0 | https://github.com/jonasbb/serde_with/ | macOS, Windows, Linux |
| serde_with_macros | 3.24.0 | MIT OR Apache-2.0 | https://github.com/jonasbb/serde_with/ | macOS, Windows, Linux |
| serialize-to-javascript | 0.1.2 | MIT OR Apache-2.0 | https://github.com/chippers/serialize-to-javascript | macOS, Windows, Linux |
| serialize-to-javascript-impl | 0.1.2 | MIT OR Apache-2.0 | https://github.com/chippers/serialize-to-javascript | macOS, Windows, Linux |
| servo_arc | 0.4.3 | MIT OR Apache-2.0 | https://github.com/servo/stylo | macOS, Windows, Linux |
| sha2 | 0.10.9 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes | macOS, Windows, Linux |
| sha2 | 0.11.0 | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes | macOS, Windows, Linux |
| simd-adler32 | 0.3.10 | MIT | https://github.com/mcountryman/simd-adler32 | macOS, Windows, Linux |
| siphasher | 1.0.4 | MIT OR Apache-2.0 | https://github.com/jedisct1/rust-siphash | macOS, Windows, Linux |
| slab | 0.4.12 | MIT | https://github.com/tokio-rs/slab | Linux |
| smallvec | 1.16.2 | MIT OR Apache-2.0 | https://github.com/servo/rust-smallvec | macOS, Windows, Linux |
| softbuffer | 0.4.8 | MIT OR Apache-2.0 | https://github.com/rust-windowing/softbuffer | Windows |
| soup3 | 0.5.0 | MIT | https://gitlab.gnome.org/World/Rust/soup3-rs | Linux |
| soup3-sys | 0.5.0 | MIT | https://gitlab.gnome.org/World/Rust/soup3-rs | Linux |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | https://github.com/storyyeller/stable_deref_trait | macOS, Windows, Linux |
| string_cache | 0.9.0 | MIT OR Apache-2.0 | https://github.com/servo/string-cache | macOS, Windows, Linux |
| strsim | 0.11.1 | MIT | https://github.com/rapidfuzz/strsim-rs | macOS, Windows, Linux |
| subtle | 2.6.1 | BSD-3-Clause | https://github.com/dalek-cryptography/subtle | macOS, Windows, Linux |
| swift-rs | 1.0.8 | MIT OR Apache-2.0 | https://github.com/Brendonovich/swift-rs | macOS, Windows, Linux |
| syn | 1.0.109 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn | Linux |
| syn | 2.0.119 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn | macOS, Windows, Linux |
| syn | 3.0.6 | MIT OR Apache-2.0 | https://github.com/dtolnay/syn | macOS, Windows, Linux |
| synstructure | 0.14.0 | MIT | https://github.com/mystor/synstructure | macOS, Windows, Linux |
| tao | 0.37.1 | Apache-2.0 | https://github.com/tauri-apps/tao | macOS, Windows, Linux |
| tauri | 2.12.1 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri | macOS, Windows, Linux |
| tauri-codegen | 2.7.1 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri | macOS, Windows, Linux |
| tauri-macros | 2.7.1 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri | macOS, Windows, Linux |
| tauri-runtime | 2.12.1 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri | macOS, Windows, Linux |
| tauri-runtime-wry | 2.12.1 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri | macOS, Windows, Linux |
| tauri-utils | 2.10.1 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri | macOS, Windows, Linux |
| tendril | 0.5.1 | MIT OR Apache-2.0 | https://github.com/servo/html5ever | macOS, Windows, Linux |
| thiserror | 1.0.69 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror | Linux |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror | macOS, Windows, Linux |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror | Linux |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror | macOS, Windows, Linux |
| time | 0.3.55 | MIT OR Apache-2.0 | https://github.com/time-rs/time | macOS, Windows, Linux |
| time-core | 0.1.9 | MIT OR Apache-2.0 | https://github.com/time-rs/time | macOS, Windows, Linux |
| time-macros | 0.2.32 | MIT OR Apache-2.0 | https://github.com/time-rs/time | macOS, Windows, Linux |
| tinystr | 0.8.4 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| tokio | 1.53.2 | MIT | https://github.com/tokio-rs/tokio | macOS, Windows, Linux |
| toml | 1.1.6+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | macOS, Windows, Linux |
| toml_datetime | 0.6.3 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | Linux |
| toml_datetime | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | macOS, Windows, Linux |
| toml_edit | 0.19.15 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | Linux |
| toml_edit | 0.20.2 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | Linux |
| toml_parser | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | macOS, Windows, Linux |
| toml_writer | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml | macOS, Windows, Linux |
| tracing | 0.1.44 | MIT | https://github.com/tokio-rs/tracing | Windows |
| tracing-core | 0.1.36 | MIT | https://github.com/tokio-rs/tracing | Windows |
| typeid | 1.0.3 | MIT OR Apache-2.0 | https://github.com/dtolnay/typeid | macOS, Windows, Linux |
| typenum | 1.20.1 | MIT OR Apache-2.0 | https://github.com/paholg/typenum | macOS, Windows, Linux |
| unicode-ident | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident | macOS, Windows, Linux |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-segmentation | Windows |
| untrusted | 0.9.0 | ISC | https://github.com/briansmith/untrusted | macOS, Windows, Linux |
| ureq | 3.4.2 | MIT OR Apache-2.0 | https://github.com/algesten/ureq | macOS, Windows, Linux |
| ureq-proto | 0.6.4 | MIT OR Apache-2.0 | https://github.com/algesten/ureq-proto | macOS, Windows, Linux |
| url | 2.5.8 | MIT OR Apache-2.0 | https://github.com/servo/rust-url | macOS, Windows, Linux |
| urlpattern | 0.6.0 | MIT | https://github.com/denoland/rust-urlpattern | macOS, Windows, Linux |
| utf8-zero | 0.8.1 | MIT OR Apache-2.0 | https://github.com/algesten/utf8-zero | macOS, Windows, Linux |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT | https://github.com/hsivonen/utf8_iter | macOS, Windows, Linux |
| uuid | 1.27.0 | Apache-2.0 OR MIT | https://github.com/uuid-rs/uuid | macOS, Windows, Linux |
| walkdir | 2.5.0 | Unlicense/MIT | https://github.com/BurntSushi/walkdir | macOS, Windows, Linux |
| web-time | 1.1.0 | MIT OR Apache-2.0 | https://github.com/daxpedda/web-time | macOS, Windows, Linux |
| web_atoms | 0.2.6 | MIT OR Apache-2.0 | https://github.com/servo/html5ever | macOS, Windows, Linux |
| webkit2gtk | 2.0.2 | MIT | https://github.com/tauri-apps/webkit2gtk-rs | Linux |
| webkit2gtk-sys | 2.0.2 | MIT | https://github.com/tauri-apps/webkit2gtk-rs | Linux |
| webpki-roots | 1.0.9 | CDLA-Permissive-2.0 | https://github.com/rustls/webpki-roots | macOS, Windows, Linux |
| webview2-com | 0.39.1 | MIT | https://github.com/wravery/webview2-rs | Windows |
| webview2-com-macros | 0.8.1 | MIT | https://github.com/wravery/webview2-rs | Windows |
| webview2-com-sys | 0.39.1 | MIT | https://github.com/wravery/webview2-rs | Windows |
| winapi-util | 0.1.11 | Unlicense OR MIT | https://github.com/BurntSushi/winapi-util | Windows |
| window-vibrancy | 0.8.1 | Apache-2.0 OR MIT | https://github.com/tauri-apps/tauri-plugin-vibrancy | macOS, Windows |
| windows | 0.58.0 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows | 0.62.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-core | 0.58.0 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-core | 0.62.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-future | 0.3.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-implement | 0.58.0 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-interface | 0.58.0 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-result | 0.2.0 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-result | 0.4.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-strings | 0.1.0 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows-version | 0.1.7 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs | Windows |
| winnow | 0.5.40 | MIT | https://github.com/winnow-rs/winnow | Linux |
| winnow | 1.0.4 | MIT | https://github.com/winnow-rs/winnow | macOS, Windows, Linux |
| writeable | 0.6.4 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| wry | 0.57.0 | Apache-2.0 OR MIT | https://github.com/tauri-apps/wry | macOS, Windows, Linux |
| x11 | 2.21.0 | MIT | https://github.com/AltF02/x11-rs.git | Linux |
| x11-dl | 2.21.0 | MIT | https://github.com/AltF02/x11-rs.git | Linux |
| yoke | 0.8.3 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| yoke-derive | 0.8.4 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| zerofrom | 0.1.8 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| zerofrom-derive | 0.1.8 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| zeroize | 1.9.1 | Apache-2.0 OR MIT | https://github.com/RustCrypto/utils | macOS, Windows, Linux |
| zerotrie | 0.2.5 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| zerovec | 0.11.8 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| zerovec-derive | 0.11.6 | Unicode-3.0 | https://github.com/unicode-org/icu4x | macOS, Windows, Linux |
| zmij | 1.0.23 | MIT | https://github.com/dtolnay/zmij | macOS, Windows, Linux |
