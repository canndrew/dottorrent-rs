use std::fmt::{mod, Show, Formatter};

pub struct Sha1Hash {
  pub hash: [u8, ..20],
}

impl Show for Sha1Hash {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    for b in self.hash.iter() {
      try!(write!(f, "{:02x}", *b));
    }
    Ok(())
  }
}

