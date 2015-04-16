use std::vec::Vec;
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use std::str::{from_utf8, Utf8Error};
use std::num::ToPrimitive;
use std::io::{self, Read};

use bencode::{self, Bencode, FromBencode};
use bencode::Bencode::{ByteString, Number, List, Dict};
use url::{self, Url, Host};

use hash::{Sha1Hash, InvalidHashLength};

use self::TorrentDirTreeNode::{FileNode, DirNode};

use self::TorrentDirTreeNode::{FileNode, DirNode};

/// A torrent.
#[derive(Debug)]
pub struct Torrent {
  /// A list of tracker URLs, divided into tiers as per bittorrent 
  /// [BEP 12](http://www.bittorrent.org/beps/bep_0012.html).
  pub trackers: Vec<Vec<Url>>,
  /// A list of peers that may be seeding this torrent. Peers are in the form
  /// `(host, port)`. For example `[(1.2.3.4, 1000), (foo.org, 1001)]`.
  pub nodes: Vec<(Host, u16)>,
  /// A list of locations where the file(s) can be downloaded from over http.
  /// This is for HTTP seeding (Hoffman-style). See
  /// [BEP 17](http://www.bittorrent.org/beps/bep_0017.html) for more info.
  pub httpseeds: Vec<Url>,
  /// A url where the file(s) can be downloaded. This is for HTTP/FTP seeding
  /// (GetRight-style). See
  /// [BEP 19](http://www.bittorrent.org/beps/bep_0019.html) for more info.
  pub urllist: Option<Url>,
  /// Is this a private torrent?
  pub private: bool,
  /// The length of a piece in bytes.
  pub piece_length: u64,
  /// The hashes of the individual torrent pieces.
  pub pieces: Vec<Sha1Hash>,
  /// The root of the Merkle hash of the torrent. See
  /// [BEP 30](http://www.bittorrent.org/beps/bep_0030.html) for more info.
  pub merkle_root: Option<Sha1Hash>,
  /// The root file or directory name of the torrent.
  pub filename: String,
  /// The directory structure of the torrent.
  pub contents: TorrentDirTreeNode,
}

/// A node in a directory structure.
#[derive(Debug)]
pub enum TorrentDirTreeNode {
  /// A file node in a directory structure. `FileNode(n)` represents a file of
  /// size `n`.
  FileNode(u64),
  /// A directory node in a directory structure. A map of filenames to
  /// directories and/or files.
  DirNode(HashMap<String, TorrentDirTreeNode>),
}

#[derive(Debug)]
pub enum TorrentFromBencodeError {
  NotADict,
  AnnounceListNotAList,
  AnnounceListTierNotAList,
  TrackerUrlNotAString,
  TrackerUrlParseError(url::ParseError),
  TrackerUrlInvalidUtf8(Utf8Error),
  AnnounceUrlNotAString,
  AnnounceUrlParseError(url::ParseError),
  AnnounceUrlInvalidUtf8(Utf8Error),
  NodeListNotAList,
  NodeNotAList,
  NodeHostNotAString,
  NodeHostParseError(url::ParseError),
  NodeHostInvalidUtf8(Utf8Error),
  NodePortNotANumber,
  NodePortOutOfRange,
  NodeInvalidList,
  UrlListNotAString,
  UrlListParseError(url::ParseError),
  UrlListInvalidUtf8(Utf8Error),
  HttpSeedsNotAList,
  HttpSeedNotAString,
  HttpSeedParseError(url::ParseError),
  HttpSeedInvalidUtf8(Utf8Error),
  InfoDictNotADict,
  RootHashNotAString,
  RootHashInvalidHashLength(usize),
  PrivateFlagNotANumber,
  NameNotAString,
  NameInvalidUtf8(Utf8Error),
  NameNotPresent,
  PieceLengthNotANumber,
  PieceLengthOutOfRange,
  PieceLengthNotPresent,
  InvalidPiecesLength(usize),
  PiecesNotAString,
  PiecesNotPresent,
  LengthNotANumber,
  LengthOutOfRange,
  FilesNotAList,
  NietherLengthOrFilesPresent,
  FileInfoNotADict,
  FileLengthNotANumber,
  FileLengthOutOfRange,
  FileLengthNotPresent,
  FilePathNotAList,
  FilePathNotPresent,
  DirNameNotAString,
  DirNameInvalidUtf8(Utf8Error),
  FileNameNotAString,
  FileNameInvalidUtf8(Utf8Error),
  EmptyFilePath,
  DuplicateFileName,
}

