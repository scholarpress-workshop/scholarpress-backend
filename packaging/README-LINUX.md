# ScholarPress on Linux and WSL

1. Extract the archive.
2. Run `./start-scholarpress.sh /path/to/openwork-workspace`.
3. Add the printed `http://127.0.0.1:8765/mcp` URL in OpenWork under `Settings` > `Extensions` > `Add Custom App`.

The launcher stores ScholarPress data under `.scholarpress` inside the selected OpenWork workspace. Override bundled tools with `SCHOLARPRESS_TYPST_PATH` and `SCHOLARPRESS_PANDOC_PATH` before starting the launcher.
