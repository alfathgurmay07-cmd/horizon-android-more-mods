# User-supplied HNMOD package references

The following two prebuilt bundles were inspected from the supplied files. Their ZIP integrity, `horizon-native.json`, mixin ID/version and Horizon revision pins agree.

| Package | Version | Horizon revision | SHA-256 |
|---|---:|---|---|
| Spatial GUI | 0.2.1 | `66f3981f522b24c250b21523a70080965b940424` | `f130bac3c85fe2295aedf5d2f92d8d6262be2003d9b791a9771ee06e97886345` |
| Show Me Your Skin | 2.0.3 | `f319701e8c9ab657ff562250b3ba07b2d654e3bc` | `884a49c001ff2b346399503bfb5a7abb986bdb753111343b362ad1aab9b5c9ec` |

Both target Minecraft 26.3 and pin horizon-mixins revision `e8cd0aa38cffef49cb2e088ef716baa107148c02`. The two bundles target different Horizon commits; build/use each with its matching engine source and do not assume they can be enabled together.

The original prebuilt `.hnmod` binaries are not mirrored by this metadata-only reference. The source files supplied in chat remain available as downloads in that conversation. Archive checks are not a full native compile or a graphical runtime test.