impl FromBencode for Torrent {
  type Err = TorrentFromBencodeError;

  fn from_bencode(bencode: &Bencode) -> Result<Torrent, TorrentFromBencodeError> {
    use self::TorrentFromBencodeError::*;

    let hm = try_case!(Dict, bencode, NotADict);

    let announce_list = match hm.get(&b"announce-list"[..]) {
      Some(a) => {
        let al = try_case!(List, a, AnnounceListNotAList);
        let mut tiers_vec: Vec<Vec<Url>> = Vec::new();
        for tier in al.iter() {
          let t = try_case!(List, tier, AnnounceListTierNotAList);
          let mut tier_vec: Vec<Url> = Vec::new();
          for tracker in t.iter() {
            let u = try_case!(ByteString, tracker, TrackerUrlNotAString);
            match from_utf8(&u[..]) {
              Ok(ss)  => match Url::parse(ss) {
                Ok(url) => tier_vec.push(url),
                Err(e)  => return Err(TrackerUrlParseError(e)),
              },
              Err(e)  => return Err(TrackerUrlInvalidUtf8(e)),
            }
          };
          tiers_vec.push(tier_vec);
        };
        Some(tiers_vec)
      },
      None    => None,
    };

    let announce = match hm.get(&b"announce"[..]) {
      Some(s) => match from_utf8(&try_case!(ByteString, s, AnnounceUrlNotAString)[..]) {
        Ok(ss)  => match Url::parse(ss) {
          Ok(url) => Some(url),
          Err(e)  => return Err(AnnounceUrlParseError(e)),
        },
        Err(e)  => return Err(AnnounceUrlInvalidUtf8(e)),
      },
      None    => None,
    };

    let trackers = match announce_list {
      Some(al)  => al,
      None      => match announce {
        Some(s) => {
          let mut t: Vec<Url> = Vec::new();
          let mut u: Vec<Vec<Url>> = Vec::new();
          t.push(s);
          u.push(t);
          u
        },
        None    => Vec::new(),
      },
    };

    let nodes: Vec<(Host, u16)> = match hm.get(&b"nodes"[..]) {
      Some(nl_be) => {
        let nl = try_case!(List, nl_be, NodeListNotAList);
        let mut nodes: Vec<(Host, u16)> = Vec::new();
        for n_be in nl.iter() {
          let n = try_case!(List, n_be, NodeNotAList);
          let mut niter = n.iter();
          match (niter.next(), niter.next(), niter.next()) {
            (Some(addr_be), Some(port_be), None) => {
              let addr = match from_utf8(&try_case!(ByteString, addr_be, NodeHostNotAString)[..]) {
                Ok(ss)  => match Host::parse(ss) {
                  Ok(h)   => h,
                  Err(e)  => return Err(NodeHostParseError(e)),
                },
                Err(e)  => return Err(NodeHostInvalidUtf8(e)),
              };
              let port = match try_case!(Number, port_be, NodePortNotANumber).to_u16() {
                Some(port)  => port,
                None        => return Err(NodePortOutOfRange),
              };
              nodes.push((addr, port));
            },
            _ => return Err(NodeInvalidList),
          }
        };
        nodes
      },
      None    => Vec::new(),
    };

    let urllist: Option<Url> = match hm.get(&b"url-list"[..]) {
      Some(ul_be) => match from_utf8(&try_case!(ByteString, ul_be, UrlListNotAString)) {
        Ok(ul)  => match Url::parse(ul) {
          Ok(ul)  => Some(ul),
          Err(e)  => return Err(UrlListParseError(e)),
        },
        Err(e)  => return Err(UrlListInvalidUtf8(e)),
      },
      None  => None,
    };

    let httpseeds: Vec<Url> = match hm.get(&b"httpseeds"[..]) {
      Some(hl_be) => {
        let hl = try_case!(List, hl_be, HttpSeedsNotAList);
        let mut httpseeds: Vec<Url> = Vec::new();
        for h_be in hl.iter() {
          let h = match from_utf8(&try_case!(ByteString, h_be, HttpSeedNotAString)[..]) {
            Ok(ss)  => match Url::parse(ss) {
              Ok(url) => url,
              Err(e)  => return Err(HttpSeedParseError(e)),
            },
            Err(e)  => return Err(HttpSeedInvalidUtf8(e)),
          };
          httpseeds.push(h);
        };
        httpseeds
      },
      None  => Vec::new(),
    };

    let info = match hm.get(&b"info"[..]) {
      Some(i) => try_case!(Dict, i, InfoDictNotADict),
      None    => hm,
    };

    let merkle_root = match info.get(&b"root hash"[..]) {
      Some(mr_be) => {
        let mr = try_case!(ByteString, mr_be, RootHashNotAString);
        match Sha1Hash::from_buffer(mr.as_slice()) {
          Ok(hash)  => Some(hash),
          Err(e)    => match e {
            InvalidHashLength(l) => return Err(RootHashInvalidHashLength(l)),
          },
        }
      },
      None  => None,
    };

    let private = match info.get(&b"private"[..]) {
      Some(p_be)  => {
        let p = try_case!(Number, p_be, PrivateFlagNotANumber);
        *p != 0
      },
      None => false,
    };

    let name = match info.get(&b"name"[..]) {
      Some(name_be) => match from_utf8(&try_case!(ByteString, name_be, NameNotAString)[..]) {
        Ok(ss)  => String::from_str(ss),
        Err(e)  => return Err(NameInvalidUtf8(e)),
      },
      None          => return Err(NameNotPresent)
    };

    let piece_length = match info.get(&b"piece length"[..]) {
      Some(pl_be) => match try_case!(Number, pl_be, PieceLengthNotANumber).to_u64() {
        Some(pl)  => pl,
        None      => return Err(PieceLengthOutOfRange),
      },
      None        => return Err(PieceLengthNotPresent),
    };

    let pieces = match info.get(&b"pieces"[..]) {
      Some(p_be) => try_case!(ByteString, p_be, PiecesNotAString),
      None       => return Err(PiecesNotPresent),
    };

    let mut pieces_vec: Vec<Sha1Hash> = Vec::new();
    let mut remaining = &pieces[..];

    loop {
      if remaining.len() < 20 {
        return Err(InvalidPiecesLength(pieces.len()));
      }
      pieces_vec.push(Sha1Hash::from_buffer(&remaining[.. 20]).unwrap());
      remaining = &remaining[20 ..];

      if remaining.len() == 0 {
        break;
      }
    }
    
    match info.get(&b"length"[..]) {
      Some(l) => {
        let length = match try_case!(Number, l, LengthNotANumber).to_u64() {
          Some(l) => l,
          None    => return Err(LengthOutOfRange),
        };
        Ok(Torrent {
          trackers:     trackers,
          nodes:        nodes,
          httpseeds:    httpseeds,
          urllist:      urllist,
          private:      private,
          piece_length: piece_length,
          pieces:       pieces_vec,
          merkle_root:  merkle_root,
          filename:     name,
          contents:     FileNode(length),
        })
      },
      None    => {
        let files = match info.get(&b"files"[..]) {
          Some(files_be)  => try_case!(List, files_be, FilesNotAList),
          None            => return Err(NietherLengthOrFilesPresent),
        };
        let mut filetree: HashMap<String, TorrentDirTreeNode> = HashMap::new();
        for fileinfo_be in files.iter() {
          let fileinfo = try_case!(Dict, fileinfo_be, FileInfoNotADict);
          let length = match fileinfo.get(&b"length"[..]) {
            Some(l_be)  => match try_case!(Number, l_be, FileLengthNotANumber).to_u64() {
              Some(l) => l,
              None    => return Err(FileLengthOutOfRange),
            },
            None  => return Err(FileLengthNotPresent),
          };
          let path = match fileinfo.get(&b"path"[..]) {
            Some(p_be)  => try_case!(List, p_be, FilePathNotAList).as_slice(),
            None        => return Err(FilePathNotPresent),
          };
          match path {
            [dirlist.., ref fname_be]  => {
              fn getdir<'a>(dir: &'a mut HashMap<String, TorrentDirTreeNode>, dl: &[Bencode]) -> Result<&'a mut HashMap<String, TorrentDirTreeNode>, TorrentFromBencodeError> {
                match dl {
                  [ref nextdir_be, rest..]  => {
                    let nextdir = match from_utf8(&try_case!(ByteString, nextdir_be, DirNameNotAString)[..]) {
                      Ok(ss)  => String::from_str(ss),
                      Err(e)  => return Err(DirNameInvalidUtf8(e)),
                    };
                    match dir.entry(nextdir).or_insert_with(|| DirNode(HashMap::new())) {
                      &mut FileNode(_)              => return Err(DuplicateFileName),
                      &mut DirNode(ref mut entries) => getdir(entries, rest),
                    }
                  },
                  []  => Ok(dir),
                }
              };

              let dir = try!(getdir(&mut filetree, dirlist));
              let fname = match from_utf8(&try_case!(ByteString, fname_be, FileNameNotAString)[..]) {
                Ok(ss)  => String::from_str(ss),
                Err(e)  => return Err(FileNameInvalidUtf8(e))
              };
              match dir.insert(fname, FileNode(length)) {
                None    => (),
                Some(_) => return Err(DuplicateFileName),
              };
            },
            []  => return Err(EmptyFilePath),
          };
        };
        Ok(Torrent {
          trackers:     trackers,
          nodes:        nodes,
          httpseeds:    httpseeds,
          urllist:      urllist,
          private:      private,
          piece_length: piece_length,
          pieces:       pieces_vec,
          merkle_root:  merkle_root,
          filename:     name,
          contents:     DirNode(filetree),
        })
      }
    }
  }
}

