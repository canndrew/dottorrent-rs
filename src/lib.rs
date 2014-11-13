//! A small rust library for parsing and inspecting .torrent files.
//!
//! # Example
//!
//! ```rust
//! use std::os;
//! use dottorrent::Torrent;
//! 
//! fn main() {
//!   let args = os::args();
//!   for file in args.slice_from(1).iter() {
//!     println!("{}:", file);
//!     let p = Path::new(file.as_slice());
//!     match Torrent::load_file(&p) {
//!       Some(t) => {
//!         println!("Filename: {}", t.filename);
//!       },
//!       None    => println!("Malformed torrent"),
//!     };
//!     println!("");
//!   }
//! }
//! ```

#![feature(macro_rules)]
#![feature(slicing_syntax)]
#![feature(advanced_slice_patterns)]

extern crate bencode;

pub use torrent::{Torrent, TorrentDirTreeNode};
pub use hash::Sha1Hash;

macro_rules! try_opt(
  ($ex:expr)  => (match $ex {
    Some(x) => x,
    None    => return None,
  })
)

macro_rules! try_case(
  ($t:ident, $ex:expr) => (match $ex {
    &$t(ref x) => x,
    _          => return None,
  })
)

mod hash;
mod torrent;

