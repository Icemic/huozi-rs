# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.13.0 (2025-10-08)

### New Features

 - <csr-id-c6ca983b2bf1313a56278ec6df409509b1e7cd6a/> update wgpu version to 27.0

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 18 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Update wgpu version to 27.0 ([`c6ca983`](https://github.com/Icemic/huozi-rs/commit/c6ca983b2bf1313a56278ec6df409509b1e7cd6a))
</details>

## v0.12.0 (2025-09-20)

### New Features

 - <csr-id-bfa5c27f5499720d0c6ebbb530b2879223373741/> bump wgpu to ^26.0

### Bug Fixes

 - <csr-id-afee0947cfb41438b3a15b67bd3df97b78c9ca97/> update deps

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 7 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi v0.12.0 ([`b531fd9`](https://github.com/Icemic/huozi-rs/commit/b531fd96e2b660800c88f45fe1988438fcdd825c))
    - Update deps ([`afee094`](https://github.com/Icemic/huozi-rs/commit/afee0947cfb41438b3a15b67bd3df97b78c9ca97))
    - Bump wgpu to ^26.0 ([`bfa5c27`](https://github.com/Icemic/huozi-rs/commit/bfa5c27f5499720d0c6ebbb530b2879223373741))
</details>

## v0.11.0 (2025-09-13)

<csr-id-fe08e4fecbabe57da06876fbad36aa2526279a08/>
<csr-id-55800ade7b9b1f52d35a90c33d3e739edcd09484/>
<csr-id-ef2edac8d0cbb517d709bdfc706573172dd50950/>
<csr-id-3c77fa16189fd7237c3dc50d684e0c6c76391492/>

### Chore

 - <csr-id-fe08e4fecbabe57da06876fbad36aa2526279a08/> update roadmap

### New Features

<csr-id-a75e83cf1f9ea836164791d7218c2b25c2de9822/>

 - <csr-id-89285ec6723e726a796bd449cffbac4a74c6c903/> add text shaping and font analysis tools
   Adds rustybuzz and ttf-parser dependencies to enable advanced text shaping and font metrics analysis.
   
   Includes two new example programs:
   - buzz_info: demonstrates text shaping with language support and OpenType features

### Bug Fixes

 - <csr-id-1942b9608b424de1b88712e177ecbbe4fa31f34f/> adjust stroke and shadow calculations for consistent scaling
 - <csr-id-dc9525021521ef5cbb9d9b16bc6d6165b3c6299b/> apply consistent scaling to both x and y axes
 - <csr-id-aa3595d150e1e3184517e1a034a78f21216d63d1/> update ab_glyph version and adjust default features in Cargo.toml

### Refactor

 - <csr-id-55800ade7b9b1f52d35a90c33d3e739edcd09484/> remove rusttype.rs
 - <csr-id-ef2edac8d0cbb517d709bdfc706573172dd50950/> optimizes performance
 - <csr-id-3c77fa16189fd7237c3dc50d684e0c6c76391492/> remove unused code and example function

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release over the course of 45 calendar days.
 - 46 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi v0.11.0 ([`9e516aa`](https://github.com/Icemic/huozi-rs/commit/9e516aa37ba80022e092a79383038d10f83e7612))
    - Add text shaping and font analysis tools ([`89285ec`](https://github.com/Icemic/huozi-rs/commit/89285ec6723e726a796bd449cffbac4a74c6c903))
    - Adjust stroke and shadow calculations for consistent scaling ([`1942b96`](https://github.com/Icemic/huozi-rs/commit/1942b9608b424de1b88712e177ecbbe4fa31f34f))
    - Apply consistent scaling to both x and y axes ([`dc95250`](https://github.com/Icemic/huozi-rs/commit/dc9525021521ef5cbb9d9b16bc6d6165b3c6299b))
    - Update roadmap ([`fe08e4f`](https://github.com/Icemic/huozi-rs/commit/fe08e4fecbabe57da06876fbad36aa2526279a08))
    - Add FiraCode font files and license ([`a75e83c`](https://github.com/Icemic/huozi-rs/commit/a75e83cf1f9ea836164791d7218c2b25c2de9822))
    - Remove rusttype.rs ([`55800ad`](https://github.com/Icemic/huozi-rs/commit/55800ade7b9b1f52d35a90c33d3e739edcd09484))
    - Optimizes performance ([`ef2edac`](https://github.com/Icemic/huozi-rs/commit/ef2edac8d0cbb517d709bdfc706573172dd50950))
    - Update ab_glyph version and adjust default features in Cargo.toml ([`aa3595d`](https://github.com/Icemic/huozi-rs/commit/aa3595d150e1e3184517e1a034a78f21216d63d1))
    - Remove unused code and example function ([`3c77fa1`](https://github.com/Icemic/huozi-rs/commit/3c77fa16189fd7237c3dc50d684e0c6c76391492))
</details>

<csr-unknown>
face_info: provides font glyph metrics and bounding box information<csr-unknown/>

## v0.10.0 (2025-07-28)

### Bug Fixes

 - <csr-id-684038e0fc151c3dc83800ebeefcefd747f6cf2b/> implement Send and Sync traits for Huozi struct when font_kit feature is enabled
 - <csr-id-37e35c1c9c09efdb0d1f8c85a8f03b7ccdcc6d41/> revert changes on sdf calculation
 - <csr-id-6f7e92bb82be3d685149d34fc8056862c50be8fe/> optimize glyph rasterization and metrics calculation in font_kit
 - <csr-id-ffd6f2da732add785f4131b33bda177215d520c1/> update hinting options to None for improved glyph rendering
 - <csr-id-48936b30fb371ba7283afe0dae5d6906e8a9aae3/> improve SDF alpha handling and prevent interpolation artifacts
 - <csr-id-8e5a5d2e84c81a0933950d7a08cfe5f30987cc88/> Corrects buffer values for different color spaces
   Adjusts buffer calculations to account for gamma correction differences between Linear and SRGB color spaces.
   
   Uses theoretically precise conversion values (Linear 0.5 = SRGB 0.735357) for fill rendering and maintains original empirically tuned values for stroke and shadow effects.
   
   Uncomments gamma calculation to enable proper anti-aliasing support.
 - <csr-id-375ed182547af5cb1307b2164f229675961c8a52/> Correct glyph scaling and rendering parameters for font-kit backend

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 17 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi v0.10.0 ([`5902e4a`](https://github.com/Icemic/huozi-rs/commit/5902e4ae6e24d0d810009fc80e46b0305d30676b))
    - Implement Send and Sync traits for Huozi struct when font_kit feature is enabled ([`684038e`](https://github.com/Icemic/huozi-rs/commit/684038e0fc151c3dc83800ebeefcefd747f6cf2b))
    - Revert changes on sdf calculation ([`37e35c1`](https://github.com/Icemic/huozi-rs/commit/37e35c1c9c09efdb0d1f8c85a8f03b7ccdcc6d41))
    - Optimize glyph rasterization and metrics calculation in font_kit ([`6f7e92b`](https://github.com/Icemic/huozi-rs/commit/6f7e92bb82be3d685149d34fc8056862c50be8fe))
    - Update hinting options to None for improved glyph rendering ([`ffd6f2d`](https://github.com/Icemic/huozi-rs/commit/ffd6f2da732add785f4131b33bda177215d520c1))
    - Improve SDF alpha handling and prevent interpolation artifacts ([`48936b3`](https://github.com/Icemic/huozi-rs/commit/48936b30fb371ba7283afe0dae5d6906e8a9aae3))
    - Corrects buffer values for different color spaces ([`8e5a5d2`](https://github.com/Icemic/huozi-rs/commit/8e5a5d2e84c81a0933950d7a08cfe5f30987cc88))
    - Correct glyph scaling and rendering parameters for font-kit backend ([`375ed18`](https://github.com/Icemic/huozi-rs/commit/375ed182547af5cb1307b2164f229675961c8a52))
</details>

## v0.9.0 (2025-07-10)

<csr-id-30933fff198c564bac4da833839befb450d2758d/>
<csr-id-9273541ecd536ce9df96609fc47cf7025cbb56e3/>
<csr-id-b71b1d09d23621da8c62398fd0b293d5b151ef79/>

### Chore

 - <csr-id-30933fff198c564bac4da833839befb450d2758d/> add TsangerYuYang font assets with LFS support
 - <csr-id-9273541ecd536ce9df96609fc47cf7025cbb56e3/> Remove unused variables and update comment format

### Chore

 - <csr-id-b71b1d09d23621da8c62398fd0b293d5b151ef79/> add comprehensive changelog documentation

### New Features

 - <csr-id-4e64b5616c5fb7948439d82b738d341e9a37173b/> upgrade nom parser to v8.0
 - <csr-id-5ba9198a5b37043a9e905f5a2d14885687bd8bf1/> upgrade dependencies and simplify color handling
   Updates csscolorparser to v0.7, lru to v0.16, pollster to v0.4, and android_logger to v0.15.
   
   Removes custom color extension trait by leveraging built-in methods from the updated csscolorparser library, reducing code complexity and maintenance overhead.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 203 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi v0.9.0 ([`96cfc4d`](https://github.com/Icemic/huozi-rs/commit/96cfc4dff6f9df129925cef30f9142b040bb0794))
    - Add comprehensive changelog documentation ([`b71b1d0`](https://github.com/Icemic/huozi-rs/commit/b71b1d09d23621da8c62398fd0b293d5b151ef79))
    - Add TsangerYuYang font assets with LFS support ([`30933ff`](https://github.com/Icemic/huozi-rs/commit/30933fff198c564bac4da833839befb450d2758d))
    - Remove unused variables and update comment format ([`9273541`](https://github.com/Icemic/huozi-rs/commit/9273541ecd536ce9df96609fc47cf7025cbb56e3))
    - Upgrade nom parser to v8.0 ([`4e64b56`](https://github.com/Icemic/huozi-rs/commit/4e64b5616c5fb7948439d82b738d341e9a37173b))
    - Upgrade dependencies and simplify color handling ([`5ba9198`](https://github.com/Icemic/huozi-rs/commit/5ba9198a5b37043a9e905f5a2d14885687bd8bf1))
</details>

## v0.8.0 (2024-12-19)

<csr-id-849d16cdb1a250ed5219747d3d6e02450089a6ec/>

### Chore

 - <csr-id-849d16cdb1a250ed5219747d3d6e02450089a6ec/> Release huozi version 0.8.0

### New Features

 - <csr-id-198e61774fd02000c8bef3b3c47ffa9de5c9ae8d/> update constants for grid size, font size, buffer, and ascent to enlarge texture size for a single glyph from 48x48 to 96x96 for a better rendering appearance
 - <csr-id-accb24c53e37701df8c3d9972399cbe1be37769a/> add experimental font-kit support

### Bug Fixes

 - <csr-id-19c7aa712645afe74fd28f34489741d4d4283b28/> handle overwriting of texture blocks for expired glyphs
 - <csr-id-fa658767d207165835a9086d2822fc81b66c537b/> update font file path in texture example

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 144 calendar days.
 - 144 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.8.0 ([`849d16c`](https://github.com/Icemic/huozi-rs/commit/849d16cdb1a250ed5219747d3d6e02450089a6ec))
    - Update constants for grid size, font size, buffer, and ascent to enlarge texture size for a single glyph from 48x48 to 96x96 for a better rendering appearance ([`198e617`](https://github.com/Icemic/huozi-rs/commit/198e61774fd02000c8bef3b3c47ffa9de5c9ae8d))
    - Handle overwriting of texture blocks for expired glyphs ([`19c7aa7`](https://github.com/Icemic/huozi-rs/commit/19c7aa712645afe74fd28f34489741d4d4283b28))
    - Update font file path in texture example ([`fa65876`](https://github.com/Icemic/huozi-rs/commit/fa658767d207165835a9086d2822fc81b66c537b))
    - Add experimental font-kit support ([`accb24c`](https://github.com/Icemic/huozi-rs/commit/accb24c53e37701df8c3d9972399cbe1be37769a))
    - Update readme ([`413a5a6`](https://github.com/Icemic/huozi-rs/commit/413a5a6b1a2f8cbbe8c830395fb0ca24bbb90493))
</details>

## v0.7.0 (2024-07-28)

<csr-id-f3fb438776f557ca5bc61f9181206cb954f4b01e/>
<csr-id-79a67baaaa980e7a1edc405e9625a56f496c0241/>
<csr-id-70abde19be0a2e134c702fbe908816814f30574f/>

### Chore

 - <csr-id-f3fb438776f557ca5bc61f9181206cb954f4b01e/> Release huozi version 0.7.0
 - <csr-id-79a67baaaa980e7a1edc405e9625a56f496c0241/> fix Cargo.toml
 - <csr-id-70abde19be0a2e134c702fbe908816814f30574f/> update example for android compatibility

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 20 calendar days.
 - 20 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.7.0 ([`f3fb438`](https://github.com/Icemic/huozi-rs/commit/f3fb438776f557ca5bc61f9181206cb954f4b01e))
    - Bump wgpu to 22.0.0 ([`7a239aa`](https://github.com/Icemic/huozi-rs/commit/7a239aa679c3ef2f3de5234cafbc166d873f5508))
    - Fix Cargo.toml ([`79a67ba`](https://github.com/Icemic/huozi-rs/commit/79a67baaaa980e7a1edc405e9625a56f496c0241))
    - Feat: support generate vertices of sRGB color; fix: adapts example to use sRGB color; ([`2b35eb4`](https://github.com/Icemic/huozi-rs/commit/2b35eb4099133300074a1c389216e981d6929d33))
    - Update example for android compatibility ([`70abde1`](https://github.com/Icemic/huozi-rs/commit/70abde19be0a2e134c702fbe908816814f30574f))
</details>

## v0.6.0 (2024-07-07)

<csr-id-8139e3141b3010d21b8b9ecc3bd2172a1e918e84/>

### Chore

 - <csr-id-8139e3141b3010d21b8b9ecc3bd2172a1e918e84/> Release huozi version 0.6.0

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 124 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.6.0 ([`8139e31`](https://github.com/Icemic/huozi-rs/commit/8139e3141b3010d21b8b9ecc3bd2172a1e918e84))
    - Upgrade wgpu to 0.20; Upgrade fontdue to 0.9 ([`525a4f2`](https://github.com/Icemic/huozi-rs/commit/525a4f26e6457aaee6cf50585d61216971b7945b))
</details>

## v0.5.2 (2024-03-05)

<csr-id-c323551fa196476244e77e43fbf3f9c56aeec3ee/>

### Chore

 - <csr-id-c323551fa196476244e77e43fbf3f9c56aeec3ee/> Release huozi version 0.5.2

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.5.2 ([`c323551`](https://github.com/Icemic/huozi-rs/commit/c323551fa196476244e77e43fbf3f9c56aeec3ee))
    - Add Debug and Clone derive to GlyphVertices struct ([`b28d374`](https://github.com/Icemic/huozi-rs/commit/b28d374671c3c51c9398458586a3069ef12c5a26))
</details>

## v0.5.1 (2024-03-03)

<csr-id-3914f70736992f3ce6d19b31bdfbbf4e78d4f403/>
<csr-id-29276b37326ff7694471bbe938f1001edc4fede3/>

### Chore

 - <csr-id-3914f70736992f3ce6d19b31bdfbbf4e78d4f403/> Release huozi version 0.5.1
 - <csr-id-29276b37326ff7694471bbe938f1001edc4fede3/> Release huozi version 0.5.0

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 20 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.5.1 ([`3914f70`](https://github.com/Icemic/huozi-rs/commit/3914f70736992f3ce6d19b31bdfbbf4e78d4f403))
    - Release huozi version 0.5.0 ([`29276b3`](https://github.com/Icemic/huozi-rs/commit/29276b37326ff7694471bbe938f1001edc4fede3))
    - Add `GlyphVertices` struct, providing glyph position infomation for rendering ([`dfb91c2`](https://github.com/Icemic/huozi-rs/commit/dfb91c2d30f8e47a7a45f789262b8e764f85e0d3))
    - Fix export name conflict ([`7c8db4b`](https://github.com/Icemic/huozi-rs/commit/7c8db4b4d7f7d286e332ce9c125e79097be66473))
</details>

## v0.4.2 (2024-02-11)

<csr-id-466c0a75c8cc2de5eb65f2525b6b9e17624b3dcc/>

### Chore

 - <csr-id-466c0a75c8cc2de5eb65f2525b6b9e17624b3dcc/> Release huozi version 0.4.2

### New Features

 - <csr-id-109604f68f8400e2d6c2d39c7ed5b9de07fc74c6/> Add image version tracking to the cache

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.4.2 ([`466c0a7`](https://github.com/Icemic/huozi-rs/commit/466c0a75c8cc2de5eb65f2525b6b9e17624b3dcc))
    - Add image version tracking to the cache ([`109604f`](https://github.com/Icemic/huozi-rs/commit/109604f68f8400e2d6c2d39c7ed5b9de07fc74c6))
</details>

## v0.4.1 (2024-02-10)

<csr-id-9611f3a97ef85dfedc773199363f652647b83a73/>

### Chore

 - <csr-id-9611f3a97ef85dfedc773199363f652647b83a73/> Release huozi version 0.4.1

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 6 calendar days.
 - 8 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.4.1 ([`9611f3a`](https://github.com/Icemic/huozi-rs/commit/9611f3a97ef85dfedc773199363f652647b83a73))
    - Fix text layouting that calculates total width and height wrongly when text is single line ([`1539cbf`](https://github.com/Icemic/huozi-rs/commit/1539cbfdb6ba589e170e935f91ef901de0c424bd))
    - Add readme and license file ([`b4d8b64`](https://github.com/Icemic/huozi-rs/commit/b4d8b64450fc8c889f7364c70e8aa50654da32fa))
</details>

## v0.4.0 (2024-02-02)

<csr-id-8c89261042bd9d74297a1af623a19f327594a698/>

### Chore

 - <csr-id-8c89261042bd9d74297a1af623a19f327594a698/> Release huozi version 0.4.0

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 48 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.4.0 ([`8c89261`](https://github.com/Icemic/huozi-rs/commit/8c89261042bd9d74297a1af623a19f327594a698))
    - Upgrade wgpu to 0.19.1 and winit to 0.29.10 ([`539e39c`](https://github.com/Icemic/huozi-rs/commit/539e39c5275e2494ac414a4b1a2e03e7e0db791f))
</details>

## v0.3.0 (2023-12-15)

<csr-id-0846e566036bf908fa5a959ed4d2be4786df5443/>

### Chore

 - <csr-id-0846e566036bf908fa5a959ed4d2be4786df5443/> Release huozi version 0.3.0

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 113 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.3.0 ([`0846e56`](https://github.com/Icemic/huozi-rs/commit/0846e566036bf908fa5a959ed4d2be4786df5443))
    - Update dependencies and fix window size ([`5e38ef8`](https://github.com/Icemic/huozi-rs/commit/5e38ef8415e2393439699609b18d4380504a507d))
</details>

## v0.2.0 (2023-08-24)

<csr-id-cf73c122bd0ad34d9495ccb39bf05cc15b4b3a4e/>

### Chore

 - <csr-id-cf73c122bd0ad34d9495ccb39bf05cc15b4b3a4e/> Release huozi version 0.2.0

### New Features

 - <csr-id-3dc0732bbe0d367aac698d71e2e189bea831b684/> bump all deps' version

### Bug Fixes

 - <csr-id-f7f3ed1d68f149118026ef5e5834b111839153d8/> coordinate calculation
 - <csr-id-3904d319a31bdf01c2c6050477f984728300cc72/> lint
 - <csr-id-d3367a0f61bdc872710aca0ee75985e014358372/> switch to use stable toolchain

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 7 calendar days.
 - 116 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release huozi version 0.2.0 ([`cf73c12`](https://github.com/Icemic/huozi-rs/commit/cf73c122bd0ad34d9495ccb39bf05cc15b4b3a4e))
    - Coordinate calculation ([`f7f3ed1`](https://github.com/Icemic/huozi-rs/commit/f7f3ed1d68f149118026ef5e5834b111839153d8))
    - Update ([`900a869`](https://github.com/Icemic/huozi-rs/commit/900a869b729ae3bad81dff74b2ad8baba51e3fe3))
    - Lint ([`3904d31`](https://github.com/Icemic/huozi-rs/commit/3904d319a31bdf01c2c6050477f984728300cc72))
    - Bump all deps' version ([`3dc0732`](https://github.com/Icemic/huozi-rs/commit/3dc0732bbe0d367aac698d71e2e189bea831b684))
    - Switch to use stable toolchain ([`d3367a0`](https://github.com/Icemic/huozi-rs/commit/d3367a0f61bdc872710aca0ee75985e014358372))
</details>

## v0.1.0 (2023-04-30)

<csr-id-f4eda510ecba5c0a56c4006798ba3b6021005fb0/>
<csr-id-760bfea80da5cc408f5b083550512ffd2bf30d5a/>
<csr-id-10c52143972473b87f05dd50e8d966a2b9a3e5c1/>

### Chore

 - <csr-id-f4eda510ecba5c0a56c4006798ba3b6021005fb0/> add ignore files in package
 - <csr-id-760bfea80da5cc408f5b083550512ffd2bf30d5a/> add package metadata

### New Features

 - <csr-id-937c6f5c798b709f84e0f4e53e5ca83522db0054/> use open source fonts as samples
 - <csr-id-e00d96b5bfc72696d8d5fc33f9c47c1522794ca1/> huozi add parse api
 - <csr-id-6ae68e7c622ca8b57384912d1c71c3bb18dbe044/> adjust parse return value
 - <csr-id-9ba08faca72090b1a79cf614a72c4e4ca90ff3db/> move parser tests to mod
 - <csr-id-a5ddb201b3f387f503102a571aed6a4d907c7465/> parser
 - <csr-id-d7b3239421213b6312b31940bb0cd20608e9fb2c/> move layout func to Huozi impl
 - <csr-id-15d65f1df63693709a76d86d22dba99d81727211/> warn if glyph not exists
 - <csr-id-4645e32f18284fc12ac3cc2e1ae27a3cc07a93c6/> properly handles characters larger than 1em in width
 - <csr-id-5fb4c2f4453a9a166c376a5e39e23df0e0b2b16f/> make `wgpu` optional
 - <csr-id-e48cda3a49bfa22587cbe1a5cc16236057d4f54c/> better texture generation example
 - <csr-id-6ca41fc8a66aab188480365671e60afc15a636e3/> implement color to [f32;4] with trait
 - <csr-id-b204ed93a6c9630469e1a56e19c4844fdde826f8/> support stroke and shadow
 - <csr-id-d3057d794ca712edb51a24da58ee3d9466de4e87/> add layouting time log
 - <csr-id-3e69da196173c1439183432b4deb0a434240da19/> move color and buffer to vertex attribute
 - <csr-id-810413de40e7129c2fe65127c69b8f029f7ff204/> add features
 - <csr-id-f3418f8fa02b8920a8bd0d71b11fea713d227571/> switch font extractor via features
 - <csr-id-9a54ad624b028ea8880092da883433ed775c6b39/> font extractor, ab_glyph implement
 - <csr-id-1d490c64ca5ba4cc9cc035bc168b2434ac21d128/> better texture vertex calculation with font size
 - <csr-id-94d51a074b56cc8b6235e21a3a964c88b6d7eb1c/> log font load time and sdf texture gen time
 - <csr-id-8e935ea48ce1f8aa8e3232e84a32392e0cbd28d9/> basic layout
 - <csr-id-87723c4bb9866edf7dc3f5e1149d363a17f4357a/> tidy up codebase
 - <csr-id-9d8b9dcdd83c18efc073ebb9b13de5cc1e8ef58b/> cache sdf glyph with lru
 - <csr-id-58dbfc96fab5c5109528d0088168d6a0db27a90b/> render example
 - <csr-id-26008f4df915ce48b26ad783bf52935a85b9d47e/> sdf texture gen

### Bug Fixes

 - <csr-id-0042f6658a483e9343fa5a2cf2dda898a1d7e411/> adjust default values
 - <csr-id-1eba8f7e9826721361e2536143d28ced10941e33/> bump versions
 - <csr-id-f8ac1f4c300525122c08d125587fbb43a195f88b/> assets
 - <csr-id-053003defdb84833d1e2d735bf2c7ccecfbe6fba/> remove fonts with copyright problem
 - <csr-id-ba851f0b0bb54412ad4b85ea52ee72ee51744aab/> context name
 - <csr-id-bdf4ac1da381962f4ce04372d3d60dd84af23978/> remove useless EOF check
 - <csr-id-8331ecd3b3a60c3b9e070e643efedd38a8c58ec7/> lint
 - <csr-id-4e1fd9161a3572585fec8dafc61323d697864fbc/> texture gen example
 - <csr-id-c34827e2ad9d4559a483f999aeb50033745e1706/> shader buffer size
 - <csr-id-a06bd725395b60b12fb3896c19a12ad4795b8194/> critical bug causes render wrong
 - <csr-id-9061e7abae2679bdd1e2dcd8e6888d05cdba4e5f/> texture use constant as size
 - <csr-id-051dd8007fd59fd28704d20418ef86370709c076/> use fake INFINITY in edt algorithm
 - <csr-id-e9c7dbc88dafdbe688ef72dd85e35e6fc17b3d3f/> shader
 - <csr-id-b719c0b7b3aee04ff1e1800bfb9f1c90562da3d3/> gamma value
 - <csr-id-3b91363e38f6e9a00ec1c48ccd50cd6be8c36c62/> handling \n and \r
 - <csr-id-f80b686181658a29694b0b050fb3c64ff5b8ccdb/> sdf generation params
 - <csr-id-1dcfcbd5b64dae8f5d416cd734a2fb52b725c2f2/> remove `page` in uniforms
 - <csr-id-33265928eb2547fc5f0890b05259c4cf4bbb8744/> wired border around rendered glyph
 - <csr-id-5691faf1cff2a3a2ad9d6815ead206a1f0bf6fff/> texture format
 - <csr-id-e4dcdfd72639186ca0a22b4131ae9b25eaa414bc/> tidy up

### Other

 - <csr-id-10c52143972473b87f05dd50e8d966a2b9a3e5c1/> move font assets into examples/

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 51 commits contributed to the release over the course of 208 calendar days.
 - 47 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Add ignore files in package ([`f4eda51`](https://github.com/Icemic/huozi-rs/commit/f4eda510ecba5c0a56c4006798ba3b6021005fb0))
    - Add package metadata ([`760bfea`](https://github.com/Icemic/huozi-rs/commit/760bfea80da5cc408f5b083550512ffd2bf30d5a))
    - Adjust default values ([`0042f66`](https://github.com/Icemic/huozi-rs/commit/0042f6658a483e9343fa5a2cf2dda898a1d7e411))
    - Bump versions ([`1eba8f7`](https://github.com/Icemic/huozi-rs/commit/1eba8f7e9826721361e2536143d28ced10941e33))
    - Assets ([`f8ac1f4`](https://github.com/Icemic/huozi-rs/commit/f8ac1f4c300525122c08d125587fbb43a195f88b))
    - Move font assets into examples/ ([`10c5214`](https://github.com/Icemic/huozi-rs/commit/10c52143972473b87f05dd50e8d966a2b9a3e5c1))
    - Remove fonts with copyright problem ([`053003d`](https://github.com/Icemic/huozi-rs/commit/053003defdb84833d1e2d735bf2c7ccecfbe6fba))
    - Use open source fonts as samples ([`937c6f5`](https://github.com/Icemic/huozi-rs/commit/937c6f5c798b709f84e0f4e53e5ca83522db0054))
    - Feat: parse to String instead of &str fix: unescape ([`8c527cb`](https://github.com/Icemic/huozi-rs/commit/8c527cbbb6d1a74a036d1ac283ce115390ea9c1b))
    - Huozi add parse api ([`e00d96b`](https://github.com/Icemic/huozi-rs/commit/e00d96b5bfc72696d8d5fc33f9c47c1522794ca1))
    - Adjust parse return value ([`6ae68e7`](https://github.com/Icemic/huozi-rs/commit/6ae68e7c622ca8b57384912d1c71c3bb18dbe044))
    - Context name ([`ba851f0`](https://github.com/Icemic/huozi-rs/commit/ba851f0b0bb54412ad4b85ea52ee72ee51744aab))
    - Remove useless EOF check ([`bdf4ac1`](https://github.com/Icemic/huozi-rs/commit/bdf4ac1da381962f4ce04372d3d60dd84af23978))
    - Move parser tests to mod ([`9ba08fa`](https://github.com/Icemic/huozi-rs/commit/9ba08faca72090b1a79cf614a72c4e4ca90ff3db))
    - Parser ([`a5ddb20`](https://github.com/Icemic/huozi-rs/commit/a5ddb201b3f387f503102a571aed6a4d907c7465))
    - Move layout func to Huozi impl ([`d7b3239`](https://github.com/Icemic/huozi-rs/commit/d7b3239421213b6312b31940bb0cd20608e9fb2c))
    - Lint ([`8331ecd`](https://github.com/Icemic/huozi-rs/commit/8331ecd3b3a60c3b9e070e643efedd38a8c58ec7))
    - Warn if glyph not exists ([`15d65f1`](https://github.com/Icemic/huozi-rs/commit/15d65f1df63693709a76d86d22dba99d81727211))
    - Properly handles characters larger than 1em in width ([`4645e32`](https://github.com/Icemic/huozi-rs/commit/4645e32f18284fc12ac3cc2e1ae27a3cc07a93c6))
    - Texture gen example ([`4e1fd91`](https://github.com/Icemic/huozi-rs/commit/4e1fd9161a3572585fec8dafc61323d697864fbc))
    - Shader buffer size ([`c34827e`](https://github.com/Icemic/huozi-rs/commit/c34827e2ad9d4559a483f999aeb50033745e1706))
    - Fix ([`35f1a4a`](https://github.com/Icemic/huozi-rs/commit/35f1a4ae6e4056d8f8e7c789a58423ecf0942158))
    - Make `wgpu` optional ([`5fb4c2f`](https://github.com/Icemic/huozi-rs/commit/5fb4c2f4453a9a166c376a5e39e23df0e0b2b16f))
    - Better texture generation example ([`e48cda3`](https://github.com/Icemic/huozi-rs/commit/e48cda3a49bfa22587cbe1a5cc16236057d4f54c))
    - Critical bug causes render wrong ([`a06bd72`](https://github.com/Icemic/huozi-rs/commit/a06bd725395b60b12fb3896c19a12ad4795b8194))
    - Texture use constant as size ([`9061e7a`](https://github.com/Icemic/huozi-rs/commit/9061e7abae2679bdd1e2dcd8e6888d05cdba4e5f))
    - Implement color to [f32;4] with trait ([`6ca41fc`](https://github.com/Icemic/huozi-rs/commit/6ca41fc8a66aab188480365671e60afc15a636e3))
    - Support stroke and shadow ([`b204ed9`](https://github.com/Icemic/huozi-rs/commit/b204ed93a6c9630469e1a56e19c4844fdde826f8))
    - Add layouting time log ([`d3057d7`](https://github.com/Icemic/huozi-rs/commit/d3057d794ca712edb51a24da58ee3d9466de4e87))
    - Move color and buffer to vertex attribute ([`3e69da1`](https://github.com/Icemic/huozi-rs/commit/3e69da196173c1439183432b4deb0a434240da19))
    - Use fake INFINITY in edt algorithm ([`051dd80`](https://github.com/Icemic/huozi-rs/commit/051dd8007fd59fd28704d20418ef86370709c076))
    - Shader ([`e9c7dbc`](https://github.com/Icemic/huozi-rs/commit/e9c7dbc88dafdbe688ef72dd85e35e6fc17b3d3f))
    - Add features ([`810413d`](https://github.com/Icemic/huozi-rs/commit/810413de40e7129c2fe65127c69b8f029f7ff204))
    - Switch font extractor via features ([`f3418f8`](https://github.com/Icemic/huozi-rs/commit/f3418f8fa02b8920a8bd0d71b11fea713d227571))
    - Gamma value ([`b719c0b`](https://github.com/Icemic/huozi-rs/commit/b719c0b7b3aee04ff1e1800bfb9f1c90562da3d3))
    - Font extractor, ab_glyph implement ([`9a54ad6`](https://github.com/Icemic/huozi-rs/commit/9a54ad624b028ea8880092da883433ed775c6b39))
    - Fix ([`c7f553b`](https://github.com/Icemic/huozi-rs/commit/c7f553bff9f40a1dde50b24c2411975bbad27cbe))
    - Handling \n and \r ([`3b91363`](https://github.com/Icemic/huozi-rs/commit/3b91363e38f6e9a00ec1c48ccd50cd6be8c36c62))
    - Better texture vertex calculation with font size ([`1d490c6`](https://github.com/Icemic/huozi-rs/commit/1d490c64ca5ba4cc9cc035bc168b2434ac21d128))
    - Log font load time and sdf texture gen time ([`94d51a0`](https://github.com/Icemic/huozi-rs/commit/94d51a074b56cc8b6235e21a3a964c88b6d7eb1c))
    - Sdf generation params ([`f80b686`](https://github.com/Icemic/huozi-rs/commit/f80b686181658a29694b0b050fb3c64ff5b8ccdb))
    - Remove `page` in uniforms ([`1dcfcbd`](https://github.com/Icemic/huozi-rs/commit/1dcfcbd5b64dae8f5d416cd734a2fb52b725c2f2))
    - Wired border around rendered glyph ([`3326592`](https://github.com/Icemic/huozi-rs/commit/33265928eb2547fc5f0890b05259c4cf4bbb8744))
    - Basic layout ([`8e935ea`](https://github.com/Icemic/huozi-rs/commit/8e935ea48ce1f8aa8e3232e84a32392e0cbd28d9))
    - Tidy up codebase ([`87723c4`](https://github.com/Icemic/huozi-rs/commit/87723c4bb9866edf7dc3f5e1149d363a17f4357a))
    - Texture format ([`5691faf`](https://github.com/Icemic/huozi-rs/commit/5691faf1cff2a3a2ad9d6815ead206a1f0bf6fff))
    - Tidy up ([`e4dcdfd`](https://github.com/Icemic/huozi-rs/commit/e4dcdfd72639186ca0a22b4131ae9b25eaa414bc))
    - Cache sdf glyph with lru ([`9d8b9dc`](https://github.com/Icemic/huozi-rs/commit/9d8b9dcdd83c18efc073ebb9b13de5cc1e8ef58b))
    - Render example ([`58dbfc9`](https://github.com/Icemic/huozi-rs/commit/58dbfc96fab5c5109528d0088168d6a0db27a90b))
    - Sdf texture gen ([`26008f4`](https://github.com/Icemic/huozi-rs/commit/26008f4df915ce48b26ad783bf52935a85b9d47e))
    - Initial commit ([`810f26f`](https://github.com/Icemic/huozi-rs/commit/810f26f5440f1d97e84189ee7f680ff36bfc590e))
</details>

