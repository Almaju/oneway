# Oneway Editor Support

## Zed

### Syntax Highlighting

1. In Zed, open the command palette and run "Install Dev Extension"
2. Select the `editors/zed-oneway` directory

### Language Server (diagnostics)

1. Build and install the LSP:
   ```
   just install-lsp
   ```

2. Add to your Zed `settings.json` (Cmd+, or `zed: open settings`):
   ```json
   {
     "lsp": {
       "oneway-lsp": {
         "binary": {
           "path": "oneway-lsp"
         }
       }
     },
     "languages": {
       "Oneway": {
         "language_servers": ["oneway-lsp"]
       }
     }
   }
   ```

3. Reopen any `.ow` file — you'll get real-time error highlighting for:
   - Lex errors (invalid characters, attempted comments)
   - Parse errors (syntax mistakes)
   - Sort-order violations (unsorted fields, functions, imports, etc.)
