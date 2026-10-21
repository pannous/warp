Vendoring is deactivated (kept for a possible offline agent later): main has no `vendor/` and no source replacement.
The machinery is kept as it is: the `vendor` branch and .github/workflows/offline-build-refresh.yml, which still
refreshes it when Cargo.lock changes on main; how to use it again: notes/cloud_offline_build.md.
