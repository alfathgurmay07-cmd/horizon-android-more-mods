# Punchy

- Native mod ID: `punchy`
- Supplied HNMOD version: `0.1.2`
- Original archive: `punchy-rust-0.1.2.hnmod`
- Pinned Horizon revision: `81ae4a7d8f27d38e5cabbdcda0a06b2c4b000045`
- Pinned horizon-mixins revision: `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`
- Required hook API: `1`

## Port status

Integrated into the Android client as a statically bundled **preview**. The Punchy engine is compiled into the Android client. Its default animation, geometry, and compatibility tuning files are shipped as the required `Punchy Built-in Animations` resource pack, layered below player-selected packs so a custom pack can override any matching resource path. The port wires procedural first-person hand motion (idle, walk/sprint, jump/land, flight, swimming, climbing/crawling, horse riding, and boat rowing) into Punchy, alongside first-person hand/item action geometry, animation ticks, keybinds, settings, and successful-use/mining triggers. Punchy Moves is reserved for third-person player motion and elytra trails. The required built-in pack is installed under the game resource root and pinned below player-selected resource packs. A custom pack can replace any built-in animation, geometry, or tuning asset by supplying the same resource path. It is not dynamically loaded from a runtime mods directory.

This is not complete Punchy renderer parity. Armour/body animation, every custom animation condition, and full Java/Fabric behaviour are not claimed as implemented. The Mods description identifies it as a preview; the short Mods title keeps the ON/OFF state visible.
