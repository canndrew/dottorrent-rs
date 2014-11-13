use std::fmt::{mod, Show, Formatter};

pub struct Sha1Hash {
  pub hash: [u8, ..20],
}

impl Sha1Hash {
  /// Create a `Sha1Hash` from a slice. Returns None if the slice is not 160
  /// bits (20 bytes) long.
  pub fn from_buffer(s: &[u8]) -> Option<Sha1Hash> {
    match s.len() == 20 {
      true  => {
        let mut hash: [u8, ..20] = unsafe { uninitialized() };
        for (d, s) in hash.iter().zip(s.iter()) {
          *d = *s;
        };
        Some(Sha1Hash {
          hash: hash,
        })
      },
      false => None,
    }
  }
}

impl Show for Sha1Hash {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    for b in self.hash.iter() {
      try!(write!(f, "{:02x}", *b));
    }
    Ok(())
  }
}

