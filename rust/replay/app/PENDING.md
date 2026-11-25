# Pending sequences of `state/app`

Check S (`check.mjs`) runs the sequences that do not reach a sibling `todo!()`. These sequences reach one, so gate G2 (slice 2.49) runs them.

| Sequence | Blocked by |
| --- | --- |
| `closeAnchoredUi` | `tabs`, `extensions`, `bookmarks`, `sidebar`, `profiles` (`apply` is `todo!()` in each) |
| `setProgramControl`, `setControlFromSettings`, `stopProgramControl`, and any `controlChanged` with `reason` while `cliControl` is on | `settings::apply` (`updateSettings`) |
| `noteTabReady`, and any server status `on` or `starting` after `startProgramControl` (`pushActiveTab`) | `shownReadyTab` reads the focused window, the shown page and the page readiness of `browser`, `peek` and `pages`; their `State` has no such fields yet |
| `installOrUpgradeCommand` | `settings::State` reads for `userTurnedControlOff`, and `finishAutoInstall` calls `settings::apply`; it needs the `ArdiaCli` and document reads of the host |
| `showAppMenu` | `commands` (`appMenuItems` reads `COMMANDS`; `runCommand`) and a result action for `showMenu` |
| `startAppIntegration`, `startNativeSync`, `startScripting`, `startNotificationResponses` | They wire other folders and write no store of `app`; slice 2.49 checks them in the running app |
