//! A small rust library for parsing and inspecting .torrent files.
//!
//! # Example
//!
//! ```rust
//! use std::env;
//! use std::path::Path;
//! use dottorrent::Torrent;
//! 
//! fn main() {
//!  let args: Vec<String> = env::args().collect();
//!  for file in args[1 ..].iter() {
//!    println!("## {}", file);
//!    println!("");
//!    let p = Path::new(&file[..]);
//!    match Torrent::load_file(&p) {
//!      Ok(t)  => {
//!        println!("trackers: {:?}", t.trackers);
//!        println!("filename: {}", t.filename);
//!        println!("everything: {:?}", t);
//!      },
//!      Err(e) => println!("Error: {:?}", e),
//!    };
//!    println!("");
//!    println!("");
//!  }
//!}
//! ```

#![feature(core)]
#![feature(collections)]
#![feature(convert)]
#![feature(slice_patterns)]
#![feature(advanced_slice_patterns)]

extern crate bencode;
extern crate url;

pub use torrent::{Torrent, TorrentDirTreeNode};
pub use hash::Sha1Hash;

macro_rules! try_opt (
  ($ex:expr)  => (match $ex {
    Some(x) => x,
    None    => return None,
  })
);

macro_rules! try_case (
  ($t:ident, $ex:expr, $err:ident) => (match $ex {
    &$t(ref x) => x,
    _          => return Err($err),
  })
);

mod hash;
mod torrent;

