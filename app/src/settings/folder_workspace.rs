use settings::{
    macros::define_settings_group, RespectUserSyncSetting, SupportedPlatforms, SyncToCloud,
};

define_settings_group!(FolderWorkspaceSettings, settings: [
    default_command_for_new_workspaces: DefaultCommandForNewWorkspaces {
        type: String,
        default: "claude".to_string(),
        supported_platforms: SupportedPlatforms::ALL,
        sync_to_cloud: SyncToCloud::Globally(RespectUserSyncSetting::Yes),
        private: false,
        toml_path: "folder_workspaces.default_command_for_new_workspaces",
        description: "Command auto-run in new tabs of newly-created folder workspaces. Set to empty string to disable. Existing workspaces keep their per-workspace value.",
    }
]);
