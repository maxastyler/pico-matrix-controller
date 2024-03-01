#![no_std]

#[derive(Default, Debug, Copy, Clone)]
#[repr(C, align(4))]
pub struct RGB8 {
    pub padding: u8,
    pub b: u8,
    pub r: u8,
    pub g: u8,
}

pub trait FrameTime {
    fn frame_time(&self) -> u64;
}

pub trait Updateable {
    type Message;

    fn update<D: MatrixDisplay>(&mut self, message: Option<Self::Message>, display: &mut D);
}

pub struct MatrixState<ImageState> {
    im: ImageState,
    brightness: f32,
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

#[macro_export]
macro_rules! create_matrix_state {
    ($name: ident; $message_type_name: ident; $($i: ident),*) => {
	use $crate::{FrameTime, Updateable, MatrixDisplay};
	use serde::{Serialize, Deserialize};
	pub enum $name {
	    $($i($i)),*
	}

	#[derive(Serialize, Deserialize)]
	pub enum $message_type_name {
	    $($i(<$i as Updateable>::Message)),*
	}

	impl Updateable for $name {
	    type Message = $message_type_name;

	    fn update<D: MatrixDisplay>(&mut self, message: Option<Self::Message>, display: &mut D) {
		match self {
		    $($name::$i(inner) => {
			let message = match message {
			    Some(Self::Message::$i(inner)) => Some(inner),
			    _ => None
			};
			inner.update(message, display);
		    })*
		}
	    }
	}

	impl FrameTime for $name {
	    fn frame_time(&self) -> u64 {
		match self {
		    $($name::$i(inner) => {inner.frame_time()})*
		}
	    }
	}
    }
}

#[cfg(test)]
mod test {
    use crate::{FrameTime, Updateable};
    #[test]
    fn is_true() {
        assert!(true)
    }

    #[test]
    fn test_create_matrix_state() {
        struct Hi;
        impl Updateable for Hi {
            type Message = u32;

            fn update<D: crate::MatrixDisplay>(
                &mut self,
                message: Option<Self::Message>,
                display: &mut D,
            ) {
                todo!()
            }
        }
        impl FrameTime for Hi {
            fn frame_time(&self) -> u64 {
                3
            }
        }
        struct There;
        impl Updateable for There {
            type Message = f64;

            fn update<D: crate::MatrixDisplay>(
                &mut self,
                message: Option<Self::Message>,
                display: &mut D,
            ) {
                todo!()
            }
        }
        impl FrameTime for There {
            fn frame_time(&self) -> u64 {
                2
            }
        }
        create_matrix_state!(Hello; HelloMessage; Hi, There);
        assert_eq!(Hello::Hi(Hi).frame_time(), 3);
        HelloMessage::Hi(32);
    }
}
