//! Interface for instruments and effects that live outside the engine, such
//! as hosted CLAP plugins. The UI thread creates them and hands the audio-thread half
//! (an [`AudioProcessor`]) to the engine with `Command::AttachProcessor`.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PluginEventKind {
    NoteOn { velocity: f32 },
    NoteOff,
    AllNotesOff,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PluginEvent {
    /// Frame offset from the start of the buffers passed to `process`.
    pub offset: u32,
    pub pitch: u8,
    pub kind: PluginEventKind,
}

/// The audio-thread half of a hosted instrument or effect.
pub trait AudioProcessor: Send {
    /// Renders `left.len()` frames and adds them into `left`/`right`.
    /// Events are sorted by offset.
    fn process(&mut self, events: &[PluginEvent], left: &mut [f32], right: &mut [f32]);

    /// As an effect: replaces `left`/`right` with what the plugin makes of
    /// them. Leaves them alone if it can't.
    fn process_effect(&mut self, _left: &mut [f32], _right: &mut [f32]) {}

    /// Called on the audio thread right before the processor is handed back
    /// to the UI thread to be dropped (e.g. to stop processing).
    fn suspend(&mut self) {}
}
