# shengjian-windows-host

Headless Windows host for the core. No Tauri/Vue; it demonstrates the intended
host boundary and the migration path.

## Commands

```powershell
cargo run -p shengjian-windows-host -- scan
cargo run -p shengjian-windows-host -- run --seconds 20
cargo run -p shengjian-windows-host -- run --seconds 20 --mappings .\button-mappings.json --profiles .\button-profiles.json
```

Add `--persist` if those imports should become the default persisted config.

`scan` lists paired remotes using the opaque token (not the Bluetooth address).
`run` connects, selects a CABLE endpoint, applies optional core mappings/profiles,
loads `%LOCALAPPDATA%\Shengjian\settings.json` for gain/endpoint, streams
`core_event` JSON lines, then disconnects cleanly.

Core mappings/profiles are loaded by `live_core_engine` from
`%LOCALAPPDATA%\Shengjian\button-mappings.json` and
`button-profiles.json`. Explicit `--mappings`/`--profiles` imports are transient
by default; pass `--persist` to write them back to the default paths.

`run --profiles <json>` also polls the foreground executable every 500ms and
reapplies the overlay only when the active profile changes. Verified on real
RC003 by opening Notepad while the host was running (`foreground profile changed`).

## Legacy migration

```powershell
cargo run -p shengjian-windows-host -- migrate `
  --out-dir .\migrated `
  --legacy-settings <settings.json> `
  --legacy-mappings <button-mappings.json> `
  --legacy-profiles <button-profiles.json>
```

Outputs:

- `settings.json` — `SettingsDocument`
- `button-mappings.json` — core `ButtonMappingSet`
- `button-profiles.json` — core `ButtonProfilesDocument`

Inputs are explicit paths; the host does not guess old product directories.