#[derive(Debug)]
pub enum LoadFileError {
  Io(io::Error),
  InvalidBencode(bencode::streaming::Error),
  FromBencode(TorrentFromBencodeError),
}

#[derive(Debug)]
pub enum FromBufferError {
  InvalidBencode(bencode::streaming::Error),
  FromBencode(TorrentFromBencodeError),
}

impl Torrent {
  /// Load a torrent from a file. Returns None if the torrent file is malformed,
  /// or then was an error reading the file.
  pub fn load_file(path: &Path) -> Result<Torrent, LoadFileError> {
    let mut f = match File::open(path) {
      Ok(f)   => f,
      Err(e)  => return Err(LoadFileError::Io(e)),
    };
    let mut data: Vec<u8> = Vec::new();
    match f.read_to_end(&mut data) {
      Ok(_)   => (),
      Err(e)  => return Err(LoadFileError::Io(e)),
    };
    match Torrent::from_buffer(data.as_slice()) {
      Ok(v)  => Ok(v),
      Err(e) => match e {
        FromBufferError::InvalidBencode(e) => return Err(LoadFileError::InvalidBencode(e)),
        FromBufferError::FromBencode(e)    => return Err(LoadFileError::FromBencode(e)),
      },
    }
  }

  /// Load a torrent from a slice of bencoded data.
  pub fn from_buffer(s: &[u8]) -> Result<Torrent, FromBufferError> {
    let ben = match bencode::from_buffer(s) {
      Ok(d)   => d,
      Err(e)  => return Err(FromBufferError::InvalidBencode(e)),
    };
    FromBencode::from_bencode(&ben).map_err(FromBufferError::FromBencode)
  }
}

