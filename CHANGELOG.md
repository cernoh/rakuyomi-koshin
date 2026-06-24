# [1.21.0](https://github.com/cernoh/rakuyomi/compare/v1.20.0...v1.21.0) (2026-06-24)


### Bug Fixes

* Add a minimum update interval to the manga update cron job. ([e866fec](https://github.com/cernoh/rakuyomi/commit/e866fec1a616ea262b3273548da8f9d140ad7893))
* add list changes button in context menu if mode tap to continue enable ([#183](https://github.com/cernoh/rakuyomi/issues/183)) ([eabebfa](https://github.com/cernoh/rakuyomi/commit/eabebfa661c34a5ed10b5ff654fbe3260877caf6))
* Add nil check for manga in context menu handler closes [#128](https://github.com/cernoh/rakuyomi/issues/128) ([11a1713](https://github.com/cernoh/rakuyomi/commit/11a171311bc1edd63205ebaa2dd405098f4a01bd))
* Add nil checks before calling methods on `ReaderUI.instance` and `FileManager.instance` to prevent potential errors. ([5767129](https://github.com/cernoh/rakuyomi/commit/576712957234546e558c00e2c3b5c4291fc8e98a)), closes [#115](https://github.com/cernoh/rakuyomi/issues/115)
* add optimize_image setting to UpdateableSettings ([ff92b0a](https://github.com/cernoh/rakuyomi/commit/ff92b0aafdf7e0d851c65d9ee4cf9c4d24bc7dc9))
* add remove manga from playlist ([#140](https://github.com/cernoh/rakuyomi/issues/140)) ([ffee371](https://github.com/cernoh/rakuyomi/commit/ffee371c146b89074f1a6dc4cd7fbff298fe82ab))
* can't load localize default header `Plural-Forms` ([#91](https://github.com/cernoh/rakuyomi/issues/91)) ([0a99ec3](https://github.com/cernoh/rakuyomi/commit/0a99ec39c671f34c6cd3f63ab62992ea87cdafe8))
* Change WASM image dimension functions to return `f32` and add `f32` Wasm value and return type support. ([4c2e59f](https://github.com/cernoh/rakuyomi/commit/4c2e59f775ba854857218288776c2b04e421a30f))
* Clear `last_read` timestamp when chapters are marked unread. ([#132](https://github.com/cernoh/rakuyomi/issues/132)) ([d672f40](https://github.com/cernoh/rakuyomi/commit/d672f406678215a6e595c37b59e08926ecc78a32))
* crash on open Rakuyomi in SimpleUI or Zen UI ([11ad526](https://github.com/cernoh/rakuyomi/commit/11ad526fc1952a138bcd2de4b16f8fd947696d6a))
* disable default features for mozjpeg dependency ([#112](https://github.com/cernoh/rakuyomi/issues/112)) ([4a7f2a6](https://github.com/cernoh/rakuyomi/commit/4a7f2a6ec95724a63128a0e07aae77f19938d901))
* dynamically scale playlist item height in PlaylistDialog closes [#127](https://github.com/cernoh/rakuyomi/issues/127) ([c496e6a](https://github.com/cernoh/rakuyomi/commit/c496e6a325ad7f2a1cd79128b6a8f4f7a164e204))
* Enhance continue reading with lang filtering** ([#103](https://github.com/cernoh/rakuyomi/issues/103)) ([15f9aa9](https://github.com/cernoh/rakuyomi/commit/15f9aa9f290f91a15007bf9b14cd4503fb01dc98))
* enhance library view metadata display with full last read text and adjusted unread chapter count, and refine menu item rendering for better layout. ([042982e](https://github.com/cernoh/rakuyomi/commit/042982e08de8f1826f1d4f2daea201c1daa3aac5))
* grid mode calculate page_num wrong ([#125](https://github.com/cernoh/rakuyomi/issues/125)) ([b305e09](https://github.com/cernoh/rakuyomi/commit/b305e09af0dcf15418ac4233d0c7e7b1e666d44d))
* **html_element:** improve text content extraction ([2cb86d1](https://github.com/cernoh/rakuyomi/commit/2cb86d13ab17346dfdb0595cfcff73368ac70744)), closes [#61](https://github.com/cernoh/rakuyomi/issues/61)
* **html_element:** simplify own_text implementation ([cea58fc](https://github.com/cernoh/rakuyomi/commit/cea58fce3644b3e6b020512e0a74b48b9052aeb3)), closes [#62](https://github.com/cernoh/rakuyomi/issues/62)
* **html:** improve fragment parsing and text extraction ([1219806](https://github.com/cernoh/rakuyomi/commit/12198068411b7c684c1dd8619aec47cc2578ac2e)), closes [tachibana-shin/rakuyomi#111](https://github.com/tachibana-shin/rakuyomi/issues/111)
* **image:** add raw image data support ([e58bb6d](https://github.com/cernoh/rakuyomi/commit/e58bb6d8c08ff5e52423d6571cb0dde1a4e60923)), closes [tachibana-shin/rakuyomi#64](https://github.com/tachibana-shin/rakuyomi/issues/64)
* Improve source settings deserialization and rendering ([f8847a2](https://github.com/cernoh/rakuyomi/commit/f8847a2e47bc1c7c13ca029e03de98a7a6c49c48)), closes [#72](https://github.com/cernoh/rakuyomi/issues/72)
* inconsistent progression to next chapter go back to rakuyomi ([#136](https://github.com/cernoh/rakuyomi/issues/136)) ([b44ae86](https://github.com/cernoh/rakuyomi/commit/b44ae86623baea49ec2812a43a61138aad4da9f1))
* **job:** make language selection optional ([#107](https://github.com/cernoh/rakuyomi/issues/107)) ([009b457](https://github.com/cernoh/rakuyomi/commit/009b457f5df3c7bf37a1ac8b9a1b6172fe44a548))
* **jobs:** Add language filtering for chapter downloads ([#100](https://github.com/cernoh/rakuyomi/issues/100)) ([c01ecaf](https://github.com/cernoh/rakuyomi/commit/c01ecaf3df6acc503c429a970a3d064a238af5df))
* **l10n:** fix UI string translatability ([ba1406e](https://github.com/cernoh/rakuyomi/commit/ba1406eef61af61989385015789e7f71b016a6d9))
* **LibraryView:** enhance playlist dialog handling ([#141](https://github.com/cernoh/rakuyomi/issues/141)) ([d137c3b](https://github.com/cernoh/rakuyomi/commit/d137c3b6d5ad5dd714ee41b871f23522b11bce24))
* **library:** wrap chapter fetch in Trapper ([#191](https://github.com/cernoh/rakuyomi/issues/191)) ([3f57091](https://github.com/cernoh/rakuyomi/commit/3f570914f6eb345d707b055700adcd60f91f62e1))
* make text setting placeholder optional in backend and frontend models ([d63bfcd](https://github.com/cernoh/rakuyomi/commit/d63bfcdbbaff0092cd9afc2d396370992cf6f39f)), closes [#145](https://github.com/cernoh/rakuyomi/issues/145)
* **manga-reader:** apply file manager override to zen UI ([#198](https://github.com/cernoh/rakuyomi/issues/198)) ([215f224](https://github.com/cernoh/rakuyomi/commit/215f2245d0487a37a9d697aee49ca676b2f73455))
* OTA update never shows the "Restart Now" dialog on old Kindles ([#187](https://github.com/cernoh/rakuyomi/issues/187)) ([f38596e](https://github.com/cernoh/rakuyomi/commit/f38596e81e6c38c87b2b4d427b7a69568de27160))
* **performance:** WIP replace image crate with zune and mozjpeg for faster image processing speedup decode `x800.000` ([bbc5f74](https://github.com/cernoh/rakuyomi/commit/bbc5f7427e41b8795069b8920f836d45c69385b0))
* **rakuyomi:** prevent network ops when offline ([#190](https://github.com/cernoh/rakuyomi/issues/190)) ([d70b126](https://github.com/cernoh/rakuyomi/commit/d70b126f4f6b71f6c6af2c0075ba2ed12d72634d))
* Reduce available width calculation in MenuItemCover by 12 units. ([0fa7f58](https://github.com/cernoh/rakuyomi/commit/0fa7f583157b96f7556bbce232f67535e642a6f6))
* refine grid/cover view layout calculations and enhance manga information display with source, last read, and unread chapter details. ([60bfdaa](https://github.com/cernoh/rakuyomi/commit/60bfdaadaa1cf3d3cc11055c544770bdf39bd51d))
* **search:** recreate results UI on return closes [#173](https://github.com/cernoh/rakuyomi/issues/173) ([7d874dd](https://github.com/cernoh/rakuyomi/commit/7d874ddfe2fb7f81aea3bc2e74f2f3f72704e638))
* seed default settings.json on first run ([#159](https://github.com/cernoh/rakuyomi/issues/159)) ([6d1b678](https://github.com/cernoh/rakuyomi/commit/6d1b6788617f17a7490d8210f5fff7ec6b0533ab))
* **selector:** normalize :contains() arguments ([88cab31](https://github.com/cernoh/rakuyomi/commit/88cab3176de5e8490d3690499fb96742221fa361))
* show all sources available ([9556cdd](https://github.com/cernoh/rakuyomi/commit/9556cdd822ae1757d7ef6ab84994709c5f62c217))
* simplify chapter read toggle logic closes [#176](https://github.com/cernoh/rakuyomi/issues/176) ([95f0c68](https://github.com/cernoh/rakuyomi/commit/95f0c68c0c5b9fdd5181a041df22797b3b98981c))
* sorters in library ([425c473](https://github.com/cernoh/rakuyomi/commit/425c47325058d61389093d26687f6dbe82f5d1d6))
* table insertion for context menu buttons ([e73c45d](https://github.com/cernoh/rakuyomi/commit/e73c45d156c40f990c55cc76f42747e0417972b4))
* Update Chinese (Traditional) localization for Rakuyomi plugin and add feature ([26603fd](https://github.com/cernoh/rakuyomi/commit/26603fd8217824822060d312a5be379a95e763dd))
* **wasm-store:** remove std reference tracking ([ed76f8e](https://github.com/cernoh/rakuyomi/commit/ed76f8e3d04a0cc1b7c2e4f9d9e6997fd71d34bf)), closes [#62](https://github.com/cernoh/rakuyomi/issues/62)


### Features

* add gallery view to search results ([#155](https://github.com/cernoh/rakuyomi/issues/155)) ([1d38265](https://github.com/cernoh/rakuyomi/commit/1d38265835ea0d5dfc3b3bd3fb611b9e3ad6b966))
* add get_url for next SDK to map aidoku-rs API ([cac3f8f](https://github.com/cernoh/rakuyomi/commit/cac3f8fa05ac57733153f677dfe24d377016c31b))
* add kind and child_nodes html node api ([76bbc23](https://github.com/cernoh/rakuyomi/commit/76bbc236d4642c87e39b0ee48c99223f05adef8a))
* add picker setting type alias for select ([b498d2c](https://github.com/cernoh/rakuyomi/commit/b498d2cf6cd613cc160cd8868d5201b9422b69f2))
* add skip resume confirmation setting ([1276bb3](https://github.com/cernoh/rakuyomi/commit/1276bb37b91659a4029529034e1eb49cd6b02625))
* **backend/shared:** Replace scraper and kuchiki with dom_query ([#57](https://github.com/cernoh/rakuyomi/issues/57)) ([6f5055f](https://github.com/cernoh/rakuyomi/commit/6f5055fdf0dceb3ea8ce3842fcdb412e7d4ca408))
* **chapter-listing:** Add chapter language filter ([#96](https://github.com/cernoh/rakuyomi/issues/96)) ([d079ae0](https://github.com/cernoh/rakuyomi/commit/d079ae03fcc7fea37b106adf2ddcb692bf46bf43)), closes [#92](https://github.com/cernoh/rakuyomi/issues/92)
* **download:** add chapter download progress ([#197](https://github.com/cernoh/rakuyomi/issues/197)) ([a61a2d9](https://github.com/cernoh/rakuyomi/commit/a61a2d9d3d9d6939eb77c4869fe4b4830a513d5f))
* implement playlist management functionality across frontend a ([#126](https://github.com/cernoh/rakuyomi/issues/126)) ([474b885](https://github.com/cernoh/rakuyomi/commit/474b8852c0dd9913041850d9cb33d995f92c61f4))
* **library:** add tap manga action setting ([939ec4d](https://github.com/cernoh/rakuyomi/commit/939ec4d8f40c9deb43dfbe0b1d94cb2495648774))
* **logging:** add option to disable plugin logging ([#195](https://github.com/cernoh/rakuyomi/issues/195)) ([161f44a](https://github.com/cernoh/rakuyomi/commit/161f44a660c22070f2d74a5da23c10e17857543e))
* luacheck ([#199](https://github.com/cernoh/rakuyomi/issues/199)) ([63b0412](https://github.com/cernoh/rakuyomi/commit/63b041223cf7fbf249195e68736a374e44f756d7))
* **net:** allow Wasm to set request timeout ([341e62e](https://github.com/cernoh/rakuyomi/commit/341e62e8c5dfec1fe0d8c9ffced4e14bcdd2b0b1))
* New UI library ([#118](https://github.com/cernoh/rakuyomi/issues/118)) ([9db09bf](https://github.com/cernoh/rakuyomi/commit/9db09bf2f9af9ec169422ddf1f550caa7bb5d1b7))
* Preload chapters while reading. ([#69](https://github.com/cernoh/rakuyomi/issues/69)) ([59c6087](https://github.com/cernoh/rakuyomi/commit/59c60878fc7679ed92838c7db11d3c306b8cb9e0))
* release ([992bf9e](https://github.com/cernoh/rakuyomi/commit/992bf9ea6d505acdb07c1fdf52d115b73448e598))
* **search:** add pagination to manga search ([#165](https://github.com/cernoh/rakuyomi/issues/165)) ([cfb031e](https://github.com/cernoh/rakuyomi/commit/cfb031e06450a1bd4e69c3928c85dd78bd4de3fa))
* **server:** add auto-stop server on rakuyomi close ([#196](https://github.com/cernoh/rakuyomi/issues/196)) ([afd5d83](https://github.com/cernoh/rakuyomi/commit/afd5d836acab5bfdfb0bf6be3032f95b047056d5))
* **wasm:** register print and abort in std ([b372d7f](https://github.com/cernoh/rakuyomi/commit/b372d7f76e558989c703648fd1a9663071c3350e)), closes [#179](https://github.com/cernoh/rakuyomi/issues/179)


### Performance Improvements

* **process:** Use FFI for binary execution ([#202](https://github.com/cernoh/rakuyomi/issues/202)) ([98dd669](https://github.com/cernoh/rakuyomi/commit/98dd669434197de37d4dbf2912f1ef402120f4dc))

# [1.35.0](https://github.com/tachibana-shin/rakuyomi/compare/v1.34.1...v1.35.0) (2026-06-19)


### Bug Fixes

* **manga-reader:** apply file manager override to zen UI ([#198](https://github.com/tachibana-shin/rakuyomi/issues/198)) ([215f224](https://github.com/tachibana-shin/rakuyomi/commit/215f2245d0487a37a9d697aee49ca676b2f73455))
* OTA update never shows the "Restart Now" dialog on old Kindles ([#187](https://github.com/tachibana-shin/rakuyomi/issues/187)) ([f38596e](https://github.com/tachibana-shin/rakuyomi/commit/f38596e81e6c38c87b2b4d427b7a69568de27160))


### Features

* **download:** add chapter download progress ([#197](https://github.com/tachibana-shin/rakuyomi/issues/197)) ([a61a2d9](https://github.com/tachibana-shin/rakuyomi/commit/a61a2d9d3d9d6939eb77c4869fe4b4830a513d5f))
* **logging:** add option to disable plugin logging ([#195](https://github.com/tachibana-shin/rakuyomi/issues/195)) ([161f44a](https://github.com/tachibana-shin/rakuyomi/commit/161f44a660c22070f2d74a5da23c10e17857543e))
* luacheck ([#199](https://github.com/tachibana-shin/rakuyomi/issues/199)) ([63b0412](https://github.com/tachibana-shin/rakuyomi/commit/63b041223cf7fbf249195e68736a374e44f756d7))
* **server:** add auto-stop server on rakuyomi close ([#196](https://github.com/tachibana-shin/rakuyomi/issues/196)) ([afd5d83](https://github.com/tachibana-shin/rakuyomi/commit/afd5d836acab5bfdfb0bf6be3032f95b047056d5))


### Performance Improvements

* **process:** Use FFI for binary execution ([#202](https://github.com/tachibana-shin/rakuyomi/issues/202)) ([98dd669](https://github.com/tachibana-shin/rakuyomi/commit/98dd669434197de37d4dbf2912f1ef402120f4dc))
