//! GPT editing actions, interface agnostic.
use structopt::clap::arg_enum;

arg_enum! {
    /// Supported formats for dumping/restoring the Gpt
    #[derive(Debug, Copy, Clone)]
    pub enum Format {
        Json,
    }
}
