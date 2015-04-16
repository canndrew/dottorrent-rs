use std::fmt;
use std::mem::uninitialized;

pub struct Sha1Hash {
  pub hash: [u8; 20],
}

#[derive(Debug)]
pub struct InvalidHashLength(pub usize);

impl Sha1Hash {
  /// Create a `Sha1Hash` from a slice. Returns None if the slice is not 160
  /// bits (20 bytes) long.
  pub fn from_buffer(s: &[u8]) -> Result<Sha1Hash, InvalidHashLength> {
    match s.len() == 20 {
      true  => {
        let mut hash: [u8; 20] = unsafe { uninitialized() };
        for (d, s) in hash.iter_mut().zip(s.iter()) {
          *d = *s;
        };
        Ok(Sha1Hash {
          hash: hash,
        })
      },
      false => Err(InvalidHashLength(s.len())),
    }
  }
}

impl fmt::Debug for Sha1Hash {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for b in self.hash.iter() {
      try!(write!(f, "{:02x}", *b));
    }
    Ok(())
  }
}

