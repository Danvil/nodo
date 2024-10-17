#[derive(Debug, Clone, Copy)]
pub enum RuntimeControl {
    /// Request the runtime to stop. It may take a while for the runtime to shut down as codelets
    /// will finish stepping and stop will be called for all active codelets.
    RequestStop,
}
