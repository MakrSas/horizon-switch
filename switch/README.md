# Horizon for Nintendo Switch

[Русская версия](README.ru.md)

An unofficial port of [Horizon](https://github.com/ludovic111/horizon), a Rust reimplementation of Minecraft Java Edition, to the Nintendo Switch as homebrew. It plays as a full single-player game. Saves work, and it installs as an NSP with its own icon on the HOME menu.

It is not affiliated with Mojang, Microsoft or Nintendo. It ships no Minecraft game files: you make them yourself from your own copy of Minecraft Java Edition.

## Status

Tested on one Switch v1 (Erista) on firmware 22.5.0 with Atmosphère 1.12.1.

| Area | Status |
|---|---|
| Single player | Works: world creation, survival and creative, saving. |
| Frame rate | Up to 60 fps at render distance 6 in handheld mode; lower at longer distances and while new terrain generates. |
| Rendering | Vulkan through NVK (Mesa's open-source NVIDIA driver, via [mesa-switch](https://github.com/danfromtico/mesa-switch)), with OpenGL through devkitPro's Mesa as the fallback. |
| Controls | Joy-Con (attached or detached) and the Pro Controller, Bedrock-style layout, rebindable in the controller settings (Controller Binds). HD Rumble, with its own toggle for breaking blocks. The touch screen works in menus and inventories, and picks hotbar slots in the world. |
| Startup | About 45 s to the world: about 11 s to the title screen, then the world generator. The screen stays black for the first seconds. |
| Skins | Wear any Minecraft account's skin: Options → Skin Customization, type its nickname. Needs the internet once; the skin is kept for offline play. |
| Sound | None yet. |
| Multiplayer | Not supported. |
| Docked mode | Untested. |

## Installing

You need a Switch running [Atmosphère](https://github.com/Atmosphere-NX/Atmosphere) and an NSP installer such as [DBI](https://github.com/rashevskyv/dbi).

1. **Make the game files from your own Minecraft.** On a PC with Minecraft Java Edition 26.3 installed through the official launcher, build Horizon's `horizon-prepare` from this repository and run:

   ```bash
   cargo run --release -p horizon-prepare -- --prepare --root resources
   ```

   It reads the game files from your Minecraft installation (`--minecraft-dir` points elsewhere) and writes Horizon's resources to `resources/`.

2. **Pack them into one RomFS image** with devkitPro's `build_romfs` (part of switch-tools):

   ```bash
   switch/scripts/make-romfs.sh resources resources.romfs
   ```

   Copy `resources.romfs` to `sdmc:/switch/horizon/resources.romfs` on the SD card (about 600 MB). A plain `sdmc:/switch/horizon/resources/` folder works too, but loads several times slower.

3. **Install `Horizon.nsp`** from the [Releases](../../releases) page with DBI (or any NSP installer) and start Horizon from the HOME menu.

Worlds, settings and logs live in `sdmc:/switch/horizon/`. The log is `sdmc:/switch/horizon/logs/latest.log`.

**Optional overclock.** With [sys-clk](https://github.com/retronx-team/sys-clk) or a similar tool you can raise clocks for title ID `0100485952450000`. The CPU is what limits the game most.

## Controls

| Action | Button |
|---|---|
| Move / look | Left stick / right stick |
| Jump | A |
| Drop item | B |
| Swap offhand | X |
| Inventory | Y |
| Attack / break, use / place | ZR, ZL |
| Hotbar | L, R |
| Sprint, sneak | Left stick click, right stick click |
| Perspective, pick block, chat, advancements | D-pad |
| Player list | − |
| Pause | + |

Everything except pause can be rebound, and the debug screen (F3) can be given a button there too.

## Building

The build runs on Linux (tested on Ubuntu 24.04).

**You need:**

- [devkitPro](https://devkitpro.org/wiki/Getting_Started) with `devkitA64`, `libnx`, `switch-mesa`, `switch-libexpat`, `switch-zlib`, `switch-zstd` and `switch-tools`.
- A nightly Rust with `rust-src` (`rustup toolchain install nightly --component rust-src`). The tested one was `rustc 1.101.0-nightly (76456a688 2026-10-25)`; set `RUST_TOOLCHAIN` to pick another.
- NVK, built once: `switch/scripts/build-nvk.sh`. It needs Docker or Podman and about 20 GB of disk, and uses [bevy-horizon](https://github.com/JohnUzoka/bevy-horizon)'s build scripts and NVK patches. It stages `switch/build/nvk/lib/libvulkan_bh.a`.
- For the NSP: [hacbrewpack](https://github.com/rlaphoenix/hacBrewPack) and your console's `prod.keys` (dumped with Lockpick_RCM). Never commit or share the keys.

**Then, from the repository root:**

```bash
switch/scripts/build.sh                                  # switch/build/horizon.elf
switch/scripts/make-nsp.sh ~/switch/prod.keys            # switch/build/Horizon.nsp
```

The desktop build is unchanged: nothing under `switch/` is used unless these scripts run.

## How the port works

- **Target.** `aarch64-nintendo-switch-newlib.json`, a custom Rust target (`target_os = "horizon"`, newlib), linked by devkitA64 against libnx. `std` is built from source with `-Z build-std`, from a copy of the toolchain's `rust-src` patched by `std/std.patch`.
- **Patched crates** (`patches/`, wired in through the workspace's `[patch.crates-io]`): `libc` (newlib on AArch64 Horizon: made by `patch_libc.py`), `getrandom`, `ring`, `winit` (a `platform_impl/nx` backend: window, applet focus and exit, touch screen), `wgpu` and `wgpu-core` (the Vulkan and GLES backends on Horizon), and an `arboard` stub. `third_party/wgpu-hal` gets Horizon's EGL surface and NVK's VI surface.
- **nxshim** (`nxshim/nxshim.c`): C glue linked into the game: stubs newlib lacks (`getrandom`, `sysconf`, `dlopen`), pthread keys past newlib's limit, `rename` over an existing file, crash and abort reports to `fatal.txt`, the RomFS mount, threads spread over the cores, and the thread pointer that NVK's Rust code reads.
- **`crates/horizon-nx`**: safe wrappers for the game: controllers and HD Rumble, memory, RomFS, and the global allocator (bevy-horizon's lock-free small-block cache in front of newlib's malloc, whose single lock held every thread back).
- **Performance.** Thread priorities on Horizon (`crates/horizon-threads`), cached HUD item drawing, and the Switch's defaults in `settings.rs`.

## Credits

- [Horizon](https://github.com/ludovic111/horizon) by Ludovic, MIT.
- [mesa-switch](https://github.com/danfromtico/mesa-switch) by danfromtico and NaGaa95, and [bevy-horizon](https://github.com/JohnUzoka/bevy-horizon) by JohnUzoka (MIT or Apache-2.0): NVK for the Switch, its patches and build scripts, the wgpu-hal Vulkan changes for Horizon, and the small-block allocator cache.
- [libnx](https://github.com/switchbrew/libnx) and [devkitPro](https://devkitpro.org).
- The patched crates keep their own licenses: libc, getrandom, winit, wgpu (MIT or Apache-2.0), ring (ISC-style).

Minecraft is a trademark of Mojang Synergies AB. Nintendo Switch is a trademark of Nintendo.
