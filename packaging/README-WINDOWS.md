# ScholarPress on Windows with Goose

1. Extract the archive.
2. Open PowerShell and run:

   ```powershell
   .\setup-goose.ps1 -ProjectPath "C:\Projects\dissertation"
   ```

3. Open Goose Desktop or run `goose session` from the project directory.
4. Enable the `ScholarPress` extension if it is not already enabled.

The setup script writes Goose's shared configuration at
`$HOME\.config\goose\config.yaml`, so the extension is available to both
Goose Desktop and Goose CLI. It creates project-local workspaces at
`<project>\.scholarpress\workspaces` and creates a timestamped config backup
before updating the extension entry.

## Options

```powershell
.\setup-goose.ps1 `
  -ProjectPath "C:\Projects\dissertation" `
  -BundlePath "C:\Tools\scholarpress" `
  -CatalogPath "C:\src\scholarpress-catalog" `
  -GooseConfigPath "C:\Users\me\.config\goose\config.yaml" `
  -StartGoose
```

The bundled catalog, Typst, and Pandoc paths are used by default. Supply
`-CatalogPath`, `-TypstPath`, or `-PandocPath` to use development overrides.
The script validates all paths before changing the Goose configuration.

## Manual configuration

In Goose Desktop, use `Extensions` > `Add custom extension`, choose
`Command-Line Extension`, and point it at the extracted `sp-mcp.exe`. Set the
following environment variables:

```text
SCHOLARPRESS_CATALOG_PATH=<bundle>\catalog
SCHOLARPRESS_WORKSPACE_ROOT=<project>\.scholarpress\workspaces
SCHOLARPRESS_TYPST_PATH=<bundle>\bin\typst.exe
SCHOLARPRESS_PANDOC_PATH=<bundle>\bin\pandoc.exe
```

The same values can be entered through `goose configure`.
