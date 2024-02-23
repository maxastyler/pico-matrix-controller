#![no_std]
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct MatrixState {
    brightness: f32,
}

pub fn add(x: usize, y: usize) -> usize {
    x + y
}

#[cfg(test)]
mod test {
    #[test]
    fn is_true() {
        assert!(true)
    }
}
