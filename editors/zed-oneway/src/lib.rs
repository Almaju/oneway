use zed_extension_api as zed;

struct OnewayExtension;

impl zed::Extension for OnewayExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let path = worktree
            .which("oneway-lsp")
            .unwrap_or_else(|| "oneway-lsp".to_string());

        Ok(zed::Command {
            command: path,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(OnewayExtension);
