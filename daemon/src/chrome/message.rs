#[derive(Debug, Clone)]
pub enum ChromeMessage {
    ActivatePlaylist {
        playlist_id: String,
    },
    ActivateTab {
        tab_id: String,
        playlist_id: String,
    },
    Pause,
    Resume,
    /// Pause when rotating, resume when paused.
    ToggleRotation,
    NextTab,
    PreviousTab,
    RefreshTab {
        tab_id: String,
    },
    RecreateTab {
        tab_id: String,
    },
    CloseTab {
        tab_id: String,
    },
    GetStatus,
    /// Put something on screen and hold it there.
    ///
    /// Without a number of seconds the hold has no end, and only `EndTakeover` clears it. That is
    /// what a button that puts the calendar up and takes it away again needs.
    Takeover {
        tab_id: Option<String>,
        stinger: Option<String>,
        seconds: Option<u64>,
    },
    /// Return to whatever the playlist was showing before a takeover.
    EndTakeover,
    Shutdown,
}
