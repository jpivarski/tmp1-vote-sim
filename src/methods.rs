mod method;
mod method_tracker;
mod plurality;
mod rangevoting;
mod tallies;

pub use method::{ElectResult, Method, Strategy, WinnerAndRunnerup};
pub use method_tracker::MethodTracker;
pub use plurality::Plurality;
pub use rangevoting::RangeVoting;
