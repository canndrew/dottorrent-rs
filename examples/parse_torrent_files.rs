extern crate dottorrent;

use std::os;
use dottorrent::Torrent;

fn main() {
  let args = os::args();
  for file in args.slice_from(1).iter() {
    println!("{}:", file);
    let p = Path::new(file.as_slice());
    match Torrent::load_file(&p) {
      Some(m) => println!("{}", m),
      None    => println!("Malformed torrent"),
    };
    println!("");
  }
}

