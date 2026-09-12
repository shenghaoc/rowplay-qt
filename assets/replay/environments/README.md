# Replay environment assets

The replay remains a generic illustrative venue. These files provide local
surface response only; they do not represent a recorded route, venue, weather,
or time of day.

No asset is fetched at runtime. Low and Medium retain procedural surfaces;
High and Ultra load the local optimized derivatives.

## Per-tier payload

What each sport actually loads at each tier. High binds diffuse + roughness per
set; Ultra adds the OpenGL normal and, for SkiErg, an Ultra-only timber terrace
set. Low and Medium bind nothing, so venue identity never waits on an image
decode.

| Sport   | Sets (High / Ultra) | High            | Ultra            |
| ------- | ------------------- | --------------- | ---------------- |
| RowErg  | 8 / 8               | 16 req, 1.4 MiB | 24 req, 2.3 MiB  |
| SkiErg  | 4 / 5               | 8 req, 0.7 MiB  | 15 req, 1.46 MiB |
| BikeErg | 4 / 4               | 8 req, 0.6 MiB  | 12 req, 0.8 MiB  |

RowErg carries the most because the river valley dresses banks, shoreline,
island, vegetation, decking, and paths. SkiErg adds a rock shoulder at High and
a timber spectator terrace at Ultra. BikeErg uses concrete around a separate
oak course instead of applying one asphalt map to the whole venue.

`renderer3d.test.ts` pins these request counts and a payload ceiling per sport,
alongside mesh and instance ceilings, so densifying a venue or adding a set
cannot pass unnoticed. Update this table in the same commit as any budget change.

## Snow 02

