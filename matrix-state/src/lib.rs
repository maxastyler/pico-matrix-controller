#![no_std]

#[derive(Default, Debug, Copy, Clone)]
#[repr(C, align(4))]
pub struct RGB8 {
    pub padding: u8,
    pub b: u8,
    pub r: u8,
    pub g: u8,
}

pub trait MatrixState {
    type Message;
    /// The amount of time to pause between frames, in milliseconds
    fn frame_spacing(&self) -> u64;
    fn update<D: MatrixDisplay>(&mut self, message: Option<Self::Message>, display: &mut D);
}

pub trait MatrixDisplay {
    fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut RGB8>;
    fn get(&self, row: usize, col: usize) -> Option<&RGB8>;
    fn size(&self) -> (usize, usize);

    fn iter_coords_helper(rows: usize, cols: usize) -> impl Iterator<Item = (usize, usize)> {
        (0..rows).flat_map(move |r| (0..cols).map(move |c| (r, c)))
    }
    fn iter_mut(&mut self) -> impl Iterator<Item = ((usize, usize), &mut RGB8)> {
        let (rows, cols) = self.size();
        unsafe {
            Self::iter_coords_helper(rows, cols).map(|(r, c)| {
                let ptr = self.get_mut(r, c).unwrap() as *mut RGB8;
                ((r, c), ptr.as_mut().unwrap())
            })
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn is_true() {
        assert!(true)
    }
}
