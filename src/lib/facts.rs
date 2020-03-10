#![deny(clippy::all)]

use serde::Serialize;

#[derive(Serialize)]
pub struct Facts {}
impl Default for Facts {
    fn default() -> Self {
        Self {}
    }
}