- Source: [Poly Haven — Snow 02](https://polyhaven.com/a/snow_02)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

The shipped 512 px JPEGs are resized and recompressed derivatives:

| File                             | Purpose                          | SHA-256                                                            |
| -------------------------------- | -------------------------------- | ------------------------------------------------------------------ |
| `snow-02/snow-diffuse-512.jpg`   | High/Ultra snow colour variation | `0ae627f87a222d82dfe2f311ef1ba427432fdc58268d4cbcfbbff7764f4f6492` |
| `snow-02/snow-roughness-512.jpg` | High/Ultra snow roughness        | `507717de6130d18c2c057ffbc4ae1575a32a5ea26e338deff6827032c4d5399f` |
| `snow-02/snow-normal-gl-512.jpg` | Ultra OpenGL normal detail       | `5bc9092efa6c2d73cd6b0f04c9ed790b4c3beeed4400938911eb405918a5cc0e` |

Original Poly Haven MD5 values recorded by its public asset API:

- diffuse: `fc54766c6b36ff298699115a619d440b`
- roughness: `1dbae0269e53dbf80d4fd1c4335f25a2`
- OpenGL normal: `f16b5701f9ad521cdd6af10c1d6d2b48`

## Rock 01

- Source: [Poly Haven — Rock 01](https://polyhaven.com/a/rock_01)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-26

The shipped 512 px JPEGs are resized and recompressed derivatives. They detail
the High/Ultra SkiErg rock-shoulder outcrops and do not depict a real mountain
or venue.

| File                                | Purpose                            | SHA-256                                                            |
| ----------------------------------- | ---------------------------------- | ------------------------------------------------------------------ |
| `rock-01/rock-01-diffuse-512.jpg`   | High/Ultra outcrop colour          | `1ee06fb1752c9eab2b21d7e89e7912efd5981b923e72a6e30f5ce4c3e3bc5898` |
| `rock-01/rock-01-roughness-512.jpg` | High/Ultra outcrop roughness       | `511ba59e965cdc61ee16d30cfc5b5e19cec2e99986945cc48cad0e14767766d0` |
| `rock-01/rock-01-normal-gl-512.jpg` | Ultra OpenGL outcrop normal detail | `6025fd703dd261b8edcbb39b21223a850f291b1b8e7564f80d7037b48160b087` |

Original Poly Haven MD5 values recorded by its public asset API:

- diffuse: `5897cd4496982b03cad0c3e2358486c0`
- roughness: `d524d822733ce9958a74ef89285ca99b`
- OpenGL normal: `d5a1d0797aaeb1ac3520504911058903`

## Wood Floor

- Source: [Poly Haven — Wood Floor](https://polyhaven.com/a/wood_floor)
- Creator: Dimitrios Savva
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-26

The shipped 512 px JPEGs are resized and recompressed derivatives. High/Ultra
BikeErg uses them only for its generic timber track; custom tangential UVs keep
the boards aligned to the lap instead of slicing across bends.

| File                                      | Purpose                          | SHA-256                                                            |
| ----------------------------------------- | -------------------------------- | ------------------------------------------------------------------ |
| `wood-floor/wood-floor-diffuse-512.jpg`   | High/Ultra track-board colour    | `763103fd5fb60cc18b1f3764bff98fbb666ab0c74512ec90f8f12626f59cb50e` |
| `wood-floor/wood-floor-roughness-512.jpg` | High/Ultra track roughness       | `6a639e0d23c6133df3a7906a8aa0ebdadfe9dba0472221dc21d7dc16fe2bcdc4` |
| `wood-floor/wood-floor-normal-gl-512.jpg` | Ultra OpenGL board normal detail | `0096fb6be668196d968459efd93cd482e16a6f62fd8b86807346930fb6a58613` |

Original Poly Haven MD5 values recorded by its public asset API:

- diffuse: `b7e927d2bf2f8f103820ff3890c8407c`
- roughness: `36146634f1dbd1bc30cd071857584c10`
- OpenGL normal: `620f174d2c09b579c5d02d37b2106668`

## Aerial Grass Rock

- Source: [Poly Haven — Aerial Grass Rock](https://polyhaven.com/a/aerial_grass_rock)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Shipped 512 px derivatives detail High/Ultra RowErg grass banks, shoreline
grass, and the basin island's lawn.

| File                                                    | Purpose                         | SHA-256                                                            |
| ------------------------------------------------------- | ------------------------------- | ------------------------------------------------------------------ |
| `aerial-grass-rock/aerial-grass-rock-diffuse-512.jpg`   | High/Ultra grass-bank colour    | `0ba2f50ac5472ea90f9a7110ae8805402592997e0f0c6e5f0ac33c8f3a3e2e34` |
| `aerial-grass-rock/aerial-grass-rock-roughness-512.jpg` | High/Ultra grass-bank roughness | `dc7c447546f9b9439b57d1a1ee730f6785c1d6189891ae2d9211949982f4e8fb` |
| `aerial-grass-rock/aerial-grass-rock-normal-gl-512.jpg` | Ultra OpenGL grass-bank normals | `7186ea4554c8a657869af2e5441503126b1060d4008798c2526bf229d58af653` |

Original Poly Haven MD5 values:

- diffuse: `e920ce36afd0abff000b8366d3d768d3`
- roughness: `f78c5cdc565f990299ae7c5a81f68cf7`
- OpenGL normal: `c8aa4c4f09b113cc7edef89ddeaccad9`

## Forest Ground 01

- Source: [Poly Haven — Forest Ground 01](https://polyhaven.com/a/forrest_ground_01)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Note: Poly Haven's asset id keeps the historical spelling `forrest_ground_01`.
Used for High/Ultra RowErg earth banks, shoreline, and island ground.

| File                                                    | Purpose                         | SHA-256                                                            |
| ------------------------------------------------------- | ------------------------------- | ------------------------------------------------------------------ |
| `forrest-ground-01/forrest-ground-01-diffuse-512.jpg`   | High/Ultra earth-bank colour    | `655b2425d9b18d150c4a7854002e9c71ed1c321f72a8b2f929436ea557af0acb` |
| `forrest-ground-01/forrest-ground-01-roughness-512.jpg` | High/Ultra earth-bank roughness | `632e6bc9faf39d25afae2af708524e7a30e4daf05eba6eee7f6b66923e79958d` |
| `forrest-ground-01/forrest-ground-01-normal-gl-512.jpg` | Ultra OpenGL earth-bank normals | `07b8bb3fa23ceb43b6f5ebdccdb49786f78222ab16a33941597c577b87e25938` |

Original Poly Haven MD5 values:

- diffuse: `236e7d928f5e357a194fd92de189cbe4`
- roughness: `72dacf5b829cafc025f08ca64e92e5eb`
- OpenGL normal: `ba4265df25aea293913d004b69ef9ab0`

## Brown Planks 03

- Source: [Poly Haven — Brown Planks 03](https://polyhaven.com/a/brown_planks_03)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Used for High/Ultra RowErg launch-dock and campus decking, regatta pavilion
bodies, island structures, BikeErg track boards, and the Ultra SkiErg spectator
terrace.

| File                                                | Purpose                            | SHA-256                                                            |
| --------------------------------------------------- | ---------------------------------- | ------------------------------------------------------------------ |
| `brown-planks-03/brown-planks-03-diffuse-512.jpg`   | High/Ultra dock/pavilion colour    | `3b53a63afb2a1ce6ee1fa45e6c7bab5b0241cb5f03f267e8c223b3e7694b66c2` |
| `brown-planks-03/brown-planks-03-roughness-512.jpg` | High/Ultra dock/pavilion roughness | `edc3f1996678a5d3ba5a306180ff6b4a0d0364b9cbdcf4e2ef0362cafbf0e3ee` |
| `brown-planks-03/brown-planks-03-normal-gl-512.jpg` | Ultra OpenGL dock/pavilion normals | `6ef4b139a08ef9f0bf75e84c0182176d5866fa2e4364c0ba5685b45e16834f37` |

Original Poly Haven MD5 values:

- diffuse: `6c9fdc21afcdf171d0a39999392a70f4`
- roughness: `7c205220c96d8fc72b1fae2ae8d7ccb6`
- OpenGL normal: `4944cb903b7d7c4029310deb8a51d69e`

## Bark Brown 01

- Source: [Poly Haven — Bark Brown 01](https://polyhaven.com/a/bark_brown_01)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Used for High/Ultra RowErg and SkiErg pine trunks.

| File                                            | Purpose                    | SHA-256                                                            |
| ----------------------------------------------- | -------------------------- | ------------------------------------------------------------------ |
| `bark-brown-01/bark-brown-01-diffuse-512.jpg`   | High/Ultra trunk colour    | `3a5c636bf35244e6bf436b5a5da999f29efffa04882c24d42387f80ec9e21049` |
| `bark-brown-01/bark-brown-01-roughness-512.jpg` | High/Ultra trunk roughness | `c8392e995cb7040fc022fba4099dcbced67662a3a52ba99c4b7452c627e4b7c0` |
| `bark-brown-01/bark-brown-01-normal-gl-512.jpg` | Ultra OpenGL trunk normals | `572f31a0c68bd0ecd3b1680ae939cb669db901419850525edf0261caf5143170` |

Original Poly Haven MD5 values:

- diffuse: `b6d5dcde10b7cd1b36d70cd33a34724a`
- roughness: `b7a1aa42ecf0b30aba7f587769d13910`
- OpenGL normal: `8ae9907be7ce562c11aab619615d30d4`

## Forest Leaves 04

- Source: [Poly Haven — Forest Leaves 04](https://polyhaven.com/a/forest_leaves_04)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Used for High/Ultra pine canopies (Row + Ski), RowErg bank and island trees, and
the wooded shoreline hills.

| File                                                  | Purpose                           | SHA-256                                                            |
| ----------------------------------------------------- | --------------------------------- | ------------------------------------------------------------------ |
| `forest-leaves-04/forest-leaves-04-diffuse-512.jpg`   | High/Ultra canopy/shore colour    | `0c4a19d3e4f81ac2bd28475d1c0c15e0b951cf68e6b35a72ec48bdd790331b47` |
| `forest-leaves-04/forest-leaves-04-roughness-512.jpg` | High/Ultra canopy/shore roughness | `001bfa94e88d2debe0b3f91c8aa72577a6d4db4e79392ba9d0354b1c86d5b925` |
| `forest-leaves-04/forest-leaves-04-normal-gl-512.jpg` | Ultra OpenGL canopy/shore normals | `d3b4bed87b801470ec1567bb582a618883ce06e6051298db38731a28362c4644` |

Original Poly Haven MD5 values:

- diffuse: `4ab1368d2d0d7fee652caa5f71f706bf`
- roughness: `60a2f7467861802b99c55480f8ac0801`
- OpenGL normal: `ca21a728629bf25d9dcbe327e0f005db`

## Leafy Grass

- Source: [Poly Haven — Leafy Grass](https://polyhaven.com/a/leafy_grass)
- Creator: Charlotte Baglioni
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Used for High/Ultra RowErg reed beds and shoreline grass.

| File                                        | Purpose                   | SHA-256                                                            |
| ------------------------------------------- | ------------------------- | ------------------------------------------------------------------ |
| `leafy-grass/leafy-grass-diffuse-512.jpg`   | High/Ultra reed colour    | `92cd6d767e9502ab4d8b1aed49b6f40e16d4b9c761020e833681fabe371506d4` |
| `leafy-grass/leafy-grass-roughness-512.jpg` | High/Ultra reed roughness | `6cca720bf619d284bedd2dac4be5a6126a3abfa566fc410312efdb9f0392391d` |
| `leafy-grass/leafy-grass-normal-gl-512.jpg` | Ultra OpenGL reed normals | `946fc165aa732befa8bbfe55073c66d1662c2463998bc24cfeb37afd32f8afaa` |

Original Poly Haven MD5 values:

- diffuse: `0dbc071e91d6905edfcfbe8eb785a1ab`
- roughness: `05f6bb1383e12b327727d313c5899cf2`
- OpenGL normal: `8279e096e204ea326d57a318869f99df`

## Dry River Pebbles

- Source: [Poly Haven — Dry River Pebbles](https://polyhaven.com/a/dry_river_pebbles)
- Creator: Amal Kumar
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Used for the High/Ultra RowErg shingle waterline between water and earth bank,
and the basin island's beach.

| File                                                    | Purpose                        | SHA-256                                                            |
| ------------------------------------------------------- | ------------------------------ | ------------------------------------------------------------------ |
| `dry-river-pebbles/dry-river-pebbles-diffuse-512.jpg`   | High/Ultra waterline colour    | `717c0d224a024e72909504dbbe9e26f85914894067b29c909a5241df0b526662` |
| `dry-river-pebbles/dry-river-pebbles-roughness-512.jpg` | High/Ultra waterline roughness | `cebf7ff77629c6ea308bfc46358314a515aeb59d076153a58712a87159cd1470` |
| `dry-river-pebbles/dry-river-pebbles-normal-gl-512.jpg` | Ultra OpenGL waterline normals | `a35aeba24d6e3744771053a5e9074d29a89afbee185bfc6917dc87245085db6d` |

Original Poly Haven MD5 values:

- diffuse: `970ed34655bdf437d6016b130742668f`
- roughness: `8e07eb86e5850fc7568344009b2065ef`
- OpenGL normal: `309cc625d2f3834f281720bdd1d8cdbe`

## Row river-valley material system

RowErg water remains an authored clear-coat basin with procedural sheen and
normals. High/Ultra surroundings form one coherent river valley:

| Scene element                    | CC0 set              |
| -------------------------------- | -------------------- |
| Grass banks + island lawn        | Aerial Grass Rock    |
| Earth banks + island ground      | Forest Ground 01     |
| Shingle waterline + island beach | Dry River Pebbles    |
| Reed beds + shoreline grass      | Leafy Grass          |
| Wooded shoreline                 | Forest Leaves 04     |
| Bank and island trees            | Forest Leaves 04     |
| Pine canopies                    | Forest Leaves 04     |
| Pine trunks                      | Bark Brown 01        |
| Dock, decking, pavilions         | Brown Planks 03      |
| Shore-campus paths               | Cobblestone Floor 03 |

SkiErg High/Ultra pine trunks and canopies reuse Bark Brown 01 and Forest
Leaves 04 so the Nordic tree line matches the same woodland material language.
Snow 02 owns the piste, Rock 01 owns one authored shoulder, and Brown Planks 03
appears only at Ultra on the spectator terrace; that extra set is an intentional
composition-tier difference.

BikeErg High/Ultra separates materials by construction: Brushed Concrete 2 on
the arena slab and infield, Concrete Floor Painted on wall and staging bays,
Brown Planks 03 on the inner safety boards, and Wood Floor on the tangentially
unwrapped racing surface. The previous Clean Asphalt set is no longer shipped
or loaded.

## Brushed Concrete 2

- Source: [Poly Haven — Brushed Concrete 2](https://polyhaven.com/a/brushed_concrete_2)
- Creators: Dimitrios Savva (Photography), Dario Barresi (Processing)
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Used for the High/Ultra BikeErg velodrome infield floor.

| File                                                      | Purpose                       | SHA-256                                                            |
| --------------------------------------------------------- | ----------------------------- | ------------------------------------------------------------------ |
| `brushed-concrete-2/brushed-concrete-2-diffuse-512.jpg`   | High/Ultra concrete colour    | `e0552acd803f63c2e055aac8c096f5a1dddc4668cd3008017bcdc0f46a2e173f` |
| `brushed-concrete-2/brushed-concrete-2-roughness-512.jpg` | High/Ultra concrete roughness | `500e543fa6bf4a14a43233352ff6273b326ab63f0e49b5d0af14edb729e43f55` |
| `brushed-concrete-2/brushed-concrete-2-normal-gl-512.jpg` | Ultra OpenGL concrete normals | `18a8e69e29c18478b07ac00cdb4ceada80f3be3a5fdfd4a0b3b05824ba2fd7b4` |

Original Poly Haven MD5 values:

- diffuse: `a8dcaf190900eff56de4571eef76cb7d`
- roughness: `c77b5b219f98aa5e458d183c9700fde2`
- OpenGL normal: `83544a8457287355d3c854791b7e7915`

## Cobblestone Floor 03

- Source: [Poly Haven — Cobblestone Floor 03](https://polyhaven.com/a/cobblestone_floor_03)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Used for High/Ultra RowErg shore-campus paths.

| File                                                          | Purpose                     | SHA-256                                                            |
| ------------------------------------------------------------- | --------------------------- | ------------------------------------------------------------------ |
| `cobblestone-floor-03/cobblestone-floor-03-diffuse-512.jpg`   | High/Ultra cobble colour    | `999a8745fbf6996860e9c64995728c97185032389201931b29ff6f80691ff0e9` |
| `cobblestone-floor-03/cobblestone-floor-03-roughness-512.jpg` | High/Ultra cobble roughness | `daeab61257a199ae69efd7ecd278f7555ed5a68ef0ba4f43fa9f7e8c492e7281` |
| `cobblestone-floor-03/cobblestone-floor-03-normal-gl-512.jpg` | Ultra OpenGL cobble normals | `947b46c737dbfe5890e264636ec437be09665a8a227a11c8b8228eec4089c335` |

Original Poly Haven MD5 values:

- diffuse: `d5e23f634ea666527747ea22734d1984`
- roughness: `e136999af6faa42d51aef4ddfe9e7b87`
- OpenGL normal: `714091302c0177ff073ee385a07800e5`

## Concrete Floor Painted

- Source: [Poly Haven — Concrete Floor Painted](https://polyhaven.com/a/concrete_floor_painted)
- Creator: Rob Tuytel
- License: [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/)
- Source resolution used: Poly Haven's 1K JPEG maps
- Retrieved: 2026-07-25

Used for High/Ultra BikeErg infield staging pads and arena wall.

| File                                                              | Purpose                               | SHA-256                                                            |
| ----------------------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------ |
| `concrete-floor-painted/concrete-floor-painted-diffuse-512.jpg`   | High/Ultra painted-concrete colour    | `19c89793bd4a07d5f233799a0a372da2faa58b9ecf50d47220e762ba9101dd1e` |
| `concrete-floor-painted/concrete-floor-painted-roughness-512.jpg` | High/Ultra painted-concrete roughness | `0ba04433f0a61569d4e1d19eef1e947364ad43280b92ebb0e4c7bf5cd98cc613` |
| `concrete-floor-painted/concrete-floor-painted-normal-gl-512.jpg` | Ultra OpenGL painted-concrete normals | `20253d5ec67097b4ee522afd787e926396add5c55a2565bc40fe3917ccecb66a` |

Original Poly Haven MD5 values:

- diffuse: `07cee8ac8966a2f84fd7bc2ecb416de9`
- roughness: `5af13a648433a32f7b3b4d6ac51a4b4c`
- OpenGL normal: `95eeeb46f335e3b4ba76340193dc1b7d`
