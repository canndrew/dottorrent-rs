extern crate dottorrent;

use std::os;
use dottorrent::Torrent;

fn main() {
  let args = os::args();
  for file in args.slice_from(1).iter() {
    println!("## {}", file);
    println!("");
    let p = Path::new(file.as_slice());
    match Torrent::load_file(&p) {
      Some(t) => {
        println!("trackers: {}", t.trackers);
        println!("filename: {}", t.filename);
        println!("everything: {}", t);
      },
      None    => println!("Missing file or malformed torrent"),
    };
    println!("");
    println!("");
  }
}

