extern crate uuid;
extern crate rand;
extern crate mio;
extern crate mio_extras;
extern crate serde;
extern crate serde_json;
extern crate serde_derive;

use std::collections::HashMap;

pub mod obj;
pub mod prog;
pub mod daemon;
pub mod pubsub;
pub mod worker;
pub mod net;

//pub use obj;
//pub use microcode;

#[cfg(test)]
mod tests;

pub fn main() {

}
