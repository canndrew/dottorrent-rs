extern crate dottorrent;

use std::env;
use std::path::Path;
use dottorrent::Torrent;

fn main() {
  let args: Vec<String> = env::args().collect();
  for file in args[1 ..].iter() {
    println!("## {}", file);
    println!("");
    let p = Path::new(&file[..]);
    match Torrent::load_file(&p) {
      Ok(t)  => {
        println!("trackers: {:?}", t.trackers);
        println!("filename: {}", t.filename);
        println!("everything: {:?}", t);
      },
      Err(e) => println!("Error: {:?}", e),
    };
    println!("");
    println!("");
  }
}

