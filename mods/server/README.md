# Server-side Horizon mods

Server-side plugin sources are kept separately from client/Android modules.

## HorizonVC

The `horizon-vc/` folder is a pinned submodule of https://github.com/Plugin-Horizon/horizon-vc.
It implements a server compatible with Simple Voice Chat protocol 20. The CI workflow runs its standalone core
protocol/routing tests, but does not yet claim that engine hooks compile or that real client audio works.

It requires the `Detimes/horizon` `mod-hooks-fake-players` engine branch while the required upstream hooks are pending.
It is not an Android client mod and is not included in the APK.
