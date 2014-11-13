use std::vec::Vec;
use std::io::File;
use std::path::Path;
use std::mem::uninitialized;
use std::collections::TreeMap;

use bencode::{mod, Bencode, FromBencode};
use bencode::{ByteString, Number, List, Dict};

use hash::Sha1Hash;

#[deriving(Show)]
pub struct Torrent {
  pub trackers: Vec<Vec<String>>,
  pub piece_length: uint,
  pub pieces: Vec<Sha1Hash>,
  pub filename: String,
  pub contents: TorrentDirTreeNode,
}

/// A node in a directory structure.
#[deriving(Show)]
pub enum TorrentDirTreeNode {
  /// A file node in a directory structure. `FileNode(n)` represents a file of
  /// size `n`.
  FileNode(uint),
  /// A directory node in a directory structure. A map of filenames to
  /// directories and/or files.
  DirNode(TreeMap<String, TorrentDirTreeNode>),
}

impl FromBencode for Torrent {
  fn from_bencode(bencode: &Bencode) -> Option<Torrent> {
    /* TODO: clean up all this util::ByteString::from_str stuff when TreeMap
     * gets a get_equiv method.
     *
     * There's quite a bit of unnecesary string copying that could be avoided
     * with better TreeMap and bencode APIs.
     */
    let hm = try_case!(Dict, bencode);

    let announce_list = match hm.get(&bencode::util::ByteString::from_str("announce-list")) {
      Some(a) => {
        let al = try_case!(List, a);
        let mut tiers_vec: Vec<Vec<String>> = Vec::new();
        for tier in al.iter() {
          let t = try_case!(List, tier);
          let mut tier_vec: Vec<String> = Vec::new();
          for tracker in t.iter() {
            let u = try_case!(ByteString, tracker);
            match String::from_utf8(u.clone()) {
              Ok(ss)  => tier_vec.push(ss),
              Err(_)  => return None,
            };
          };
          tiers_vec.push(tier_vec);
        };
        Some(tiers_vec)
      },
      None    => None,
    };

    let announce = match hm.get(&bencode::util::ByteString::from_str("announce")) {
      Some(a) => match String::from_utf8(try_case!(ByteString, a).clone()) {
        Ok(ss)  => Some(ss),
        Err(_)  => return None,
      },
      None    => None,
    };

    let trackers = match announce_list {
      Some(al)  => al,
      None      => match announce {
        Some(s) => {
          let mut t: Vec<String> = Vec::new();
          let mut u: Vec<Vec<String>> = Vec::new();
          t.push(s);
          u.push(t);
          u
        },
        None    => Vec::new(),
      },
    };

    let info = match hm.get(&bencode::util::ByteString::from_str("info")) {
      Some(i) => try_case!(Dict, i),
      None    => hm,
    };

    let name = match String::from_utf8(try_case!(ByteString,
          try_opt!(info.get(&bencode::util::ByteString::from_str("name")))).clone()) {
      Ok(ss)  => ss,
      Err(_)  => return None,
    };
    let piece_length = try_opt!(try_case!(Number, try_opt!(info.get(&bencode::util::ByteString::from_str("piece length")))).to_uint());
    let pieces = try_case!(ByteString, try_opt!(info.get(&bencode::util::ByteString::from_str("pieces"))));

    let mut pieces_vec: Vec<Sha1Hash> = Vec::new();
    let mut remaining = pieces[];

    loop {
      if remaining.len() < 20 {
        return None;
      }
      pieces_vec.push(Sha1Hash::from_buffer(remaining[.. 20]).unwrap());
      remaining = remaining[20 ..];

      if remaining.len() == 0 {
        break;
      }
    }
    
    match info.get(&bencode::util::ByteString::from_str("length")) {
      Some(l) => {
        let length = try_opt!(try_case!(Number, l).to_uint());
        Some(Torrent {
          trackers:     trackers,
          piece_length: piece_length,
          pieces:       pieces_vec,
          filename:     name,
          contents:     FileNode(length),
        })
      },
      None    => {
        let files = try_case!(List, try_opt!(info.get(&bencode::util::ByteString::from_str("files"))));
        let mut filetree: TreeMap<String, TorrentDirTreeNode> = TreeMap::new();
        for fileinfo_be in files.iter() {
          let fileinfo = try_case!(Dict, fileinfo_be);
          let length = try_opt!(try_case!(Number, try_opt!(fileinfo.get(&bencode::util::ByteString::from_str("length")))).to_uint());
          let path = try_case!(List, try_opt!(fileinfo.get(&bencode::util::ByteString::from_str("path")))).as_slice();
          match path {
            [dirlist.., ref fname_be]  => {
              fn getdir<'a>(dir: &'a mut TreeMap<String, TorrentDirTreeNode>, dl: &[Bencode]) -> Option<&'a mut TreeMap<String, TorrentDirTreeNode>> {
                match dl {
                  [ref nextdir_be, rest..]  => {
                    let nextdir = match String::from_utf8(try_case!(ByteString, nextdir_be).clone()) {
                      Ok(ss)  => ss,
                      Err(_)  => return None,
                    };
                    /* TODO: this bit is particularly ugly.
                     * collection reform could clean this up. */
                    match dir.contains_key(&nextdir) {
                      true  => match dir.get_mut(&nextdir) {
                        Some(node)  => match node {
                          &FileNode(_)              => None,
                          &DirNode(ref mut entries) => getdir(entries, rest),
                        },
                        None  => panic!(),
                      },
                      false => {
                        dir.insert(nextdir.clone(), DirNode(TreeMap::new()));
                        match dir.get_mut(&nextdir).unwrap() {
                          &DirNode(ref mut tm) => getdir(tm, rest),
                          _                    => panic!(),
                        }
                      }
                    }
                  },
                  []  => Some(dir),
                }
              };

              let dir = try_opt!(getdir(&mut filetree, dirlist));
              let fname = match String::from_utf8(try_case!(ByteString, fname_be).clone()) {
                Ok(ss)  => ss,
                Err(_)  => return None,
              };
              dir.insert(fname, FileNode(length));
            },
            []  => return None,
          };
        };
        Some(Torrent {
          trackers:     trackers,
          piece_length: piece_length,
          pieces:       pieces_vec,
          filename:     name,
          contents:     DirNode(filetree),
        })
      }
    }
  }
}

impl Torrent {
  /// Load a torrent from a file. Returns None if the torrent file is malformed,
  /// or then was an error reading the file.
  pub fn load_file(path: &Path) -> Option<Torrent> {
    let mut f = File::open(path);
    let data = match f.read_to_end() {
      Ok(d)   => d,
      Err(_)  => return None,
    };
    Torrent::from_buffer(data.as_slice())
  }

  /// Load a torrent from a slice of bencoded data.
  pub fn from_buffer(s: &[u8]) -> Option<Torrent> {
    let ben = match bencode::from_buffer(s) {
      Ok(d)   => d,
      Err(_)  => return None,
    };
    FromBencode::from_bencode(&ben)
  }
}

