# lib/host/

Modules the native driver links itself, never a program (decision host-folder): `native_host.warp` and its parts in
`native_host/` (window, sound, platform), which it uses by relative path. core/native/driver.warp links the prebuilt
`warp/native_host` at the first host question it does not answer itself (`host_export`).

They are standard modules (listed in `std_module_names`, prebuilt by `warp rebuild`), but the library word index skips
this folder: their names (`width`, `height`, `held`) are common words, and a program naming one without defining it
must never link the window and open an SDL window.

Add a module here only when the driver links it and no program `use`s it; one programs use belongs in `lib/`.
More: wiki standard-library.md "Library folders".
