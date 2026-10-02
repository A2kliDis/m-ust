pub mod shazam;
pub mod acoustid;

pub use shazam::{recognize_with_shazam, fetch_cover, CoverPixels};
pub use acoustid::{recognize_with_acoustid, AcoustIdResult};
