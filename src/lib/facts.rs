#![deny(clippy::all)]

pub struct Facts {}
impl Default for Facts {
    fn default() -> Self {
        Self {}
    }
}
